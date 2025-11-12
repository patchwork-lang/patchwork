/// Compiler driver that orchestrates the compilation pipeline

use std::path::PathBuf;
use std::collections::HashMap;
use patchwork_parser::ast::Program;
use crate::error::{CompileError, Result};
use crate::codegen::CodeGenerator;
use crate::prompts::PromptTemplate;

/// Compilation output structure
pub struct CompileOutput {
    /// Source file that was compiled
    pub source_file: PathBuf,
    /// Source code (kept alive for AST references)
    pub source: String,
    /// Generated JavaScript code
    pub javascript: String,
    /// Runtime JavaScript code 
    pub runtime: String,
    /// Prompt templates extracted during compilation 
    /// Map from template ID to markdown content
    pub prompts: HashMap<String, String>,
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

        // Read source file
        let source = self.read_source()?;

        // Parse source into AST
        let ast = self.parse(&source)?;

        if self.options.verbose {
            eprintln!("Parse successful: {} items", ast.items.len());
        }

        // Generate JavaScript code
        let (javascript, prompt_templates) = self.generate_code(&ast)?;

        if self.options.verbose {
            eprintln!("Code generation successful: {} bytes", javascript.len());
            eprintln!("Extracted {} prompt templates", prompt_templates.len());
        }

        // Include runtime code
        let runtime = crate::runtime::get_runtime_code().to_string();

        // Convert prompt templates to markdown map
        let prompts = prompt_templates.into_iter()
            .map(|t| (t.id.clone(), t.markdown.clone()))
            .collect();

        Ok(CompileOutput {
            source_file: self.options.input.clone(),
            source,
            javascript,
            runtime,
            prompts,
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
    fn parse<'a>(&self, source: &'a str) -> Result<Program<'a>> {
        // Use patchwork_parser to parse the source
        patchwork_parser::parse(source)
            .map_err(|e| CompileError::parse(&self.options.input, e.to_string()))
    }

    /// Generate JavaScript code from AST and extract prompt templates
    fn generate_code(&self, ast: &Program) -> Result<(String, Vec<PromptTemplate>)> {
        let mut generator = CodeGenerator::new();
        let javascript = generator.generate(ast)?;
        let prompts = generator.prompts().to_vec();
        Ok((javascript, prompts))
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
