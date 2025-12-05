//! Patchwork interpreter with synchronous blocking execution.
//!
//! This crate provides an interpreter for Patchwork code. Think blocks
//! block on channel operations waiting for LLM responses. Exceptions are
//! modeled as `Error::Exception(Value)` and propagate using Rust's `?` operator.

mod agent;
mod error;
mod eval;
mod interpreter;
mod runtime;
mod value;

pub use agent::{AgentHandle, ThinkRequest, ThinkResponse};
pub use error::Error;
pub use eval::{eval_block, eval_expr, eval_statement};
pub use interpreter::Interpreter;
pub use runtime::{PrintSink, Runtime};
pub use value::Value;

/// Result type for interpreter operations.
pub type Result<T> = std::result::Result<T, Error>;
