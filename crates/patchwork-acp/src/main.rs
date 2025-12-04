//! Patchwork ACP proxy - interprets Patchwork code in the ACP message chain.
//!
//! This proxy sits between an editor (like Zed) and an agent (like Claude Code),
//! intercepting prompts that contain Patchwork code and executing them with
//! integrated LLM support via think blocks.

mod agent;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use sacp::schema::{ContentBlock, PromptRequest, PromptResponse, StopReason};
use sacp::{JrHandlerChain, JrRequestCx};
use sacp_proxy::{AcpProxyExt, JrCxExt, McpServiceRegistry};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing_subscriber::EnvFilter;

use patchwork_eval::{AgentHandle, Error as EvalError, Interpreter};

/// The Patchwork proxy state.
struct PatchworkProxy {
    /// Sessions with active evaluations (session IDs).
    active_sessions: HashSet<String>,
    /// Agent handle for think blocks.
    agent_handle: Option<AgentHandle>,
}

impl PatchworkProxy {
    fn new() -> Self {
        Self {
            active_sessions: HashSet::new(),
            agent_handle: None,
        }
    }

    fn has_active_evaluation(&self, session_id: &str) -> bool {
        self.active_sessions.contains(session_id)
    }

    fn start_evaluation(&mut self, session_id: &str) {
        self.active_sessions.insert(session_id.to_string());
    }

    fn end_evaluation(&mut self, session_id: &str) {
        self.active_sessions.remove(session_id);
    }

    fn set_agent_handle(&mut self, handle: AgentHandle) {
        self.agent_handle = Some(handle);
    }

    fn agent_handle(&self) -> Option<AgentHandle> {
        self.agent_handle.clone()
    }
}

/// Check if a message appears to be Patchwork code.
///
/// Patchwork code is identified by starting with `{` (after trimming whitespace).
fn is_patchwork_code(text: &str) -> bool {
    text.trim_start().starts_with('{')
}

/// Extract the text content from a prompt request.
fn extract_prompt_text(request: &PromptRequest) -> Option<String> {
    // The prompt request contains content blocks; look for Text blocks
    request.prompt.iter().find_map(|block| {
        if let ContentBlock::Text(text_content) = block {
            Some(text_content.text.clone())
        } else {
            None
        }
    })
}

/// Handle a prompt request, checking for Patchwork code.
async fn handle_prompt(
    proxy: Arc<Mutex<PatchworkProxy>>,
    request: PromptRequest,
    cx: JrRequestCx<PromptResponse>,
) -> Result<(), sacp::Error> {
    let session_id = request.session_id.to_string();

    // Extract the prompt text
    let Some(text) = extract_prompt_text(&request) else {
        // No text content, forward unchanged
        tracing::debug!("No text content in prompt, forwarding");
        cx.connection_cx()
            .send_request_to_successor(request)
            .forward_to_request_cx(cx)?;
        return Ok(());
    };

    // Check if it's Patchwork code
    if !is_patchwork_code(&text) {
        // Not code, forward unchanged
        tracing::debug!("Not Patchwork code, forwarding");
        cx.connection_cx()
            .send_request_to_successor(request)
            .forward_to_request_cx(cx)?;
        return Ok(());
    }

    tracing::info!("Detected Patchwork code, executing...");

    // Check for active evaluation and get agent handle
    let agent_handle = {
        let proxy = proxy.lock().unwrap();

        if proxy.has_active_evaluation(&session_id) {
            // Already evaluating, return error
            cx.respond_with_error(
                sacp::Error::invalid_request()
                    .with_data("Patchwork evaluation already in progress"),
            )?;
            return Ok(());
        }

        proxy.agent_handle()
    };

    // Mark session as active
    {
        let mut proxy = proxy.lock().unwrap();
        proxy.start_evaluation(&session_id);
    }

    // Create interpreter with agent handle
    let mut interp = match agent_handle {
        Some(handle) => Interpreter::with_agent(handle),
        None => Interpreter::new(),
    };

    // Evaluate on a blocking thread since interpreter may block on channels
    let eval_result = {
        let text = text.clone();
        tokio::task::spawn_blocking(move || interp.eval(&text))
            .await
            .map_err(|e| sacp::Error::internal_error().with_data(format!("Task error: {}", e)))?
    };

    // End the evaluation regardless of result
    {
        let mut proxy = proxy.lock().unwrap();
        proxy.end_evaluation(&session_id);
    }

    match eval_result {
        Ok(value) => {
            tracing::info!("Patchwork code completed: {:?}", value);

            // Normal completion
            let response = create_text_response(format!(
                "Patchwork execution completed: {}",
                value
            ));
            cx.respond(response)?;
        }
        Err(EvalError::Exception(value)) => {
            tracing::error!("Patchwork code threw exception: {:?}", value);
            cx.respond_with_error(
                sacp::Error::internal_error()
                    .with_data(format!("Patchwork exception: {}", value)),
            )?;
        }
        Err(e) => {
            tracing::error!("Patchwork parse/eval error: {}", e);
            cx.respond_with_error(
                sacp::Error::invalid_params().with_data(format!("Patchwork error: {}", e)),
            )?;
        }
    }

    Ok(())
}

/// Create a simple text response.
fn create_text_response(_text: String) -> PromptResponse {
    // TODO: In a full implementation, we'd need to send progress notifications
    // with the text content, since PromptResponse only contains stop_reason.
    // For now, we just log and return success.
    PromptResponse {
        stop_reason: StopReason::EndTurn,
        meta: None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting Patchwork ACP proxy");

    // Create shared proxy state
    let proxy = Arc::new(Mutex::new(PatchworkProxy::new()));

    // Create MCP registry for the "do" tool
    let mcp_registry = McpServiceRegistry::default();

    // Build the handler chain
    let proxy_clone = Arc::clone(&proxy);
    JrHandlerChain::new()
        .name("patchwork-acp")
        .on_receive_request(move |request: PromptRequest, cx: JrRequestCx<PromptResponse>| {
            let proxy = Arc::clone(&proxy_clone);
            async move {
                // Create agent on first request if not already created
                ensure_agent_created(&proxy, cx.connection_cx().clone());
                handle_prompt(proxy, request, cx).await
            }
        })
        .provide_mcp(mcp_registry)
        .proxy()
        .connect_to(sacp::ByteStreams::new(
            tokio::io::stdout().compat_write(),
            tokio::io::stdin().compat(),
        ))?
        .serve()
        .await?;

    Ok(())
}

/// Ensure the agent is created (lazily, on first request).
fn ensure_agent_created(proxy: &Arc<Mutex<PatchworkProxy>>, cx: sacp::JrConnectionCx) {
    let mut proxy = proxy.lock().unwrap();
    if proxy.agent_handle.is_none() {
        let mcp_registry = McpServiceRegistry::default();
        let (agent_handle, _redirect_tx) = agent::create_agent(cx, mcp_registry);
        proxy.set_agent_handle(agent_handle);
        tracing::info!("Agent created and connected to successor");
    }
}
