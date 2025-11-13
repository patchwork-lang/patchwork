/// Code generation module
///
/// Transforms Patchwork AST into executable JavaScript.

use patchwork_parser::ast::*;
use crate::error::{CompileError, Result};
use crate::prompts::{PromptTemplate, PromptKind, extract_prompt_template};
use crate::manifest::{PluginManifest, SkillEntry, CommandEntry};
use std::fmt::Write as _;

/// JavaScript code generator
pub struct CodeGenerator {
    /// Indentation level for pretty-printing
    indent: usize,
    /// Output buffer
    output: String,
    /// Prompt templates extracted during compilation
    prompts: Vec<PromptTemplate>,
    /// Counter for generating unique prompt IDs
    prompt_counter: usize,
    /// Plugin manifest (if trait with annotations is compiled)
    manifest: Option<PluginManifest>,
    /// Module ID for this compilation unit (used for generating relative imports)
    module_id: Option<String>,
    /// Current worker name (for generating prompt skill names)
    current_worker: Option<String>,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        Self {
            indent: 0,
            output: String::new(),
            prompts: Vec::new(),
            prompt_counter: 0,
            manifest: None,
            module_id: None,
            current_worker: None,
        }
    }

    /// Set the module ID for this compilation unit
    pub fn set_module_id(&mut self, module_id: impl Into<String>) {
        self.module_id = Some(module_id.into());
    }

    /// Get the extracted prompt templates
    pub fn prompts(&self) -> &[PromptTemplate] {
        &self.prompts
    }

    /// Get the plugin manifest (if any)
    pub fn manifest(&self) -> Option<&PluginManifest> {
        self.manifest.as_ref()
    }

    /// Generate JavaScript code for a program
    pub fn generate(&mut self, program: &Program) -> Result<String> {
        // First pass: generate user imports
        let mut has_imports = false;
        for item in &program.items {
            if let Item::Import(import_decl) = item {
                self.generate_import(import_decl)?;
                has_imports = true;
            }
        }
        if has_imports {
            self.output.push('\n');
        }

        // Add runtime imports
        self.generate_runtime_imports();

        // Second pass: generate code for other top-level items
        for item in &program.items {
            match item {
                Item::Worker(worker) => {
                    self.generate_worker(worker)?;
                    self.output.push('\n');
                }
                Item::Function(func) => {
                    self.generate_function(func)?;
                    self.output.push('\n');
                }
                Item::Import(_) => {
                    // Already handled in first pass
                }
                Item::Skill(_) => {
                    // TODO: Skill support not yet implemented
                }
                Item::Trait(trait_decl) => {
                    self.generate_trait(trait_decl)?;
                    self.output.push('\n');
                }
                Item::Type(_) => {
                    // TODO: Type declaration support not yet implemented
                }
            }
        }

        Ok(std::mem::take(&mut self.output))
    }

    /// Generate runtime imports
    fn generate_runtime_imports(&mut self) {
        // Import runtime primitives from the bundled runtime file
        let runtime_path = crate::runtime::get_runtime_module_name();
        self.output.push_str("// Patchwork runtime imports\n");
        write!(self.output, "import {{ shell, SessionContext, executePrompt, delegate }} from '{}';\n\n", runtime_path).unwrap();
    }

    /// Generate import statement
    fn generate_import(&mut self, import: &ImportDecl) -> Result<()> {
        match &import.path {
            ImportPath::Simple(segments) => {
                // Check if this is a standard library import
                if segments.first() == Some(&"std") {
                    // Standard library: import std.log -> import { log } from 'patchwork-runtime'
                    if segments.len() == 2 {
                        let name = segments[1];
                        write!(self.output, "import {{ {} }} from 'patchwork-runtime';\n", name)?;
                    }
                } else {
                    // Relative import: import ./module -> import * as module from './module.js'
                    let module_name = segments.last().unwrap_or(&"module");
                    let path = self.segments_to_path(segments);
                    write!(self.output, "import * as {} from '{}.js';\n", module_name, path)?;
                }
            }
            ImportPath::RelativeMulti(names) => {
                // Multi-import: import ./{a, b, c} -> multiple imports
                for name in names {
                    write!(self.output, "import * as {} from './{}.js';\n", name, name)?;
                }
            }
        }
        Ok(())
    }

    /// Convert path segments to a relative path string
    fn segments_to_path(&self, segments: &[&str]) -> String {
        let mut path = String::new();
        for (i, segment) in segments.iter().enumerate() {
            if i > 0 {
                path.push('/');
            }
            path.push_str(segment);
        }
        path
    }

    /// Generate code for a worker declaration
    fn generate_worker(&mut self, worker: &WorkerDecl) -> Result<()> {
        // Generate: export function workerName(session, params) { ... }
        // or: export default function workerName(session, params) { ... }
        // Note: Workers are always exported (for backward compatibility and runtime invocation)
        // Workers receive a session parameter as the first argument

        // Set current worker context for prompt skill naming
        self.current_worker = Some(worker.name.to_string());

        if worker.is_default {
            write!(self.output, "export default function {}", worker.name)?;
        } else {
            // Workers are always exported by default
            write!(self.output, "export function {}", worker.name)?;
        }

        self.generate_worker_params(&worker.params)?;
        self.output.push_str(" {\n");

        self.indent += 1;
        self.generate_block(&worker.body)?;
        self.indent -= 1;

        self.output.push_str("}\n");

        // Clear worker context after generation
        self.current_worker = None;

        Ok(())
    }

    /// Generate parameter list for workers (includes session parameter)
    fn generate_worker_params(&mut self, params: &[Param]) -> Result<()> {
        self.output.push('(');
        // Workers always receive session as first parameter
        self.output.push_str("session");
        // Add user-defined parameters
        for param in params {
            self.output.push_str(", ");
            self.output.push_str(param.name);
            // Type annotations are ignored in generated code
        }
        self.output.push(')');
        Ok(())
    }

    /// Generate code for a function declaration
    fn generate_function(&mut self, func: &FunctionDecl) -> Result<()> {
        // Generate: export function funcName(params) { ... }
        // or: export default function funcName(params) { ... }
        if func.is_default {
            write!(self.output, "export default function {}", func.name)?;
        } else if func.is_exported {
            write!(self.output, "export function {}", func.name)?;
        } else {
            write!(self.output, "function {}", func.name)?;
        }

        self.generate_params(&func.params)?;
        self.output.push_str(" {\n");

        self.indent += 1;
        self.generate_block(&func.body)?;
        self.indent -= 1;

        self.output.push_str("}\n");
        Ok(())
    }

    /// Generate parameter list
    fn generate_params(&mut self, params: &[Param]) -> Result<()> {
        self.output.push('(');
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(param.name);
            // Type annotations are ignored in generated code
        }
        self.output.push(')');
        Ok(())
    }

    /// Generate code for a trait declaration
    fn generate_trait(&mut self, trait_decl: &TraitDecl) -> Result<()> {
        // Traits compile to their methods as exported functions
        // Trait methods receive a session parameter (like workers) to support self.delegate() and self.session
        // Annotations (@skill, @command) are extracted and used for plugin manifest generation

        let export_prefix = if trait_decl.is_exported || trait_decl.is_default {
            "export "
        } else {
            ""
        };

        // Extract plugin manifest from trait with annotations
        if (trait_decl.is_exported || trait_decl.is_default) && !trait_decl.methods.is_empty() {
            self.extract_plugin_manifest(trait_decl);
        }

        for (i, method) in trait_decl.methods.iter().enumerate() {
            // Generate comment showing this came from a trait
            write!(self.output, "// Method from trait {}\n", trait_decl.name)?;

            // Generate the function (exported if trait is exported or default)
            // For default exports, only the first method gets "export default", rest are just "export"
            // Trait methods receive session as first parameter (like workers)
            if trait_decl.is_default && i == 0 {
                write!(self.output, "export default function {}", method.name)?;
            } else {
                write!(self.output, "{}function {}", export_prefix, method.name)?;
            }

            self.output.push('(');
            self.output.push_str("session");
            for param in &method.params {
                self.output.push_str(", ");
                self.output.push_str(param.name);
            }
            self.output.push(')');
            self.output.push_str(" {\n");

            self.indent += 1;
            self.generate_block(&method.body)?;
            self.indent -= 1;

            self.output.push_str("}\n\n");
        }

        Ok(())
    }

    /// Extract plugin manifest from trait annotations
    fn extract_plugin_manifest(&mut self, trait_decl: &TraitDecl) {
        // Only create manifest if there are annotations
        let has_annotations = trait_decl.methods.iter()
            .any(|m| !m.annotations.is_empty());

        if !has_annotations {
            return;
        }

        let mut manifest = PluginManifest::new(trait_decl.name.to_lowercase());

        // Process each method's annotations
        for method in &trait_decl.methods {
            let mut skill_name = None;
            let mut command_name = None;

            // Collect annotations
            for annotation in &method.annotations {
                match annotation.name {
                    "skill" => {
                        skill_name = Some(annotation.arg.unwrap_or(method.name).to_string());
                    }
                    "command" => {
                        command_name = Some(annotation.arg.unwrap_or(method.name).to_string());
                    }
                    _ => {}
                }
            }

            // Create skill entry
            if let Some(ref name) = skill_name {
                manifest.skills.push(SkillEntry {
                    name: name.clone(),
                    function: method.name.to_string(),
                    description: None,  // TODO: Extract from doc comments
                    params: method.params.iter().map(|p| p.name.to_string()).collect(),
                });
            }

            // Create command entry
            if let Some(cmd_name) = command_name {
                manifest.commands.push(CommandEntry {
                    name: cmd_name,
                    skill: skill_name,  // Link to skill if both annotations present
                    function: method.name.to_string(),
                    description: None,  // TODO: Extract from doc comments
                });
            }
        }

        self.manifest = Some(manifest);
    }

    /// Generate code for a block
    fn generate_block(&mut self, block: &Block) -> Result<()> {
        for stmt in &block.statements {
            self.write_indent();
            self.generate_statement(stmt)?;
            self.output.push('\n');
        }
        Ok(())
    }

    /// Generate code for a statement
    fn generate_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::VarDecl { pattern, init } => {
                self.generate_var_decl(pattern, init)?;
            }
            Statement::Expr(expr) => {
                self.generate_expr(expr)?;
                self.output.push(';');
            }
            Statement::If { condition, then_block, else_block } => {
                self.generate_if(condition, then_block, else_block)?;
            }
            Statement::While { condition, body } => {
                self.generate_while(condition, body)?;
            }
            Statement::ForIn { var, iter, body } => {
                self.generate_for_in(var, iter, body)?;
            }
            Statement::Return(expr) => {
                self.output.push_str("return");
                if let Some(e) = expr {
                    self.output.push(' ');
                    self.generate_expr(e)?;
                }
                self.output.push(';');
            }
            Statement::Break => {
                self.output.push_str("break;");
            }
            Statement::Succeed => {
                // TODO: Succeed statement not yet implemented
                self.output.push_str("// succeed statement (not yet implemented)");
            }
            Statement::TypeDecl { .. } => {
                // Type declarations are ignored in code generation
            }
        }
        Ok(())
    }

    /// Generate variable declaration
    fn generate_var_decl(&mut self, pattern: &Pattern, init: &Option<Expr>) -> Result<()> {
        match pattern {
            Pattern::Identifier { name, .. } => {
                // Simple case: var x = init
                write!(self.output, "let {}", name)?;
                if let Some(expr) = init {
                    self.output.push_str(" = ");
                    self.generate_expr(expr)?;
                }
                self.output.push(';');
            }
            Pattern::Ignore => {
                // var _ = expr → just evaluate expr
                if let Some(expr) = init {
                    self.generate_expr(expr)?;
                    self.output.push(';');
                }
            }
            Pattern::Object(fields) => {
                // Object destructuring: var {x, y} = expr
                self.output.push_str("let {");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    // For now, only support simple field patterns
                    match &field.pattern {
                        Pattern::Identifier { name, .. } => {
                            if field.key != *name {
                                // Key doesn't match name: {key: name}
                                write!(self.output, "{}: {}", field.key, name)?;
                            } else {
                                // Key matches name: shorthand {key}
                                self.output.push_str(field.key);
                            }
                        }
                        _ => {
                            return Err(CompileError::Unsupported(
                                "Nested patterns in object destructuring not yet supported".into()
                            ));
                        }
                    }
                }
                self.output.push('}');
                if let Some(expr) = init {
                    self.output.push_str(" = ");
                    self.generate_expr(expr)?;
                }
                self.output.push(';');
            }
            Pattern::Array(items) => {
                // Array destructuring: var [x, y, z] = expr
                // Support ignore patterns: var [_, x, _] = expr
                self.output.push_str("let [");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    match item {
                        Pattern::Identifier { name, .. } => {
                            self.output.push_str(name);
                        }
                        Pattern::Ignore => {
                            // Empty slot in JavaScript destructuring
                            // [, x, ] is valid JS for ignoring positions
                        }
                        _ => {
                            return Err(CompileError::Unsupported(
                                "Nested patterns in array destructuring not yet supported".into()
                            ));
                        }
                    }
                }
                self.output.push(']');
                if let Some(expr) = init {
                    self.output.push_str(" = ");
                    self.generate_expr(expr)?;
                }
                self.output.push(';');
            }
        }
        Ok(())
    }

    /// Generate if statement
    fn generate_if(&mut self, condition: &Expr, then_block: &Block, else_block: &Option<Block>) -> Result<()> {
        self.output.push_str("if (");
        self.generate_expr(condition)?;
        self.output.push_str(") {\n");

        self.indent += 1;
        self.generate_block(then_block)?;
        self.indent -= 1;

        self.write_indent();
        self.output.push('}');

        if let Some(else_blk) = else_block {
            self.output.push_str(" else {\n");
            self.indent += 1;
            self.generate_block(else_blk)?;
            self.indent -= 1;
            self.write_indent();
            self.output.push('}');
        }

        Ok(())
    }

    /// Generate while loop
    fn generate_while(&mut self, condition: &Expr, body: &Block) -> Result<()> {
        self.output.push_str("while (");
        self.generate_expr(condition)?;
        self.output.push_str(") {\n");

        self.indent += 1;
        self.generate_block(body)?;
        self.indent -= 1;

        self.write_indent();
        self.output.push('}');
        Ok(())
    }

    /// Generate for-in loop
    fn generate_for_in(&mut self, var: &str, iter: &Expr, body: &Block) -> Result<()> {
        self.output.push_str("for (let ");
        self.output.push_str(var);
        self.output.push_str(" of ");
        self.generate_expr(iter)?;
        self.output.push_str(") {\n");

        self.indent += 1;
        self.generate_block(body)?;
        self.indent -= 1;

        self.write_indent();
        self.output.push('}');
        Ok(())
    }

    /// Generate code for an expression
    fn generate_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Identifier(name) => {
                // Bare 'self' is not supported (only self.session)
                if *name == "self" {
                    return Err(CompileError::Codegen(
                        "Bare 'self' is not supported. Use self.session to access the session context".to_string()
                    ));
                }
                self.output.push_str(name);
            }
            Expr::Number(n) => {
                self.output.push_str(n);
            }
            Expr::String(s) => {
                self.generate_string_literal(s)?;
            }
            Expr::True => {
                self.output.push_str("true");
            }
            Expr::False => {
                self.output.push_str("false");
            }
            Expr::Array(items) => {
                self.output.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.generate_expr(item)?;
                }
                self.output.push(']');
            }
            Expr::Object(fields) => {
                self.output.push_str("{ ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(field.key);
                    if let Some(val) = &field.value {
                        self.output.push_str(": ");
                        self.generate_expr(val)?;
                    }
                    // If value is None, it's shorthand syntax {x} → {x: x}
                }
                self.output.push_str(" }");
            }
            Expr::Binary { op, left, right } => {
                self.generate_binary_op(op, left, right)?;
            }
            Expr::Unary { op, operand } => {
                self.generate_unary_op(op, operand)?;
            }
            Expr::Call { callee, args } => {
                // Check if this is a self.delegate(...) call
                if let Expr::Member { object, field } = &**callee {
                    if let Expr::Identifier("self") = &**object {
                        if *field == "delegate" {
                            // self.delegate([...]) -> delegate(session, [...])
                            self.output.push_str("delegate(session, ");
                            for (i, arg) in args.iter().enumerate() {
                                if i > 0 {
                                    self.output.push_str(", ");
                                }
                                self.generate_expr(arg)?;
                            }
                            self.output.push(')');
                            return Ok(());
                        }
                    }
                }

                // Regular function call
                self.generate_expr(callee)?;
                self.output.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.generate_expr(arg)?;
                }
                self.output.push(')');
            }
            Expr::Member { object, field } => {
                // Handle self.field access
                if let Expr::Identifier("self") = **object {
                    if *field == "session" {
                        // self.session -> session
                        self.output.push_str("session");
                        return Ok(());
                    } else if *field == "delegate" {
                        // self.delegate is used in method calls, handled in Expr::Call
                        // But if accessed directly, just output "delegate"
                        self.output.push_str("delegate");
                        return Ok(());
                    } else {
                        // self.something_else -> error (not supported)
                        return Err(CompileError::Codegen(
                            format!("self.{} is not supported. Only self.session and self.delegate() are available", field)
                        ));
                    }
                }
                // Regular member access
                self.generate_expr(object)?;
                self.output.push('.');
                self.output.push_str(field);
            }
            Expr::Index { object, index } => {
                self.generate_expr(object)?;
                self.output.push('[');
                self.generate_expr(index)?;
                self.output.push(']');
            }
            Expr::Paren(inner) => {
                self.output.push('(');
                self.generate_expr(inner)?;
                self.output.push(')');
            }
            Expr::PostIncrement(operand) => {
                self.generate_expr(operand)?;
                self.output.push_str("++");
            }
            Expr::PostDecrement(operand) => {
                self.generate_expr(operand)?;
                self.output.push_str("--");
            }
            Expr::Await(inner) => {
                self.output.push_str("await ");
                self.generate_expr(inner)?;
            }
            Expr::BareCommand { name, args } => {
                // Shell command statement form
                self.generate_shell_command(name, args)?;
            }
            Expr::CommandSubst(cmd) => {
                // Shell command expression form: $(...)
                self.generate_command_subst(cmd)?;
            }
            Expr::ShellPipe { left, right } => {
                self.generate_shell_pipe(left, right)?;
            }
            Expr::ShellAnd { left, right } => {
                self.generate_shell_and(left, right)?;
            }
            Expr::ShellOr { left, right } => {
                self.generate_shell_or(left, right)?;
            }
            Expr::ShellRedirect { command, op, target } => {
                self.generate_shell_redirect(command, op, target)?;
            }
            Expr::Think(block) => {
                self.generate_prompt_expr(block, PromptKind::Think)?;
            }
            Expr::Ask(block) => {
                self.generate_prompt_expr(block, PromptKind::Ask)?;
            }
            Expr::Do(_) => {
                // TODO: Do expressions not yet implemented
                return Err(CompileError::Unsupported("Do expressions not yet supported".into()));
            }
        }
        Ok(())
    }

    /// Generate string literal with interpolation support
    fn generate_string_literal(&mut self, s: &StringLiteral) -> Result<()> {
        if s.parts.len() == 1 {
            if let StringPart::Text(text) = &s.parts[0] {
                // Simple string with no interpolation
                write!(self.output, "\"{}\"", escape_string(text))?;
                return Ok(());
            }
        }

        // String with interpolation → use template literal
        self.output.push('`');
        for part in &s.parts {
            match part {
                StringPart::Text(text) => {
                    self.output.push_str(&escape_template_literal(text));
                }
                StringPart::Interpolation(expr) => {
                    self.output.push_str("${");
                    self.generate_expr(expr)?;
                    self.output.push('}');
                }
            }
        }
        self.output.push('`');
        Ok(())
    }

    /// Generate binary operation
    fn generate_binary_op(&mut self, op: &BinOp, left: &Expr, right: &Expr) -> Result<()> {
        self.generate_expr(left)?;
        let op_str = match op {
            BinOp::Add => " + ",
            BinOp::Sub => " - ",
            BinOp::Mul => " * ",
            BinOp::Div => " / ",
            BinOp::Eq => " === ",
            BinOp::NotEq => " !== ",
            BinOp::Lt => " < ",
            BinOp::Gt => " > ",
            BinOp::And => " && ",
            BinOp::Or => " || ",
            BinOp::Assign => " = ",
            BinOp::Pipe => {
                // Pipe operator for shell - handled specially
                return Err(CompileError::Unsupported("Use ShellPipe expression for shell pipes".into()));
            }
            BinOp::Range => {
                // Range operator - not yet implemented
                return Err(CompileError::Unsupported("Range operator not yet supported".into()));
            }
        };
        self.output.push_str(op_str);
        self.generate_expr(right)?;
        Ok(())
    }

    /// Generate unary operation
    fn generate_unary_op(&mut self, op: &UnOp, operand: &Expr) -> Result<()> {
        match op {
            UnOp::Not => self.output.push('!'),
            UnOp::Neg => self.output.push('-'),
            UnOp::Throw => {
                // throw expr → throw new Error(String(expr))
                self.output.push_str("throw new Error(String(");
                self.generate_expr(operand)?;
                self.output.push_str("))");
                return Ok(());
            }
        }
        self.generate_expr(operand)?;
        Ok(())
    }

    /// Generate shell command execution (statement form)
    fn generate_shell_command(&mut self, name: &str, args: &[CommandArg]) -> Result<()> {
        // Generate a runtime function call
        self.output.push_str("await $shell(");

        // Build command string
        self.output.push('`');
        self.output.push_str(name);
        for arg in args {
            self.output.push(' ');
            match arg {
                CommandArg::Literal(lit) => {
                    self.output.push_str(lit);
                }
                CommandArg::String(s) => {
                    // Embedded string in command
                    for part in &s.parts {
                        match part {
                            StringPart::Text(text) => {
                                self.output.push_str(&escape_template_literal(text));
                            }
                            StringPart::Interpolation(expr) => {
                                self.output.push_str("${");
                                self.generate_expr(expr)?;
                                self.output.push('}');
                            }
                        }
                    }
                }
            }
        }
        self.output.push('`');
        self.output.push(')');
        Ok(())
    }

    /// Generate command substitution (expression form)
    fn generate_command_subst(&mut self, cmd: &Expr) -> Result<()> {
        // $(cmd) → await $shell(cmd, {capture: true})
        self.output.push_str("await $shell(");

        // If cmd is a bare command, extract its parts
        if let Expr::BareCommand { name, args } = cmd {
            self.output.push('`');
            self.output.push_str(name);
            for arg in args {
                self.output.push(' ');
                match arg {
                    CommandArg::Literal(lit) => {
                        self.output.push_str(lit);
                    }
                    CommandArg::String(s) => {
                        for part in &s.parts {
                            match part {
                                StringPart::Text(text) => {
                                    self.output.push_str(&escape_template_literal(text));
                                }
                                StringPart::Interpolation(expr) => {
                                    self.output.push_str("${");
                                    self.generate_expr(expr)?;
                                    self.output.push('}');
                                }
                            }
                        }
                    }
                }
            }
            self.output.push('`');
        } else {
            // Complex expression - just generate it
            self.generate_expr(cmd)?;
        }

        self.output.push_str(", {capture: true})");
        Ok(())
    }

    /// Generate shell pipe
    fn generate_shell_pipe(&mut self, left: &Expr, right: &Expr) -> Result<()> {
        // cmd1 | cmd2 → await $shellPipe([cmd1, cmd2])
        self.output.push_str("await $shellPipe([");
        self.generate_shell_expr_for_pipe(left)?;
        self.output.push_str(", ");
        self.generate_shell_expr_for_pipe(right)?;
        self.output.push_str("])");
        Ok(())
    }

    /// Generate shell && operator
    fn generate_shell_and(&mut self, left: &Expr, right: &Expr) -> Result<()> {
        // cmd1 && cmd2 → await $shellAnd([cmd1, cmd2])
        self.output.push_str("await $shellAnd([");
        self.generate_shell_expr_for_pipe(left)?;
        self.output.push_str(", ");
        self.generate_shell_expr_for_pipe(right)?;
        self.output.push_str("])");
        Ok(())
    }

    /// Generate shell || operator
    fn generate_shell_or(&mut self, left: &Expr, right: &Expr) -> Result<()> {
        // cmd1 || cmd2 → await $shellOr([cmd1, cmd2])
        self.output.push_str("await $shellOr([");
        self.generate_shell_expr_for_pipe(left)?;
        self.output.push_str(", ");
        self.generate_shell_expr_for_pipe(right)?;
        self.output.push_str("])");
        Ok(())
    }

    /// Generate shell redirect
    fn generate_shell_redirect(&mut self, command: &Expr, op: &RedirectOp, target: &Expr) -> Result<()> {
        // cmd > file → await $shellRedirect(cmd, '>', file)
        self.output.push_str("await $shellRedirect(");
        self.generate_shell_expr_for_pipe(command)?;
        self.output.push_str(", '");
        let op_str = match op {
            RedirectOp::Out => ">",
            RedirectOp::Append => ">>",
            RedirectOp::In => "<",
            RedirectOp::ErrOut => "2>",
            RedirectOp::ErrToOut => "2>&1",
        };
        self.output.push_str(op_str);
        self.output.push_str("', ");
        self.generate_expr(target)?;
        self.output.push(')');
        Ok(())
    }

    /// Helper to generate shell expressions for piping
    fn generate_shell_expr_for_pipe(&mut self, expr: &Expr) -> Result<()> {
        if let Expr::BareCommand { name, args } = expr {
            self.output.push('`');
            self.output.push_str(name);
            for arg in args {
                self.output.push(' ');
                match arg {
                    CommandArg::Literal(lit) => {
                        self.output.push_str(lit);
                    }
                    CommandArg::String(s) => {
                        for part in &s.parts {
                            match part {
                                StringPart::Text(text) => {
                                    self.output.push_str(&escape_template_literal(text));
                                }
                                StringPart::Interpolation(expr) => {
                                    self.output.push_str("${");
                                    self.generate_expr(expr)?;
                                    self.output.push('}');
                                }
                            }
                        }
                    }
                }
            }
            self.output.push('`');
        } else {
            self.generate_expr(expr)?;
        }
        Ok(())
    }

    /// Write current indentation
    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    /// Generate code for a prompt expression (think/ask)
    fn generate_prompt_expr(&mut self, block: &PromptBlock, kind: PromptKind) -> Result<()> {
        // Get current worker name (required for skill naming)
        let worker_name = self.current_worker.clone().ok_or_else(|| {
            CompileError::Unsupported(
                "Prompt blocks (think/ask) must be used inside a worker".into()
            )
        })?;

        // Generate a unique ID for this prompt within the worker
        let id = format!("{}_{}", kind.as_str(), self.prompt_counter);
        self.prompt_counter += 1;

        // Extract the prompt template with worker context
        let template = extract_prompt_template(block, kind, id.clone(), worker_name)?;

        // Generate skill name: {worker_name}_{kind}_{n}
        let skill_name = format!("{}_{}", template.worker_name, template.id);

        // Generate the IPC call:
        // await executePrompt(session, 'worker_think_0', { name: name, description: description })
        self.output.push_str("await executePrompt(session, '");
        self.output.push_str(&skill_name);
        self.output.push_str("', { ");

        // Generate the bindings object
        let mut first = true;
        for binding in &template.required_bindings {
            if !first {
                self.output.push_str(", ");
            }
            first = false;
            // binding: binding (or just use shorthand syntax)
            self.output.push_str(binding);
        }

        self.output.push_str(" })");

        // Store the template for later markdown generation
        self.prompts.push(template);

        Ok(())
    }
}

/// Escape a string for double-quoted JavaScript string literal
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape a string for JavaScript template literal
fn escape_template_literal(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_string("say \"hi\""), "say \\\"hi\\\"");
    }

    #[test]
    fn test_escape_template_literal() {
        assert_eq!(escape_template_literal("hello"), "hello");
        assert_eq!(escape_template_literal("use `backticks`"), "use \\`backticks\\`");
        assert_eq!(escape_template_literal("embed ${x}"), "embed \\${x}");
    }
}
