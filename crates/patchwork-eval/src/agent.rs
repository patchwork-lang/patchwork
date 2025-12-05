//! Agent communication interface for think blocks.
//!
//! This module defines the communication protocol between the interpreter and
//! the agent that handles LLM interactions. The interpreter sends think requests
//! and blocks waiting for responses.
//!
//! Inspired by Niko Matsakis's threadbare prototype.
//!
//! ## Channel Architecture
//!
//! - Think requests: Sent via `tokio::sync::mpsc::UnboundedSender` (non-blocking send from sync code)
//! - Think responses: Received via `std::sync::mpsc::Receiver` (blocking receive in sync interpreter)

use std::collections::HashMap;
use std::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

use crate::value::Value;

/// Response from the agent during a think session.
///
/// The interpreter blocks on an mpsc channel and receives these responses.
/// Multiple responses may arrive (Do for recursive eval, then Complete).
#[derive(Debug)]
pub enum ThinkResponse {
    /// The LLM invoked the "do" tool for recursive evaluation.
    ///
    /// The interpreter should evaluate `children[index]` and send the result
    /// back through `result_tx`.
    Do {
        /// Index of the child AST node to evaluate.
        index: usize,
        /// Channel to send the evaluation result back to the agent.
        result_tx: mpsc::SyncSender<String>,
    },

    /// The think block completed with a final value.
    Complete {
        /// The extracted value from the LLM response.
        result: Result<Value, String>,
    },
}

/// A request to execute a think block.
///
/// The interpreter sends this to the agent, then blocks waiting for
/// ThinkResponse messages on the provided channel.
pub struct ThinkRequest {
    /// The interpolated prompt text to send to the LLM.
    pub prompt: String,
    /// Variable bindings available in the think block scope.
    pub bindings: HashMap<String, Value>,
    /// Expected type hint for response extraction (e.g., "string", "json").
    pub expect: String,
    /// Channel to receive responses from the agent.
    ///
    /// The agent will send ThinkResponse messages:
    /// - Zero or more `Do` messages for recursive evaluation
    /// - Exactly one `Complete` message when finished
    pub response_tx: mpsc::Sender<ThinkResponse>,
}

/// A handle to the agent that can be used by the interpreter.
///
/// This is cloneable so it can be passed to different interpreter threads.
/// Uses tokio's UnboundedSender which allows non-blocking sends from sync code.
#[derive(Clone)]
pub struct AgentHandle {
    tx: UnboundedSender<ThinkRequest>,
}

impl AgentHandle {
    /// Create a new agent handle from a sender.
    pub fn new(tx: UnboundedSender<ThinkRequest>) -> Self {
        Self { tx }
    }

    /// Send a think request to the agent.
    ///
    /// Returns a receiver for ThinkResponse messages.
    /// The send is non-blocking (uses tokio unbounded channel),
    /// but the returned receiver is std::sync for blocking receive.
    pub fn think(
        &self,
        prompt: String,
        bindings: HashMap<String, Value>,
        expect: String,
    ) -> Result<mpsc::Receiver<ThinkResponse>, String> {
        let (response_tx, response_rx) = mpsc::channel();

        let request = ThinkRequest {
            prompt,
            bindings,
            expect,
            response_tx,
        };

        self.tx
            .send(request)
            .map_err(|e| format!("Failed to send think request: {}", e))?;

        Ok(response_rx)
    }
}
