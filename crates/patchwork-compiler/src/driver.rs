/// Compiler driver that orchestrates the compilation pipeline

use std::path::PathBuf;
use std::collections::HashMap;
use patchwork_parser::ast::Program;
use crate::error::{CompileError, Result};
use crate::codegen::CodeGenerator;
use crate::prompts::{PromptTemplate, PromptKind};
use crate::manifest::PluginManifest;
use crate::module::ModuleResolver;
use crate::typecheck::TypeChecker;

/// Compilation output structure
pub struct CompileOutput {
    /// Source file that was compiled (entry point)
    pub source_file: PathBuf,
    /// Source code (kept alive for AST references) - for single-file mode
    pub source: String,
    /// Generated JavaScript code (single-file mode) or entry point module (multi-file mode)
    pub javascript: String,
    /// Generated JavaScript modules by module ID (multi-file mode)
    pub modules: HashMap<String, String>,
    /// Runtime JavaScript code
    pub runtime: String,
    /// Prompt templates extracted during compilation
    /// Map from template ID to markdown content
    pub prompts: HashMap<String, String>,
    /// Plugin manifest (if trait with annotations was compiled)
    /// Map from relative path to file content
    pub manifest_files: HashMap<String, String>,
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

    /// Run the full compilation pipeline (multi-file mode)
    pub fn compile(&self) -> Result<CompileOutput> {
        // Determine if this is a multi-file project by checking for imports
        let source = self.read_source()?;

        // Quick check: does the source contain "import" keyword?
        // This is a heuristic to avoid parsing twice in most cases
        let has_imports = source.contains("import ");

        if has_imports {
            // Multi-file compilation
            self.compile_multi_file()
        } else {
            // Single-file compilation (backward compatibility)
            // Parse the AST - we need to keep source_copy alive for the AST references
            let source_copy = source.clone();
            let ast = self.parse(&source_copy)?;
            self.compile_single_file(source, ast)
        }
    }

    /// Run single-file compilation (original behavior)
    fn compile_single_file(&self, source: String, ast: Program) -> Result<CompileOutput> {
        if self.options.verbose {
            eprintln!("Compiling: {} (single-file mode)", self.options.input.display());
        }

        if self.options.verbose {
            eprintln!("Parse successful: {} items", ast.items.len());
        }

        // Type check the AST
        let mut type_checker = TypeChecker::new();
        type_checker.check_program(&ast)?;

        if self.options.verbose {
            eprintln!("Type checking successful");
        }

        // Generate JavaScript code
        let (javascript, prompt_templates, manifest) = self.generate_code(&ast)?;

        if self.options.verbose {
            eprintln!("Code generation successful: {} bytes", javascript.len());
            eprintln!("Extracted {} prompt templates", prompt_templates.len());
            if manifest.is_some() {
                eprintln!("Generated plugin manifest");
            }
        }

        // Include runtime code
        let runtime = crate::runtime::get_runtime_code().to_string();

        // Convert prompt templates to skill documents
        let prompts = prompt_templates.into_iter()
            .map(|t| {
                // For single-file mode, use "main" as module prefix
                let skill_name = format!("main_{}_{}", t.worker_name, t.id);
                let skill_content = generate_skill_document(&skill_name, &t);
                let skill_path = format!("skills/{}/SKILL.md", skill_name);
                (skill_path, skill_content)
            })
            .collect();

        // Convert manifest to file map
        let manifest_files = manifest.map(|m| m.get_files())
            .unwrap_or_default();

        Ok(CompileOutput {
            source_file: self.options.input.clone(),
            source,
            javascript,
            modules: HashMap::new(), // Single-file mode doesn't use modules
            runtime,
            prompts,
            manifest_files,
        })
    }

    /// Run multi-file compilation
    fn compile_multi_file(&self) -> Result<CompileOutput> {
        if self.options.verbose {
            eprintln!("Compiling: {} (multi-file mode)", self.options.input.display());
        }

        // Get the root directory (parent of entry file)
        let root = self.options.input.parent()
            .ok_or_else(|| CompileError::ModuleResolution {
                path: self.options.input.display().to_string(),
                reason: "Cannot determine parent directory".to_string(),
            })?;

        // Resolve all modules starting from entry point
        let mut resolver = ModuleResolver::new(root);
        resolver.resolve(&self.options.input)?;

        if self.options.verbose {
            eprintln!("Resolved {} modules", resolver.modules().len());
        }

        // Compile each module in dependency order
        let mut modules = HashMap::new();
        let mut all_prompts = HashMap::new();
        let mut manifest_files = HashMap::new();

        let compilation_order = resolver.compilation_order();

        for module in compilation_order {
            if self.options.verbose {
                eprintln!("Compiling module: {}", module.id);
            }

            // Re-parse the AST from the stored source
            // This ensures the AST references are valid
            let ast = patchwork_parser::parse(&module.source)
                .map_err(|e| CompileError::parse(&module.path, e.to_string()))?;

            // Type check the module
            let mut type_checker = TypeChecker::new();
            type_checker.check_program(&ast)?;

            if self.options.verbose {
                eprintln!("Type checking successful for module: {}", module.id);
            }

            let mut generator = CodeGenerator::new();
            generator.set_module_id(&module.id);

            let javascript = generator.generate(&ast)?;

            // Collect prompts from this module and generate skill documents
            for prompt in generator.prompts() {
                // Generate skill name: {module}_{worker}_{kind}_{n}
                let skill_name = format!("{}_{}_{}",
                    module.id.replace('/', "_"),
                    prompt.worker_name,
                    prompt.id
                );

                // Generate skill document content
                let skill_content = generate_skill_document(&skill_name, prompt);

                // Store skill document at skills/{skill_name}/SKILL.md
                let skill_path = format!("skills/{}/SKILL.md", skill_name);
                all_prompts.insert(skill_path, skill_content);
            }

            // Collect manifest from entry point module only
            if module.path == self.options.input {
                if let Some(manifest) = generator.manifest() {
                    manifest_files = manifest.get_files();
                }
            }

            modules.insert(format!("{}.js", module.id), javascript);
        }

        if self.options.verbose {
            eprintln!("Generated {} modules", modules.len());
            eprintln!("Extracted {} prompt templates", all_prompts.len());
            if !manifest_files.is_empty() {
                eprintln!("Generated plugin manifest");
            }
        }

        // Include runtime code
        let runtime = crate::runtime::get_runtime_code().to_string();

        // Get entry point module ID
        let entry_module_id = resolver.modules().iter()
            .find(|(_, m)| m.path == self.options.input)
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| "main".to_string());

        let entry_javascript = modules.get(&format!("{}.js", entry_module_id))
            .cloned()
            .unwrap_or_default();

        Ok(CompileOutput {
            source_file: self.options.input.clone(),
            source: String::new(), // Multi-file mode doesn't keep single source
            javascript: entry_javascript,
            modules,
            runtime,
            prompts: all_prompts,
            manifest_files,
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
    fn generate_code(&self, ast: &Program) -> Result<(String, Vec<PromptTemplate>, Option<PluginManifest>)> {
        let mut generator = CodeGenerator::new();
        let javascript = generator.generate(ast)?;
        let prompts = generator.prompts().to_vec();
        let manifest = generator.manifest().cloned();
        Ok((javascript, prompts, manifest))
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

/// Generate a skill document for a prompt block
fn generate_skill_document(skill_name: &str, prompt: &PromptTemplate) -> String {
    let kind_label = match prompt.kind {
        PromptKind::Think => "Think",
        PromptKind::Ask => "Ask",
    };

    let mut doc = String::new();

    // Frontmatter
    doc.push_str("---\n");
    doc.push_str(&format!("name: {}\n", skill_name));
    doc.push_str(&format!("description: {} block from worker {}\n", kind_label, prompt.worker_name));
    doc.push_str("allowed-tools: All\n");
    doc.push_str("---\n\n");

    // Title
    doc.push_str(&format!("# {} - {} Block\n\n",
        prompt.worker_name,
        kind_label
    ));

    // Variable bindings section (if any)
    if !prompt.required_bindings.is_empty() {
        doc.push_str("## Input Variables\n\n");
        doc.push_str("The following variables are available:\n\n");

        let mut bindings: Vec<_> = prompt.required_bindings.iter().collect();
        bindings.sort();

        for binding in bindings {
            doc.push_str(&format!("- `{}`: ${{BINDING_{}}}\n", binding, binding));
        }
        doc.push_str("\n");
    }

    // Task section with the actual prompt content
    doc.push_str("## Task\n\n");
    doc.push_str(&prompt.markdown);
    doc.push_str("\n\n");

    // Output section
    doc.push_str("## Output\n\n");
    match prompt.kind {
        PromptKind::Think => {
            doc.push_str("Return your analysis result as structured data.\n");
        }
        PromptKind::Ask => {
            doc.push_str("Interact with the user and return their response.\n");
        }
    }

    doc
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
