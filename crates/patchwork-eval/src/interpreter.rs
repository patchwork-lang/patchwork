//! The Patchwork interpreter with suspend/resume capability.

use std::collections::HashMap;
use std::path::PathBuf;

use patchwork_parser::ast::{Expr, Statement};

use crate::error::Error;
use crate::eval;
use crate::runtime::Runtime;
use crate::value::Value;

/// Variable bindings passed to an LLM operation.
pub type Bindings = HashMap<String, Value>;

/// The type of LLM operation being requested.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmOp {
    /// A `think { ... }` block - LLM processes and returns a value.
    Think,
    /// An `ask { ... }` block - interactive prompt (future).
    Ask,
}

/// The control state of the interpreter.
///
/// This enum represents the current execution state, inspired by
/// generator/coroutine semantics where execution can suspend and resume.
#[derive(Debug, Clone)]
pub enum ControlState {
    /// The interpreter is ready to evaluate or currently evaluating.
    Eval,

    /// The interpreter has suspended, waiting for an LLM response.
    Yield {
        /// The type of LLM operation.
        op: LlmOp,
        /// The interpolated prompt text to send to the LLM.
        prompt: String,
        /// Variable bindings available in the prompt context.
        bindings: Bindings,
        /// Description of the expected response type.
        expect: String,
    },

    /// The interpreter has completed successfully with a value.
    Return(Value),

    /// The interpreter has thrown an exception.
    Throw(Value),
}

impl ControlState {
    /// Check if the interpreter is in a terminal state (Return or Throw).
    pub fn is_terminal(&self) -> bool {
        matches!(self, ControlState::Return(_) | ControlState::Throw(_))
    }

    /// Check if the interpreter is yielding, waiting for LLM input.
    pub fn is_yield(&self) -> bool {
        matches!(self, ControlState::Yield { .. })
    }
}

/// The Patchwork interpreter.
///
/// Executes Patchwork code with the ability to suspend at `think` blocks,
/// allowing external systems to provide LLM responses before resuming.
#[derive(Debug, Clone)]
pub struct Interpreter {
    /// Current control state.
    state: ControlState,
    /// Runtime environment with variable bindings.
    runtime: Runtime,
}

impl Interpreter {
    /// Create a new interpreter in the Eval state.
    pub fn new() -> Self {
        Self {
            state: ControlState::Eval,
            runtime: Runtime::default(),
        }
    }

    /// Create a new interpreter with a specific working directory.
    pub fn with_working_dir(working_dir: PathBuf) -> Self {
        Self {
            state: ControlState::Eval,
            runtime: Runtime::new(working_dir),
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

    /// Get the current control state.
    pub fn state(&self) -> &ControlState {
        &self.state
    }

    /// Evaluate Patchwork code.
    ///
    /// Parses and executes the code, returning the resulting control state.
    /// If the code contains `think` blocks, the interpreter may yield,
    /// requiring a call to `resume()` with the LLM's response.
    ///
    /// For ACP usage, code starting with `{` is wrapped in a skill for execution.
    pub fn eval(&mut self, code: &str) -> crate::Result<&ControlState> {
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
                match self.execute_program(&ast) {
                    Ok(state) => {
                        self.state = state;
                        Ok(&self.state)
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        self.state = ControlState::Throw(Value::String(msg.clone()));
                        Err(e)
                    }
                }
            }
            Err(e) => {
                let msg = format!("{:?}", e);
                self.state = ControlState::Throw(Value::String(msg.clone()));
                Err(Error::Parse(msg))
            }
        }
    }

    /// Execute a parsed program.
    fn execute_program(&mut self, program: &patchwork_parser::Program) -> crate::Result<ControlState> {
        use patchwork_parser::Item;

        // Look for __main__ skill (from wrapped block) or execute items
        for item in &program.items {
            match item {
                Item::Skill(skill) if skill.name == "__main__" => {
                    // Execute the main skill's body
                    return eval::eval_block(&skill.body, &mut self.runtime);
                }
                Item::Function(func) if func.name == "__main__" => {
                    // Execute the main function's body
                    return eval::eval_block(&func.body, &mut self.runtime);
                }
                _ => {
                    // Other items (imports, type decls, etc.) - currently ignored
                    // In a full implementation, we'd register functions/skills
                }
            }
        }

        // No __main__ found, evaluate as program items
        eval::eval_program(program, &mut self.runtime)
    }

    /// Evaluate a single expression directly (for testing).
    pub fn eval_expr(&mut self, expr: &Expr) -> crate::Result<ControlState> {
        eval::eval_expr(expr, &mut self.runtime)
    }

    /// Evaluate a single statement directly (for testing).
    pub fn eval_stmt(&mut self, stmt: &Statement) -> crate::Result<ControlState> {
        eval::eval_statement(stmt, &mut self.runtime)
    }

    /// Resume execution after an LLM response.
    ///
    /// This should only be called when the interpreter is in the `Yield` state.
    /// The provided value is the LLM's response, which becomes the result of
    /// the `think` block that caused the yield.
    pub fn resume(&mut self, _value: Value) -> crate::Result<&ControlState> {
        if !self.state.is_yield() {
            return Err(Error::InvalidResume);
        }

        // Phase 1 stub - not yet implemented
        Err(Error::Runtime("resume not yet implemented".to_string()))
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_interpreter() {
        let interp = Interpreter::new();
        assert!(matches!(interp.state(), ControlState::Eval));
    }

    #[test]
    fn test_eval_empty_program() {
        let mut interp = Interpreter::new();
        // Empty program is valid Patchwork
        let result = interp.eval("");
        assert!(result.is_ok());
        assert!(matches!(interp.state(), ControlState::Return(_)));
    }

    #[test]
    fn test_eval_simple_function() {
        let mut interp = Interpreter::new();
        // A simple function definition
        let result = interp.eval("fun hello() {}");
        assert!(result.is_ok());
        assert!(matches!(interp.state(), ControlState::Return(_)));
    }

    #[test]
    fn test_resume_without_yield() {
        let mut interp = Interpreter::new();
        let result = interp.resume(Value::Null);
        assert!(matches!(result, Err(Error::InvalidResume)));
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
        if let ControlState::Return(Value::Number(n)) = interp.state() {
            assert_eq!(*n, 42.0);
        } else {
            panic!("Expected Return(Number(42)), got {:?}", interp.state());
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
        if let ControlState::Return(Value::Number(n)) = interp.state() {
            assert_eq!(*n, 6.0);
        } else {
            panic!("Expected Return(Number(6)), got {:?}", interp.state());
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
        if let ControlState::Return(Value::String(s)) = interp.state() {
            assert_eq!(s, "test");
        } else {
            panic!("Expected Return(String(\"test\")), got {:?}", interp.state());
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
        if let ControlState::Return(Value::String(s)) = interp.state() {
            assert!(s.contains("\"name\""));
            assert!(s.contains("\"hello\""));
        } else {
            panic!("Expected Return(String), got {:?}", interp.state());
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
        if let ControlState::Return(Value::Number(n)) = interp.state() {
            assert_eq!(*n, 30.0);
        } else {
            panic!("Expected Return(Number(30)), got {:?}", interp.state());
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
        if let ControlState::Return(Value::String(s)) = interp.state() {
            assert_eq!(s, "big");
        } else {
            panic!("Expected Return(String(\"big\")), got {:?}", interp.state());
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
        if let ControlState::Return(Value::String(s)) = interp.state() {
            assert_eq!(s, "Hello world!");
        } else {
            panic!("Expected Return(String(\"Hello world!\")), got {:?}", interp.state());
        }
    }

    #[test]
    fn test_think_block_yields() {
        let mut interp = Interpreter::new();
        // Note: Parser doesn't preserve whitespace perfectly in prompt blocks
        // This will be improved in later phases
        let code = r#"{
            var topic = "Rust"
            think {
                Explain $topic in one sentence.
            }
        }"#;
        let result = interp.eval(code);
        assert!(result.is_ok(), "Eval failed: {:?}", result);

        // Should yield with the interpolated prompt
        match interp.state() {
            ControlState::Yield { op, prompt, bindings, expect } => {
                assert_eq!(*op, LlmOp::Think);
                // Parser currently doesn't preserve all whitespace, so we get:
                assert!(prompt.contains("Explain"));
                assert!(prompt.contains("Rust"));
                assert!(prompt.contains("in one sentence"));
                assert_eq!(expect, "string");
                // Check that 'topic' is in bindings
                assert_eq!(bindings.get("topic"), Some(&Value::String("Rust".to_string())));
            }
            other => panic!("Expected Yield state, got {:?}", other),
        }
    }
}
