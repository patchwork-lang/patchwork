//! Agent infrastructure for LLM communication.
//!
//! The Agent manages think block execution by creating LLM sessions with the
//! successor agent (like claude-code-acp). Interpreter threads send ThinkRequests
//! through channels, and the Agent spawns async tasks to handle each request.
//!
//! This design is inspired by Niko Matsakis's threadbare prototype.

use std::collections::HashMap;
use std::sync::Arc;

use sacp::schema::{
    ContentBlock, NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse,
    SessionNotification, SessionUpdate, StopReason,
};
use sacp::JrConnectionCx;
use sacp_proxy::{McpServer, McpServiceRegistry};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{channel, unbounded_channel, Sender, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use patchwork_eval::Value;

/// A handle to the agent that can be cloned and sent to interpreter threads.
///
/// The Agent is lightweight - it just holds a sender to the agent task.
#[derive(Clone)]
pub struct Agent {
    tx: UnboundedSender<AgentRequest>,
}

/// Requests that interpreter threads can send to the agent.
pub enum AgentRequest {
    /// Execute a think block and return the LLM response.
    Think {
        /// The interpolated prompt text to send to the LLM.
        prompt: String,
        /// Variable bindings available in the think block scope.
        bindings: HashMap<String, Value>,
        /// Expected type hint for response extraction (e.g., "string", "json").
        expect: String,
        /// Channel to send the response value back to the interpreter.
        response_tx: oneshot::Sender<ThinkResult>,
    },
}

/// Result of a think block execution.
pub type ThinkResult = Result<Value, String>;

/// Response types that the agent sends back to interpreter threads during a think session.
#[allow(dead_code)]
pub enum ThinkResponse {
    /// The LLM invoked the "do" tool for recursive evaluation.
    Do {
        /// Index of the child AST node to evaluate.
        index: usize,
        /// Channel to send the evaluation result back to the agent.
        result_tx: oneshot::Sender<String>,
    },
    /// The think block completed with a final message.
    Complete { message: String },
}

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

impl Agent {
    /// Create a new agent that uses the given connection context.
    ///
    /// This spawns the agent's background tasks:
    /// - The redirect actor for routing messages to active think sessions
    /// - The request handler loop that processes ThinkRequests
    ///
    /// The agent uses the connection context to create new sessions with
    /// the successor agent for think blocks.
    pub fn new(cx: JrConnectionCx, mcp_registry: McpServiceRegistry) -> (Self, UnboundedSender<RedirectMessage>) {
        let (tx, rx) = unbounded_channel();
        let (redirect_tx, redirect_rx) = unbounded_channel();

        // Store shared state
        let state = Arc::new(AgentState {
            redirect_tx: redirect_tx.clone(),
            mcp_registry,
        });

        // Spawn the redirect actor
        tokio::spawn(Self::redirect_actor(redirect_rx));

        // Spawn the request handler loop
        tokio::spawn(Self::request_loop(cx, rx, state));

        (Self { tx }, redirect_tx)
    }

    /// Send a think request to the agent.
    ///
    /// Called from interpreter threads (via std::sync::mpsc, blocking).
    pub fn send_request(&self, request: AgentRequest) -> Result<(), String> {
        self.tx
            .send(request)
            .map_err(|e| format!("Failed to send request to agent: {}", e))
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

    /// Process incoming think requests from interpreter threads.
    async fn request_loop(
        cx: JrConnectionCx,
        mut rx: UnboundedReceiver<AgentRequest>,
        state: Arc<AgentState>,
    ) {
        while let Some(request) = rx.recv().await {
            match request {
                AgentRequest::Think {
                    prompt,
                    bindings: _,
                    expect,
                    response_tx,
                } => {
                    let cx = cx.clone();
                    let state = state.clone();
                    tokio::spawn(async move {
                        let result = Self::think_message(cx, prompt, expect, state).await;
                        let _ = response_tx.send(result);
                    });
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
        state.mcp_registry.add_registered_mcp_servers_to(&mut new_session);

        // Start a new session with the successor agent (e.g., claude-code-acp)
        let session_result = cx.send_request(new_session).block_task().await;

        let NewSessionResponse { session_id, .. } = match session_result {
            Ok(resp) => resp,
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

        // Send the prompt request to the successor
        let prompt_result = cx
            .send_request(PromptRequest {
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
