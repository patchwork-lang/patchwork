/// Patchwork compiler
///
/// Transforms Patchwork source code into executable agent systems.
/// For the MVP, targets Claude Code plugins.

pub mod driver;
pub mod error;
pub mod codegen;
pub mod runtime;
pub mod prompts;
pub mod manifest;
pub mod module;

pub use driver::{Compiler, CompileOptions, CompileOutput};
pub use error::{CompileError, Result};
pub use codegen::CodeGenerator;
pub use prompts::{PromptTemplate, PromptKind};
pub use manifest::{PluginManifest, SkillEntry, CommandEntry};
pub use module::{ModuleResolver, Module, ModuleExports};
