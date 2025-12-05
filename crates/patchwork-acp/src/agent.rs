//! Agent infrastructure for LLM communication.
//!
//! The Agent manages think block execution by creating LLM sessions with the
//! successor agent (like claude-code-acp). Interpreter threads send ThinkRequests
//! through channels, and the Agent spawns async tasks to handle each request.
//!
//! This design is inspired by Niko Matsakis's threadbare prototype.
//!
//! ## Architecture
//!
//! The interpreter sends ThinkRequests via tokio's UnboundedSender (non-blocking).
//! The agent runs in async land and receives via UnboundedReceiver.
//!
//! 1. Interpreter sends `ThinkRequest` via tokio `UnboundedSender` (non-blocking)
//! 2. Agent receives via `UnboundedReceiver` in with_client main loop
//! 3. Agent creates LLM sessions and accumulates responses
//! 4. Results are sent back via `ThinkResponse` on `std::sync::mpsc`

use std::sync::Arc;

use sacp::schema::{
    ContentBlock, NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse,
    SessionNotification, SessionUpdate, StopReason,
};
use sacp::JrConnectionCx;
use sacp_proxy::{JrCxExt, McpServer, McpServiceRegistry};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{channel, unbounded_channel, Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::{oneshot, Mutex};

use patchwork_eval::{AgentHandle, ThinkRequest, ThinkResponse, Value};

/// Result of a think block execution.
pub type ThinkResult = Result<Value, String>;

/// Messages internal to the agent for routing between sessions.
pub enum RedirectMessage {
    /// An incoming message from SACP to route to the active thinker.
    IncomingMessage(PerSessionMessage),
    /// Push a new thinker onto the stack (for nested think blocks).
    PushThinker(Sender<PerSessionMessage>),
    /// Pop the top thinker from the stack.
    PopThinker,
}

/// Messages that get routed to individual think sessions.
pub enum PerSessionMessage {
    /// Notification from the successor agent (streaming content, etc.).
    SessionNotification(SessionNotification),
    /// The LLM invoked the "do" tool.
    DoInvocation(DoArg, oneshot::Sender<String>),
    /// The prompt completed.
    PromptResponse(PromptResponse),
}

/// Argument for the MCP "do" tool.
#[derive(JsonSchema, Deserialize, Serialize)]
pub struct DoArg {
    /// Index of the child to execute.
    pub number: usize,
}

/// Result from the MCP "do" tool.
#[derive(JsonSchema, Deserialize, Serialize)]
pub struct DoResult {
    /// The text result of executing the child.
    pub text: String,
}

/// Shared state for the agent, accessible from async tasks.
pub struct AgentState {
    /// Channel for sending redirect messages.
    pub redirect_tx: UnboundedSender<RedirectMessage>,
    /// MCP registry with the "do" tool.
    pub mcp_registry: McpServiceRegistry,
}

/// Create an agent that bridges the interpreter to async LLM sessions.
///
/// Returns:
/// - `AgentHandle` - Pass this to the interpreter for think blocks
/// - `UnboundedSender<RedirectMessage>` - For routing session notifications
/// - `UnboundedReceiver<ThinkRequest>` - The receiver for the bridge loop
///
/// The caller must run the bridge loop in with_client's main_fn.
pub fn create_agent(
    cx: JrConnectionCx,
    mcp_registry: McpServiceRegistry,
) -> (
    AgentHandle,
    UnboundedSender<RedirectMessage>,
    UnboundedReceiver<ThinkRequest>,
    Arc<AgentState>,
) {
    // Create the tokio channel for think requests (non-blocking send from sync code)
    let (request_tx, request_rx) = unbounded_channel::<ThinkRequest>();

    // Create the redirect channel
    let (redirect_tx, redirect_rx) = unbounded_channel();

    // Store shared state
    let state = Arc::new(AgentState {
        redirect_tx: redirect_tx.clone(),
        mcp_registry,
    });

    // Spawn redirect actor via cx.spawn() - it doesn't need to call block_task()
    if let Err(e) = cx.spawn(async move {
        redirect_actor(redirect_rx).await;
        Ok(())
    }) {
        tracing::error!("Failed to spawn redirect actor: {}", e);
    }

    // Create the AgentHandle for the interpreter
    let handle = AgentHandle::new(request_tx);

    (handle, redirect_tx, request_rx, state)
}


/// Process a single think request from the interpreter.
pub async fn process_think_request(cx: JrConnectionCx, request: ThinkRequest, state: Arc<AgentState>) -> Result<(), sacp::Error> {
    let ThinkRequest {
        prompt,
        bindings: _,
        expect,
        response_tx,
    } = request;

    // Execute the think block and send responses
    let result = think_message(cx, prompt, expect, state).await;

    // Send the Complete response
    let _ = response_tx.send(ThinkResponse::Complete { result });

    Ok(())
}

/// The redirect actor maintains a stack of active thinkers and routes messages.
///
/// When nested think blocks occur, each one pushes onto the stack. Messages
/// are always routed to the top of the stack (the innermost active think).
async fn redirect_actor(mut rx: UnboundedReceiver<RedirectMessage>) {
    let mut stack: Vec<Sender<PerSessionMessage>> = vec![];

    while let Some(message) = rx.recv().await {
        match message {
            RedirectMessage::IncomingMessage(msg) => {
                if let Some(sender) = stack.last() {
                    if sender.send(msg).await.is_err() {
                        tracing::warn!("Failed to send message to thinker");
                    }
                }
            }
            RedirectMessage::PushThinker(sender) => {
                stack.push(sender);
            }
            RedirectMessage::PopThinker => {
                stack.pop();
            }
        }
    }
}

/// Handle a single think block by creating an LLM session with the successor.
async fn think_message(
    cx: JrConnectionCx,
    prompt: String,
    expect: String,
    state: Arc<AgentState>,
) -> ThinkResult {
    // Build the augmented prompt with type hints
    let augmented_prompt = augment_prompt_with_type_hint(&prompt, &expect);

    // Create session request with our MCP server
    let mut new_session = NewSessionRequest {
        cwd: std::env::current_dir().unwrap_or_default(),
        mcp_servers: vec![],
        meta: None,
    };
    state
        .mcp_registry
        .add_registered_mcp_servers_to(&mut new_session);

    // Start a new session with the successor agent (e.g., claude-code-acp)
    // This uses block_task().await directly because think_message is spawned via cx.spawn(),
    // so it's part of the connection's event loop and can receive responses.
    tracing::info!("THINK_MSG: about to send session/new to successor");
    let response_future = cx.send_request_to_successor(new_session);
    tracing::info!("THINK_MSG: request future created, now calling block_task()");
    let session_result = response_future.block_task().await;
    tracing::info!("THINK_MSG: block_task() RETURNED! is_ok={:?}", session_result.is_ok());
    let NewSessionResponse { session_id, .. } = match session_result {
        Ok(resp) => {
            tracing::info!("think_message: got session_id={}", resp.session_id);
            resp
        }
        Err(e) => return Err(format!("Failed to create session: {}", e)),
    };

    // Create channel for receiving messages for this think session
    let (think_tx, mut think_rx) = channel(128);
    if state
        .redirect_tx
        .send(RedirectMessage::PushThinker(think_tx))
        .is_err()
    {
        return Err("Redirect actor not running".to_string());
    }
    tracing::info!("think_message: pushed thinker onto stack");

    // Send the prompt request to the successor
    tracing::info!("think_message: sending prompt to successor for session {}", session_id);
    let prompt_result = cx
        .send_request_to_successor(PromptRequest {
            session_id: session_id.clone(),
            prompt: vec![augmented_prompt.into()],
            meta: None,
        })
        .await_when_result_received({
            let redirect_tx = state.redirect_tx.clone();
            async move |response| {
                redirect_tx
                    .send(RedirectMessage::IncomingMessage(
                        PerSessionMessage::PromptResponse(response?),
                    ))
                    .map_err(sacp::util::internal_error)
            }
        });

    if let Err(e) = prompt_result {
        let _ = state.redirect_tx.send(RedirectMessage::PopThinker);
        return Err(format!("Failed to send prompt: {}", e));
    }

    // Accumulate the response
    let mut result_text = String::new();

    while let Some(message) = think_rx.recv().await {
        match message {
            PerSessionMessage::SessionNotification(notification) => {
                // Accumulate streaming text from the LLM
                if let SessionUpdate::AgentMessageChunk(chunk) = notification.update {
                    if let ContentBlock::Text(text) = chunk.content {
                        result_text.push_str(&text.text);
                    }
                }
            }
            PerSessionMessage::DoInvocation(DoArg { number }, do_tx) => {
                // TODO: In Phase 5, this will trigger recursive evaluation
                // For now, return an error
                let _ = do_tx.send(format!("do({}) not yet implemented", number));
            }
            PerSessionMessage::PromptResponse(response) => {
                match response.stop_reason {
                    StopReason::EndTurn => break,
                    reason => {
                        tracing::warn!("Unexpected stop reason: {:?}", reason);
                        break;
                    }
                }
            }
        }
    }

    // Pop ourselves from the redirect stack
    let _ = state.redirect_tx.send(RedirectMessage::PopThinker);

    // Extract the typed value from the response
    extract_response_value(&result_text, &expect)
}

/// Create the MCP server offering the "do" tool.
///
/// This should be registered with the proxy's MCP registry so that when
/// we create sessions with the successor, the "do" tool is available.
pub fn create_mcp_server(redirect_tx: UnboundedSender<RedirectMessage>) -> McpServer {
    let redirect_tx = Arc::new(Mutex::new(redirect_tx));
    McpServer::new()
        .instructions("Patchwork interpreter tools for recursive evaluation")
        .tool_fn(
            "do",
            "Execute a Patchwork code fragment by index. Call this when instructed to execute a specific numbered code block.",
            {
                let redirect_tx = redirect_tx.clone();
                async move |arg: DoArg, _cx| -> Result<DoResult, sacp::Error> {
                    let (result_tx, result_rx) = oneshot::channel();
                    let tx = redirect_tx.lock().await;
                    tx.send(RedirectMessage::IncomingMessage(
                        PerSessionMessage::DoInvocation(arg, result_tx),
                    ))
                    .map_err(sacp::util::internal_error)?;
                    drop(tx);
                    Ok(DoResult {
                        text: result_rx.await.map_err(sacp::util::internal_error)?,
                    })
                }
            },
            |f, a, b| Box::pin(f(a, b)),
        )
}

/// Augment the prompt with type hint instructions for response formatting.
fn augment_prompt_with_type_hint(prompt: &str, expect: &str) -> String {
    match expect {
        "string" => format!(
            "{}\n\nRespond with a string value. Format your response as:\n```text\nyour response here\n```",
            prompt
        ),
        "json" => format!(
            "{}\n\nRespond with a JSON value. Format your response as:\n```json\nyour JSON here\n```",
            prompt
        ),
        _ => prompt.to_string(),
    }
}

/// Extract a typed value from the LLM response using markdown code fence markers.
pub fn extract_response_value(response: &str, expect: &str) -> ThinkResult {
    // Try to find a code fence with the expected type
    let fence_marker = match expect {
        "string" => "```text",
        "json" => "```json",
        _ => "```",
    };

    if let Some(value) = extract_code_fence(response, fence_marker) {
        match expect {
            "string" => Ok(Value::String(value)),
            "json" => serde_json::from_str(&value)
                .map(json_to_value)
                .map_err(|e| format!("Failed to parse JSON: {}", e)),
            _ => Ok(Value::String(value)),
        }
    } else {
        // Fallback: use full response text
        Ok(Value::String(response.to_string()))
    }
}

/// Extract content between a code fence marker and the closing ```.
pub fn extract_code_fence(text: &str, marker: &str) -> Option<String> {
    let start = text.find(marker)?;
    let after_marker = start + marker.len();

    // Find the end of the opening line (skip past the marker line)
    let content_start = text[after_marker..]
        .find('\n')
        .map(|i| after_marker + i + 1)?;

    // Find the closing fence
    let remaining = &text[content_start..];
    let end = remaining.find("```")?;

    let content = &remaining[..end];
    Some(content.trim().to_string())
}

/// Convert serde_json::Value to patchwork_eval::Value.
fn json_to_value(json: serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(b),
        serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => Value::Array(arr.into_iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            Value::Object(obj.into_iter().map(|(k, v)| (k, json_to_value(v))).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_fence_text() {
        let response = r#"Here's the cleaned transcript:

```text
Hello world, this is the content.
Multiple lines work too.
```

That's all!"#;

        let result = extract_code_fence(response, "```text");
        assert_eq!(
            result,
            Some("Hello world, this is the content.\nMultiple lines work too.".to_string())
        );
    }

    #[test]
    fn test_extract_code_fence_json() {
        let response = r#"Here's the data:

```json
{"name": "test", "value": 42}
```
"#;

        let result = extract_code_fence(response, "```json");
        assert_eq!(result, Some(r#"{"name": "test", "value": 42}"#.to_string()));
    }

    #[test]
    fn test_extract_code_fence_no_fence() {
        let response = "Just plain text without any code fences";
        let result = extract_code_fence(response, "```text");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_response_value_string() {
        let response = "```text\nHello world\n```";
        let result = extract_response_value(response, "string");
        assert!(matches!(result, Ok(Value::String(s)) if s == "Hello world"));
    }

    #[test]
    fn test_extract_response_value_json() {
        let response = r#"```json
{"key": "value"}
```"#;
        let result = extract_response_value(response, "json");
        assert!(result.is_ok());
        if let Ok(Value::Object(obj)) = result {
            assert!(obj.contains_key("key"));
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_extract_response_value_fallback() {
        let response = "Just plain text";
        let result = extract_response_value(response, "string");
        assert!(matches!(result, Ok(Value::String(s)) if s == "Just plain text"));
    }

    #[test]
    fn test_augment_prompt_string() {
        let prompt = "Explain Rust";
        let result = augment_prompt_with_type_hint(prompt, "string");
        assert!(result.contains("Explain Rust"));
        assert!(result.contains("```text"));
    }

    #[test]
    fn test_augment_prompt_json() {
        let prompt = "Give me data";
        let result = augment_prompt_with_type_hint(prompt, "json");
        assert!(result.contains("Give me data"));
        assert!(result.contains("```json"));
    }
}
