/// Error types for the Patchwork compiler

use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CompileError>;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error in {file}: {message}")]
    Parse { file: PathBuf, message: String },

    #[error("Semantic error in {file}: {message}")]
    Semantic { file: PathBuf, message: String },

    #[error("Code generation error: {0}")]
    Codegen(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Multiple errors occurred:\n{}", .0.iter().map(|e| format!("  - {}", e)).collect::<Vec<_>>().join("\n"))]
    Multiple(Vec<CompileError>),

    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    #[error("Formatting error: {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Module not found: '{module}' imported from {from}")]
    ModuleNotFound { module: String, from: String },

    #[error("Module resolution error for {path}: {reason}")]
    ModuleResolution { path: String, reason: String },
}

impl CompileError {
    pub fn parse(file: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        CompileError::Parse {
            file: file.into(),
            message: message.into(),
        }
    }

    pub fn semantic(file: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        CompileError::Semantic {
            file: file.into(),
            message: message.into(),
        }
    }

    pub fn codegen(message: impl Into<String>) -> Self {
        CompileError::Codegen(message.into())
    }
}
