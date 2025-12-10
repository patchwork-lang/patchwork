//! Runtime environment for the Patchwork interpreter.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::value::Value;

/// A sink for print output, allowing redirection away from stdout.
pub type PrintSink = Sender<String>;

/// Status of a plan entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanEntryStatus {
    Pending,
    InProgress,
    Completed,
}

/// A single entry in the execution plan.
#[derive(Debug, Clone)]
pub struct PlanEntry {
    /// Human-readable description of this task.
    pub content: String,
    /// Current execution status.
    pub status: PlanEntryStatus,
}

/// A plan update message sent from the interpreter.
#[derive(Debug, Clone)]
pub struct PlanUpdate {
    /// The complete list of plan entries (client replaces entire plan).
    pub entries: Vec<PlanEntry>,
}

/// A sink for plan updates, allowing the ACP proxy to receive execution progress.
pub type PlanReporter = Sender<PlanUpdate>;

/// The runtime environment for executing Patchwork code.
///
/// Holds variable bindings and execution context like the working directory.
#[derive(Debug)]
pub struct Runtime {
    /// Variable bindings, organized as a stack of scopes.
    /// Inner vec is the most recent scope, outer vec contains parent scopes.
    scopes: Vec<HashMap<String, Value>>,
    /// Current working directory for file operations and shell commands.
    working_dir: PathBuf,
    /// Optional sink for print output. If None, prints go to stdout.
    print_sink: Option<PrintSink>,
    /// Optional sink for plan updates. If None, no plan reporting.
    plan_reporter: Option<PlanReporter>,
}

impl Runtime {
    /// Create a new runtime with the given working directory.
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            working_dir,
            print_sink: None,
            plan_reporter: None,
        }
    }

    /// Create a new runtime with a print sink for output redirection.
    pub fn with_print_sink(working_dir: PathBuf, print_sink: PrintSink) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            working_dir,
            print_sink: Some(print_sink),
            plan_reporter: None,
        }
    }

    /// Set the print sink for output redirection.
    pub fn set_print_sink(&mut self, sink: PrintSink) {
        self.print_sink = Some(sink);
    }

    /// Set the plan reporter for execution progress updates.
    pub fn set_plan_reporter(&mut self, reporter: PlanReporter) {
        self.plan_reporter = Some(reporter);
    }

    /// Send a print message to the sink, or stdout if no sink is configured.
    ///
    /// Returns Ok(()) on success, or Err if the channel is disconnected.
    pub fn print(&self, message: String) -> Result<(), String> {
        if let Some(ref sink) = self.print_sink {
            sink.send(message).map_err(|e| format!("Print channel disconnected: {}", e))
        } else {
            println!("{}", message);
            Ok(())
        }
    }

    /// Send a plan update to the reporter, if configured.
    ///
    /// Silently does nothing if no reporter is configured.
    pub fn report_plan(&self, update: PlanUpdate) {
        if let Some(ref reporter) = self.plan_reporter {
            // Ignore errors - if the channel is disconnected, we just don't report
            let _ = reporter.send(update);
        }
    }

    /// Get the current working directory.
    pub fn working_dir(&self) -> &PathBuf {
        &self.working_dir
    }

    /// Set the current working directory.
    pub fn set_working_dir(&mut self, dir: PathBuf) {
        self.working_dir = dir;
    }

    /// Push a new scope onto the scope stack (entering a block).
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope from the stack (leaving a block).
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Define a new variable in the current scope.
    ///
    /// Returns an error if the variable already exists in the current scope.
    pub fn define_var(&mut self, name: &str, value: Value) -> Result<(), String> {
        let current_scope = self.scopes.last_mut()
            .expect("scope stack should never be empty");

        if current_scope.contains_key(name) {
            return Err(format!("Variable '{}' already defined in this scope", name));
        }

        current_scope.insert(name.to_string(), value);
        Ok(())
    }

    /// Get the value of a variable, searching from innermost to outermost scope.
    pub fn get_var(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }
        None
    }

    /// Set the value of an existing variable.
    ///
    /// Searches from innermost to outermost scope for the variable.
    /// Returns an error if the variable doesn't exist.
    pub fn set_var(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        Err(format!("Variable '{}' not defined", name))
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            print_sink: None,
            plan_reporter: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_and_get_var() {
        let mut rt = Runtime::default();
        rt.define_var("x", Value::Number(42.0)).unwrap();
        assert_eq!(rt.get_var("x"), Some(&Value::Number(42.0)));
    }

    #[test]
    fn test_undefined_var() {
        let rt = Runtime::default();
        assert_eq!(rt.get_var("x"), None);
    }

    #[test]
    fn test_set_var() {
        let mut rt = Runtime::default();
        rt.define_var("x", Value::Number(1.0)).unwrap();
        rt.set_var("x", Value::Number(2.0)).unwrap();
        assert_eq!(rt.get_var("x"), Some(&Value::Number(2.0)));
    }

    #[test]
    fn test_set_undefined_var_fails() {
        let mut rt = Runtime::default();
        let result = rt.set_var("x", Value::Number(1.0));
        assert!(result.is_err());
    }

    #[test]
    fn test_scope_shadowing() {
        let mut rt = Runtime::default();
        rt.define_var("x", Value::Number(1.0)).unwrap();

        rt.push_scope();
        rt.define_var("x", Value::Number(2.0)).unwrap();
        assert_eq!(rt.get_var("x"), Some(&Value::Number(2.0)));

        rt.pop_scope();
        assert_eq!(rt.get_var("x"), Some(&Value::Number(1.0)));
    }

    #[test]
    fn test_inner_scope_sees_outer() {
        let mut rt = Runtime::default();
        rt.define_var("x", Value::Number(1.0)).unwrap();

        rt.push_scope();
        assert_eq!(rt.get_var("x"), Some(&Value::Number(1.0)));
    }
}
