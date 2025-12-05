//! The Patchwork interpreter.
//!
//! This module provides a synchronous interpreter for Patchwork code.
//! Think blocks block on channel operations waiting for LLM responses.

use std::path::PathBuf;

use patchwork_parser::ast::{Expr, Statement};

use crate::agent::AgentHandle;
use crate::error::Error;
use crate::eval;
use crate::runtime::{PrintSink, Runtime};
use crate::value::Value;

/// The Patchwork interpreter.
///
/// Executes Patchwork code synchronously. Think blocks block on channel
/// operations waiting for LLM responses from the agent.
pub struct Interpreter {
    /// Runtime environment with variable bindings.
    runtime: Runtime,
    /// Optional agent handle for think blocks.
    agent: Option<AgentHandle>,
}

impl Interpreter {
    /// Create a new interpreter without an agent.
    ///
    /// Think blocks will return placeholder values instead of blocking on LLM.
    pub fn new() -> Self {
        Self {
            runtime: Runtime::default(),
            agent: None,
        }
    }

    /// Create a new interpreter with an agent handle.
    ///
    /// Think blocks will block on the agent channel waiting for LLM responses.
    pub fn with_agent(agent: AgentHandle) -> Self {
        Self {
            runtime: Runtime::default(),
            agent: Some(agent),
        }
    }

    /// Create a new interpreter with a specific working directory and agent.
    pub fn with_working_dir_and_agent(working_dir: PathBuf, agent: AgentHandle) -> Self {
        Self {
            runtime: Runtime::new(working_dir),
            agent: Some(agent),
        }
    }

    /// Create a new interpreter with a specific working directory.
    pub fn with_working_dir(working_dir: PathBuf) -> Self {
        Self {
            runtime: Runtime::new(working_dir),
            agent: None,
        }
    }

    /// Get a reference to the runtime.
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Get a mutable reference to the runtime.
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    /// Get a reference to the agent handle, if present.
    pub fn agent(&self) -> Option<&AgentHandle> {
        self.agent.as_ref()
    }

    /// Set a print sink for redirecting print() output.
    ///
    /// When set, all print() calls will send to this channel instead of stdout.
    pub fn set_print_sink(&mut self, sink: PrintSink) {
        self.runtime.set_print_sink(sink);
    }

    /// Evaluate Patchwork code.
    ///
    /// Parses and executes the code, returning the final value or an error.
    ///
    /// For ACP usage, code starting with `{` is wrapped in a skill for execution.
    pub fn eval(&mut self, code: &str) -> crate::Result<Value> {
        // For ACP, bare blocks `{ ... }` need to be wrapped in a skill to be valid
        let wrapped_code;
        let code_to_parse = if code.trim_start().starts_with('{') {
            wrapped_code = format!("skill __main__() {}", code);
            &wrapped_code
        } else {
            code
        };

        // Parse the code using patchwork-parser
        match patchwork_parser::parse(code_to_parse) {
            Ok(ast) => {
                eprintln!("[patchwork-eval] Parsed AST: {:?}", ast);

                // Execute the program - look for the __main__ skill or evaluate items
                self.execute_program(&ast)
            }
            Err(e) => {
                let msg = format_parse_error(&e, code_to_parse);
                Err(Error::Parse(msg))
            }
        }
    }

    /// Execute a parsed program.
    fn execute_program(&mut self, program: &patchwork_parser::Program) -> crate::Result<Value> {
        use patchwork_parser::Item;

        // Look for __main__ skill (from wrapped block) or execute items
        for item in &program.items {
            match item {
                Item::Skill(skill) if skill.name == "__main__" => {
                    // Execute the main skill's body
                    return eval::eval_block(&skill.body, &mut self.runtime, self.agent.as_ref());
                }
                Item::Function(func) if func.name == "__main__" => {
                    // Execute the main function's body
                    return eval::eval_block(&func.body, &mut self.runtime, self.agent.as_ref());
                }
                _ => {
                    // Other items (imports, type decls, etc.) - currently ignored
                    // In a full implementation, we'd register functions/skills
                }
            }
        }

        // No __main__ found, evaluate as program items
        eval::eval_program(program, &mut self.runtime, self.agent.as_ref())
    }

    /// Evaluate a single expression directly (for testing).
    pub fn eval_expr(&mut self, expr: &Expr) -> crate::Result<Value> {
        eval::eval_expr(expr, &mut self.runtime, self.agent.as_ref())
    }

    /// Evaluate a single statement directly (for testing).
    pub fn eval_stmt(&mut self, stmt: &Statement) -> crate::Result<Value> {
        eval::eval_statement(stmt, &mut self.runtime, self.agent.as_ref())
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a parse error with source context.
fn format_parse_error(error: &patchwork_parser::ParseError, source: &str) -> String {
    use patchwork_parser::ParseError;

    let (message, span) = match error {
        ParseError::LexerError { message, span, .. } => (message.clone(), *span),
        ParseError::UnexpectedToken { message, span, .. } => (message.clone(), *span),
    };

    // If we have a span, add line/column information and a source snippet
    if let Some((start, end)) = span {
        let (line, col) = byte_offset_to_line_col(source, start);
        let source_line = get_source_line(source, line);

        // Build error message with context
        let mut result = format!("at line {}, column {}: {}\n", line, col, message);

        // Add the source line
        result.push_str(&format!("  {}\n", source_line));

        // Add the error indicator
        let spaces = " ".repeat(col.saturating_sub(1) + 2);
        let carets = "^".repeat((end - start).max(1));
        result.push_str(&format!("{}{}", spaces, carets));

        result
    } else {
        message
    }
}

/// Convert a byte offset to line and column numbers (1-indexed).
fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut line_start = 0;

    for (i, c) in source.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }

    let col = offset - line_start + 1;
    (line, col)
}

/// Get the source line at the given line number (1-indexed).
fn get_source_line(source: &str, line_num: usize) -> &str {
    source
        .lines()
        .nth(line_num.saturating_sub(1))
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_interpreter() {
        let _interp = Interpreter::new();
        // Interpreter is ready to evaluate
    }

    #[test]
    fn test_eval_empty_program() {
        let mut interp = Interpreter::new();
        // Empty program is valid Patchwork
        let result = interp.eval("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_simple_function() {
        let mut interp = Interpreter::new();
        // A simple function definition
        let result = interp.eval("fun hello() {}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_eval_block_with_var() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var x = 42
            x
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::Number(n)) = result {
            assert_eq!(n, 42.0);
        } else {
            panic!("Expected Number(42), got {:?}", result);
        }
    }

    #[test]
    fn test_eval_for_loop() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var sum = 0
            for var i in [1, 2, 3] {
                sum = sum + i
            }
            sum
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::Number(n)) = result {
            assert_eq!(n, 6.0);
        } else {
            panic!("Expected Number(6), got {:?}", result);
        }
    }

    #[test]
    fn test_eval_json_parse_from_file() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temp file with JSON content
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"name": "test", "value": 123}}"#).unwrap();
        let path = file.path().to_str().unwrap();

        let mut interp = Interpreter::new();
        let code = format!(r#"{{
            var text = read("{}")
            var data = json(text)
            data.name
        }}"#, path);

        let result = interp.eval(&code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, "test");
        } else {
            panic!("Expected String(\"test\"), got {:?}", result);
        }
    }

    #[test]
    fn test_eval_json_with_read() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temp file with JSON content
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, r#"{{"name": "test", "value": 123}}"#).unwrap();
        let path = file.path().to_str().unwrap();

        let mut interp = Interpreter::new();
        // Test using read() + json() - the standard pattern
        let code = format!(r#"{{
            var text = read("{}")
            var data = json(text)
            data.name
        }}"#, path);

        let result = interp.eval(&code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, "test");
        } else {
            panic!("Expected String(\"test\"), got {:?}", result);
        }
    }

    #[test]
    fn test_eval_cat_function() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var obj = { name: "hello", value: 42 }
            cat(obj)
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::String(s)) = result {
            assert!(s.contains("\"name\""));
            assert!(s.contains("\"hello\""));
        } else {
            panic!("Expected String, got {:?}", result);
        }
    }

    #[test]
    fn test_eval_destructuring() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var data = { x: 10, y: 20 }
            var { x, y } = data
            x + y
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::Number(n)) = result {
            assert_eq!(n, 30.0);
        } else {
            panic!("Expected Number(30), got {:?}", result);
        }
    }

    #[test]
    fn test_eval_if_else() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var x = 10
            if x > 5 {
                "big"
            } else {
                "small"
            }
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, "big");
        } else {
            panic!("Expected String(\"big\"), got {:?}", result);
        }
    }

    #[test]
    fn test_phase2_demo_simplified() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temp directory structure mimicking the demo
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();

        // Create interview directories with metadata.json files
        for name in ["interview1", "interview2"] {
            let dir = base.join(name);
            fs::create_dir(&dir).unwrap();

            let metadata = format!(
                r#"{{"interviewee": "{}_person", "date": "2024-01-01"}}"#,
                name
            );
            fs::write(dir.join("metadata.json"), metadata).unwrap();
        }

        // Run the simplified Phase 2 demo (without think blocks)
        let mut interp = Interpreter::with_working_dir(base.to_path_buf());
        let code = r#"{
            var items = ["interview1", "interview2"]
            for var interview in items {
                var text = read("$interview/metadata.json")
                var data = json(text)
                var output = cat(data)
                write("$interview/output.json", output)
            }
        }"#;

        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);

        // Verify output files were created
        for name in ["interview1", "interview2"] {
            let output_path = base.join(name).join("output.json");
            assert!(output_path.exists(), "Output file not created: {:?}", output_path);

            let content = fs::read_to_string(&output_path).unwrap();
            assert!(content.contains("interviewee"), "Missing interviewee in output: {}", content);
            assert!(content.contains(&format!("{}_person", name)), "Wrong person in output: {}", content);
        }
    }

    #[test]
    fn test_string_interpolation() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var name = "world"
            var msg = "Hello $name!"
            msg
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, "Hello world!");
        } else {
            panic!("Expected String(\"Hello world!\"), got {:?}", result);
        }
    }

    #[test]
    fn test_string_escape_sequences() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var msg = "Line1\nLine2\tTabbed"
            msg
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, "Line1\nLine2\tTabbed");
            assert!(s.contains('\n'), "Should contain newline");
            assert!(s.contains('\t'), "Should contain tab");
        } else {
            panic!("Expected String with escapes, got {:?}", result);
        }
    }

    #[test]
    fn test_string_escaped_quotes() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var msg = "He said \"hello\" there"
            msg
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, "He said \"hello\" there");
        } else {
            panic!("Expected String with escaped quotes, got {:?}", result);
        }
    }

    #[test]
    fn test_string_escaped_backslash() {
        let mut interp = Interpreter::new();
        // Note: trailing \\ before closing " has a lexer bug, so test mid-string
        let code = r#"{
            var msg = "path\\to\\file"
            msg
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);
        if let Ok(Value::String(s)) = result {
            assert_eq!(s, "path\\to\\file");
        } else {
            panic!("Expected String with backslashes, got {:?}", result);
        }
    }

    #[test]
    fn test_think_block_returns_placeholder() {
        let mut interp = Interpreter::new();
        let code = r#"{
            var topic = "Rust"
            think {
                Explain $topic in one sentence.
            }
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);

        // In Phase 3, think blocks return a placeholder object with the prompt
        if let Ok(Value::Object(obj)) = result {
            let prompt = obj.get("__think_prompt").expect("Missing __think_prompt");
            if let Value::String(s) = prompt {
                assert!(s.contains("Explain"));
                assert!(s.contains("Rust"));
                assert!(s.contains("in one sentence"));
            } else {
                panic!("Expected prompt string, got {:?}", prompt);
            }
        } else {
            panic!("Expected Object with __think_prompt, got {:?}", result);
        }
    }

    #[test]
    fn test_exception_propagation() {
        let mut interp = Interpreter::new();
        let code = r#"{
            throw "oops"
        }"#;
        let result = interp.eval(code);
        match result {
            Err(Error::Exception(Value::String(s))) => {
                assert_eq!(s, "oops");
            }
            other => panic!("Expected Exception, got {:?}", other),
        }
    }
}
