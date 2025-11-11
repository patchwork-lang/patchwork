/// Compiler driver that orchestrates the compilation pipeline

use std::path::PathBuf;
use patchwork_parser::ast::Program;
use crate::error::{CompileError, Result};

/// Compilation output structure
#[derive(Debug)]
pub struct CompileOutput {
    /// Source file that was compiled
    pub source_file: PathBuf,
    /// Parsed AST
    pub ast: Program<'static>,
    // Future: generated JS, markdown files, etc.
}

/// Options for compilation
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Input source file
    pub input: PathBuf,
    /// Output directory (optional, defaults to ./out)
    pub output_dir: Option<PathBuf>,
    /// Whether to print debug output
    pub verbose: bool,
}

impl CompileOptions {
    pub fn new(input: impl Into<PathBuf>) -> Self {
        Self {
            input: input.into(),
            output_dir: None,
            verbose: false,
        }
    }

    pub fn output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = Some(dir.into());
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

/// The Patchwork compiler
pub struct Compiler {
    options: CompileOptions,
}

impl Compiler {
    /// Create a new compiler with the given options
    pub fn new(options: CompileOptions) -> Self {
        Self { options }
    }

    /// Run the full compilation pipeline
    pub fn compile(&self) -> Result<CompileOutput> {
        if self.options.verbose {
            eprintln!("Compiling: {}", self.options.input.display());
        }

        // Phase 1: Read source file
        let source = self.read_source()?;

        // Phase 2: Parse source into AST
        let ast = self.parse(&source)?;

        if self.options.verbose {
            eprintln!("Parse successful: {} items", ast.items.len());
        }

        // For Phase 1, we just return the AST
        // Future phases will add semantic analysis, codegen, etc.
        Ok(CompileOutput {
            source_file: self.options.input.clone(),
            ast,
        })
    }

    /// Read the source file
    fn read_source(&self) -> Result<String> {
        if !self.options.input.exists() {
            return Err(CompileError::FileNotFound(self.options.input.clone()));
        }

        std::fs::read_to_string(&self.options.input).map_err(CompileError::from)
    }

    /// Parse source code into AST
    fn parse(&self, _source: &str) -> Result<Program<'static>> {
        // Use patchwork_parser to parse the source
        // For now, we'll use a simplified approach
        // TODO: Integrate with actual parser

        // Placeholder - will integrate with patchwork-parser
        // For now, return an empty program to satisfy the type system
        Ok(Program { items: vec![] })
    }

    /// Get the output directory (creates if needed)
    #[allow(dead_code)]
    fn get_output_dir(&self) -> Result<PathBuf> {
        let dir = self.options.output_dir.clone()
            .unwrap_or_else(|| PathBuf::from("./out"));

        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_options_builder() {
        let opts = CompileOptions::new("test.pw")
            .output_dir("./output")
            .verbose(true);

        assert_eq!(opts.input, PathBuf::from("test.pw"));
        assert_eq!(opts.output_dir, Some(PathBuf::from("./output")));
        assert!(opts.verbose);
    }
}
