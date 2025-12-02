//! Expression and statement evaluation for the Patchwork interpreter.

use std::fs;
use std::process::Command;

use patchwork_parser::ast::{
    Block, BinOp, CommandArg, Expr, ObjectPatternField, Pattern, Program,
    RedirectOp, Statement, StringLiteral, StringPart, UnOp, PromptBlock, PromptItem,
};

use crate::error::Error;
use crate::runtime::Runtime;
use crate::value::Value;
use crate::interpreter::{ControlState, LlmOp, Bindings};

/// Result type for evaluation operations - now returns ControlState.
pub type EvalResult = Result<ControlState, Error>;

/// Macro to extract a value from a ControlState or early-return with Yield/Throw.
///
/// Similar to Rust's `try!` macro, but for control flow states instead of errors.
/// Extracts the value from `Return(v)`, or propagates `Yield`/`Throw` states up.
macro_rules! try_eval {
    ($state:expr) => {
        match $state {
            ControlState::Return(v) => v,
            other => return Ok(other),
        }
    };
}

/// Evaluate a complete program.
pub fn eval_program(program: &Program, runtime: &mut Runtime) -> Result<ControlState, Error> {
    // For now, a program is just a series of items (functions, skills, etc.)
    // In this phase, we're focused on evaluating code blocks, not top-level definitions.
    // The main entry point for ACP is typically a block expression `{ ... }`.
    //
    // If the program contains statements in a block context, evaluate those.
    // For now, return null - actual execution happens via eval_block.
    let _ = (program, runtime);
    Ok(ControlState::Return(Value::Null))
}

/// Evaluate a block of statements.
pub fn eval_block(block: &Block, runtime: &mut Runtime) -> EvalResult {
    runtime.push_scope();
    let mut result = Value::Null;

    for stmt in &block.statements {
        let state = eval_statement(stmt, runtime)?;
        // Check if we need to propagate a non-Return state (Yield, Throw)
        match state {
            ControlState::Return(v) => result = v,
            other => {
                runtime.pop_scope();
                return Ok(other);
            }
        }
    }

    runtime.pop_scope();
    Ok(ControlState::Return(result))
}

/// Evaluate a single statement.
pub fn eval_statement(stmt: &Statement, runtime: &mut Runtime) -> EvalResult {
    match stmt {
        Statement::VarDecl { pattern, init } => {
            let value = match init {
                Some(expr) => try_eval!(eval_expr(expr, runtime)?),
                None => Value::Null,
            };
            bind_pattern(pattern, value, runtime)?;
            Ok(ControlState::Return(Value::Null))
        }

        Statement::Expr(expr) => eval_expr(expr, runtime),

        Statement::If { condition, then_block, else_block } => {
            let cond_value = try_eval!(eval_expr(condition, runtime)?);

            if cond_value.to_bool() {
                eval_block(then_block, runtime)
            } else if let Some(else_blk) = else_block {
                eval_block(else_blk, runtime)
            } else {
                Ok(ControlState::Return(Value::Null))
            }
        }

        Statement::ForIn { var, iter, body } => {
            let iter_value = try_eval!(eval_expr(iter, runtime)?);

            let items = match iter_value {
                Value::Array(arr) => arr,
                Value::String(s) => {
                    // Iterate over lines
                    s.lines().map(|line| Value::String(line.to_string())).collect()
                }
                other => {
                    return Err(Error::Runtime(format!(
                        "Cannot iterate over {}", type_name(&other)
                    )));
                }
            };

            let mut result = Value::Null;
            for item in items {
                runtime.push_scope();
                runtime.define_var(var, item).map_err(Error::Runtime)?;
                let state = eval_block(body, runtime)?;
                runtime.pop_scope();

                // Propagate Yield or Throw
                match state {
                    ControlState::Return(v) => result = v,
                    other => return Ok(other),
                }
            }
            Ok(ControlState::Return(result))
        }

        Statement::While { condition, body } => {
            let mut result = Value::Null;
            loop {
                let cond_value = try_eval!(eval_expr(condition, runtime)?);

                if !cond_value.to_bool() {
                    break;
                }

                let state = eval_block(body, runtime)?;
                // Propagate Yield or Throw
                match state {
                    ControlState::Return(v) => result = v,
                    other => return Ok(other),
                }
            }
            Ok(ControlState::Return(result))
        }

        Statement::Return(expr) => {
            let value = match expr {
                Some(e) => try_eval!(eval_expr(e, runtime)?),
                None => Value::Null,
            };
            // For now, just return the value. Proper return handling
            // will need control flow tracking.
            Ok(ControlState::Return(value))
        }

        Statement::Succeed => Ok(ControlState::Return(Value::Null)),

        Statement::Break => {
            // Break handling will need control flow tracking
            Err(Error::Runtime("break outside of loop".to_string()))
        }

        Statement::TypeDecl { .. } => {
            // Type declarations are compile-time only
            Ok(ControlState::Return(Value::Null))
        }
    }
}

/// Bind a value to a pattern, defining variables in the runtime.
fn bind_pattern(pattern: &Pattern, value: Value, runtime: &mut Runtime) -> Result<(), Error> {
    match pattern {
        Pattern::Identifier { name, .. } => {
            runtime.define_var(name, value).map_err(Error::Runtime)?;
        }

        Pattern::Ignore => {
            // Do nothing - the value is discarded
        }

        Pattern::Object(fields) => {
            let obj = match value {
                Value::Object(o) => o,
                other => {
                    return Err(Error::Runtime(format!(
                        "Cannot destructure {} as object", type_name(&other)
                    )));
                }
            };
            for field in fields {
                let field_value = obj.get(field.key).cloned().unwrap_or(Value::Null);
                bind_object_pattern_field(field, field_value, runtime)?;
            }
        }

        Pattern::Array(patterns) => {
            let arr = match value {
                Value::Array(a) => a,
                other => {
                    return Err(Error::Runtime(format!(
                        "Cannot destructure {} as array", type_name(&other)
                    )));
                }
            };
            for (i, pat) in patterns.iter().enumerate() {
                let item_value = arr.get(i).cloned().unwrap_or(Value::Null);
                bind_pattern(pat, item_value, runtime)?;
            }
        }
    }
    Ok(())
}

/// Bind an object pattern field.
fn bind_object_pattern_field(
    field: &ObjectPatternField,
    value: Value,
    runtime: &mut Runtime,
) -> Result<(), Error> {
    bind_pattern(&field.pattern, value, runtime)
}

/// Evaluate an expression.
pub fn eval_expr(expr: &Expr, runtime: &mut Runtime) -> EvalResult {
    match expr {
        Expr::Identifier(name) => {
            let value = runtime.get_var(name)
                .cloned()
                .ok_or_else(|| Error::Runtime(format!("Undefined variable: {}", name)))?;
            Ok(ControlState::Return(value))
        }

        Expr::Number(s) => {
            let n: f64 = s.parse()
                .map_err(|_| Error::Runtime(format!("Invalid number: {}", s)))?;
            Ok(ControlState::Return(Value::Number(n)))
        }

        Expr::String(string_lit) => eval_string_literal(string_lit, runtime),

        Expr::True => Ok(ControlState::Return(Value::Boolean(true))),
        Expr::False => Ok(ControlState::Return(Value::Boolean(false))),

        Expr::Array(items) => {
            let mut values = Vec::new();
            for item in items {
                let state = eval_expr(item, runtime)?;
                let value = try_eval!(state);
                values.push(value);
            }
            Ok(ControlState::Return(Value::Array(values)))
        }

        Expr::Object(fields) => {
            let mut map = std::collections::HashMap::new();
            for field in fields {
                let value = match &field.value {
                    Some(expr) => {
                        let state = eval_expr(expr, runtime)?;
                        try_eval!(state)
                    }
                    None => {
                        // Shorthand: {x} means {x: x}
                        runtime.get_var(field.key)
                            .cloned()
                            .ok_or_else(|| Error::Runtime(format!("Undefined variable: {}", field.key)))?
                    }
                };
                map.insert(field.key.to_string(), value);
            }
            Ok(ControlState::Return(Value::Object(map)))
        }

        Expr::Binary { op, left, right } => eval_binary(op, left, right, runtime),

        Expr::Unary { op, operand } => eval_unary(op, operand, runtime),

        Expr::Call { callee, args } => eval_call(callee, args, runtime),

        Expr::Member { object, field } => {
            let obj_state = eval_expr(object, runtime)?;
            let obj_value = try_eval!(obj_state);

            match obj_value {
                Value::Object(map) => {
                    Ok(ControlState::Return(map.get(*field).cloned().unwrap_or(Value::Null)))
                }
                other => Err(Error::Runtime(format!(
                    "Cannot access field '{}' on {}", field, type_name(&other)
                )))
            }
        }

        Expr::Index { object, index } => {
            let obj_state = eval_expr(object, runtime)?;
            let obj_value = try_eval!(obj_state);

            let idx_state = eval_expr(index, runtime)?;
            let idx_value = try_eval!(idx_state);

            match (obj_value, idx_value) {
                (Value::Array(arr), Value::Number(n)) => {
                    let i = n as usize;
                    Ok(ControlState::Return(arr.get(i).cloned().unwrap_or(Value::Null)))
                }
                (Value::Object(map), Value::String(key)) => {
                    Ok(ControlState::Return(map.get(&key).cloned().unwrap_or(Value::Null)))
                }
                (obj, idx) => Err(Error::Runtime(format!(
                    "Cannot index {} with {}", type_name(&obj), type_name(&idx)
                )))
            }
        }

        Expr::PostIncrement(operand) | Expr::PostDecrement(operand) => {
            // For now, simplified - just evaluate and return
            eval_expr(operand, runtime)
        }

        Expr::Paren(inner) => eval_expr(inner, runtime),

        Expr::Await(inner) => {
            // In synchronous evaluation, await is a no-op
            eval_expr(inner, runtime)
        }

        Expr::Think(prompt_block) => eval_think_block(prompt_block, LlmOp::Think, runtime),

        Expr::Ask(prompt_block) => eval_think_block(prompt_block, LlmOp::Ask, runtime),

        Expr::Do(block) => eval_block(block, runtime),

        Expr::BareCommand { name, args } => eval_bare_command(name, args, runtime),

        Expr::CommandSubst(inner) => {
            // Execute inner expression as command, return stdout
            let state = eval_expr(inner, runtime)?;
            let result = try_eval!(state);

            match result {
                Value::String(s) => Ok(ControlState::Return(Value::String(s.trim_end_matches('\n').to_string()))),
                other => Ok(ControlState::Return(other)),
            }
        }

        Expr::ShellPipe { left, right } => {
            // For now, simplified pipe - just execute right with left's output
            // A proper implementation would use actual pipes
            let _left_state = eval_expr(left, runtime)?;
            eval_expr(right, runtime)
        }

        Expr::ShellAnd { left, right } => {
            let left_state = eval_expr(left, runtime)?;
            let left_result = try_eval!(left_state);

            if left_result.to_bool() {
                eval_expr(right, runtime)
            } else {
                Ok(ControlState::Return(left_result))
            }
        }

        Expr::ShellOr { left, right } => {
            let left_state = eval_expr(left, runtime)?;
            let left_result = try_eval!(left_state);

            if left_result.to_bool() {
                Ok(ControlState::Return(left_result))
            } else {
                eval_expr(right, runtime)
            }
        }

        Expr::ShellRedirect { command, op, target } => {
            eval_shell_redirect(command, op, target, runtime)
        }
    }
}

/// Evaluate a string literal with interpolation.
fn eval_string_literal(lit: &StringLiteral, runtime: &mut Runtime) -> EvalResult {
    let mut result = String::new();
    for part in &lit.parts {
        match part {
            StringPart::Text(s) => result.push_str(s),
            StringPart::Interpolation(expr) => {
                let state = eval_expr(expr, runtime)?;
                let value = try_eval!(state);
                result.push_str(&value.to_string_value());
            }
        }
    }
    Ok(ControlState::Return(Value::String(result)))
}

/// Evaluate a think or ask block, yielding control with the interpolated prompt.
fn eval_think_block(prompt_block: &PromptBlock, op: LlmOp, runtime: &mut Runtime) -> EvalResult {
    // Interpolate the prompt, gathering variable bindings
    let mut prompt_text = String::new();
    let mut bindings = Bindings::new();

    for item in &prompt_block.items {
        match item {
            PromptItem::Text(text) => {
                prompt_text.push_str(text);
            }
            PromptItem::Interpolation(expr) => {
                let state = eval_expr(expr, runtime)?;
                let value = try_eval!(state);

                // Add to prompt text
                prompt_text.push_str(&value.to_string_value());

                // Track variable if it's a simple identifier
                if let Expr::Identifier(name) = expr {
                    bindings.insert(name.to_string(), value);
                }
            }
            PromptItem::Code(block) => {
                // Embedded code blocks - execute them
                let state = eval_block(block, runtime)?;
                // If the code yields or throws, propagate it
                match state {
                    ControlState::Return(_) => {
                        // Code block executed successfully, continue
                    }
                    other => return Ok(other),
                }
            }
        }
    }

    // For now, expect string responses
    let expect = "string".to_string();

    // Return Yield state
    Ok(ControlState::Yield {
        op,
        prompt: prompt_text,
        bindings,
        expect,
    })
}

/// Evaluate a binary operation.
fn eval_binary(op: &BinOp, left: &Expr, right: &Expr, runtime: &mut Runtime) -> EvalResult {
    // Handle assignment specially
    if let BinOp::Assign = op {
        let state = eval_expr(right, runtime)?;
        let value = try_eval!(state);

        match left {
            Expr::Identifier(name) => {
                runtime.set_var(name, value.clone()).map_err(Error::Runtime)?;
                return Ok(ControlState::Return(value));
            }
            _ => return Err(Error::Runtime("Invalid assignment target".to_string())),
        }
    }

    let left_state = eval_expr(left, runtime)?;
    let left_val = try_eval!(left_state);

    let right_state = eval_expr(right, runtime)?;
    let right_val = try_eval!(right_state);

    let result = match op {
        BinOp::Add => {
            match (&left_val, &right_val) {
                (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
                (Value::String(a), Value::String(b)) => Value::String(format!("{}{}", a, b)),
                (Value::String(a), b) => Value::String(format!("{}{}", a, b.to_string_value())),
                (a, Value::String(b)) => Value::String(format!("{}{}", a.to_string_value(), b)),
                _ => {
                    return Err(Error::Runtime(format!(
                        "Cannot add {} and {}", type_name(&left_val), type_name(&right_val)
                    )))
                }
            }
        }
        BinOp::Sub => num_op(&left_val, &right_val, |a, b| a - b)?,
        BinOp::Mul => num_op(&left_val, &right_val, |a, b| a * b)?,
        BinOp::Div => num_op(&left_val, &right_val, |a, b| a / b)?,
        BinOp::Eq => Value::Boolean(values_equal(&left_val, &right_val)),
        BinOp::NotEq => Value::Boolean(!values_equal(&left_val, &right_val)),
        BinOp::Lt => compare_values(&left_val, &right_val, |ord| ord.is_lt())?,
        BinOp::Gt => compare_values(&left_val, &right_val, |ord| ord.is_gt())?,
        BinOp::And => Value::Boolean(left_val.to_bool() && right_val.to_bool()),
        BinOp::Or => Value::Boolean(left_val.to_bool() || right_val.to_bool()),
        BinOp::Pipe => {
            // Should be handled as ShellPipe, not BinOp::Pipe
            return Err(Error::Runtime("Pipe operator not supported here".to_string()))
        }
        BinOp::Range => {
            // Create a range array
            match (&left_val, &right_val) {
                (Value::Number(start), Value::Number(end)) => {
                    let start = *start as i64;
                    let end = *end as i64;
                    let range: Vec<Value> = (start..=end)
                        .map(|n| Value::Number(n as f64))
                        .collect();
                    Value::Array(range)
                }
                _ => return Err(Error::Runtime("Range requires numbers".to_string())),
            }
        }
        BinOp::Assign => unreachable!("handled above"),
    };

    Ok(ControlState::Return(result))
}

/// Numeric binary operation helper.
fn num_op(left: &Value, right: &Value, op: fn(f64, f64) -> f64) -> Result<Value, Error> {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(op(*a, *b))),
        _ => Err(Error::Runtime(format!(
            "Cannot perform numeric operation on {} and {}",
            type_name(left), type_name(right)
        ))),
    }
}

/// Check if two values are equal.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
        }
        _ => false,
    }
}

/// Compare two values.
fn compare_values(a: &Value, b: &Value, pred: fn(std::cmp::Ordering) -> bool) -> Result<Value, Error> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            Ok(Value::Boolean(pred(a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))))
        }
        (Value::String(a), Value::String(b)) => {
            Ok(Value::Boolean(pred(a.cmp(b))))
        }
        _ => Err(Error::Runtime(format!(
            "Cannot compare {} and {}", type_name(a), type_name(b)
        ))),
    }
}

/// Evaluate a unary operation.
fn eval_unary(op: &UnOp, operand: &Expr, runtime: &mut Runtime) -> EvalResult {
    let state = eval_expr(operand, runtime)?;
    let value = try_eval!(state);

    match op {
        UnOp::Not => Ok(ControlState::Return(Value::Boolean(!value.to_bool()))),
        UnOp::Neg => {
            match value {
                Value::Number(n) => Ok(ControlState::Return(Value::Number(-n))),
                _ => Err(Error::Runtime(format!("Cannot negate {}", type_name(&value)))),
            }
        }
        UnOp::Throw => Err(Error::Runtime(value.to_string_value())),
    }
}

/// Evaluate a function call.
fn eval_call(callee: &Expr, args: &[Expr], runtime: &mut Runtime) -> EvalResult {
    // Check for builtin functions
    if let Expr::Identifier(name) = callee {
        let mut arg_values = Vec::new();
        for arg in args {
            let state = eval_expr(arg, runtime)?;
            let value = try_eval!(state);
            arg_values.push(value);
        }

        return eval_builtin(name, &arg_values, runtime);
    }

    // For now, only builtins are supported
    Err(Error::Runtime("User-defined functions not yet implemented".to_string()))
}

/// Evaluate a builtin function call.
fn eval_builtin(name: &str, args: &[Value], runtime: &Runtime) -> EvalResult {
    let result = match name {
        "cat" => {
            // cat(value) - serialize to pretty JSON
            if args.len() != 1 {
                return Err(Error::Runtime("cat() takes exactly 1 argument".to_string()));
            }
            Value::String(args[0].to_json())
        }

        "json" => {
            // json(text) - parse JSON string
            if args.len() != 1 {
                return Err(Error::Runtime("json() takes exactly 1 argument".to_string()));
            }
            let text = args[0].to_string_value();
            Value::from_json(&text).map_err(Error::Runtime)?
        }

        "print" => {
            // print(values...) - print to stdout
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", arg.to_string_value());
            }
            println!();
            Value::Null
        }

        "len" => {
            if args.len() != 1 {
                return Err(Error::Runtime("len() takes exactly 1 argument".to_string()));
            }
            match &args[0] {
                Value::Array(arr) => Value::Number(arr.len() as f64),
                Value::String(s) => Value::Number(s.len() as f64),
                Value::Object(obj) => Value::Number(obj.len() as f64),
                other => return Err(Error::Runtime(format!("Cannot get length of {}", type_name(other)))),
            }
        }

        "keys" => {
            if args.len() != 1 {
                return Err(Error::Runtime("keys() takes exactly 1 argument".to_string()));
            }
            match &args[0] {
                Value::Object(obj) => {
                    let keys: Vec<Value> = obj.keys()
                        .map(|k| Value::String(k.clone()))
                        .collect();
                    Value::Array(keys)
                }
                other => return Err(Error::Runtime(format!("Cannot get keys of {}", type_name(other)))),
            }
        }

        "values" => {
            if args.len() != 1 {
                return Err(Error::Runtime("values() takes exactly 1 argument".to_string()));
            }
            match &args[0] {
                Value::Object(obj) => {
                    let values: Vec<Value> = obj.values().cloned().collect();
                    Value::Array(values)
                }
                other => return Err(Error::Runtime(format!("Cannot get values of {}", type_name(other)))),
            }
        }

        "typeof" => {
            if args.len() != 1 {
                return Err(Error::Runtime("typeof() takes exactly 1 argument".to_string()));
            }
            Value::String(type_name(&args[0]).to_string())
        }

        "read" => {
            // read(path) - read file contents as string
            if args.len() != 1 {
                return Err(Error::Runtime("read() takes exactly 1 argument".to_string()));
            }
            let path = resolve_path(&args[0].to_string_value(), runtime);
            let contents = fs::read_to_string(&path)
                .map_err(|e| Error::Runtime(format!("Failed to read {}: {}", path.display(), e)))?;
            Value::String(contents)
        }

        "write" => {
            // write(path, content) - write string to file
            if args.len() != 2 {
                return Err(Error::Runtime("write() takes exactly 2 arguments".to_string()));
            }
            let path = resolve_path(&args[0].to_string_value(), runtime);
            let content = args[1].to_string_value();
            fs::write(&path, content)
                .map_err(|e| Error::Runtime(format!("Failed to write {}: {}", path.display(), e)))?;
            Value::Null
        }

        _ => return Err(Error::Runtime(format!("Unknown function: {}", name))),
    };

    Ok(ControlState::Return(result))
}

/// Evaluate a bare shell command.
fn eval_bare_command(name: &str, args: &[CommandArg], runtime: &mut Runtime) -> EvalResult {
    let mut cmd_args = Vec::new();
    for arg in args {
        match arg {
            CommandArg::Literal(s) => cmd_args.push(s.to_string()),
            CommandArg::String(string_lit) => {
                let state = eval_string_literal(string_lit, runtime)?;
                let value = try_eval!(state);
                cmd_args.push(value.to_string_value());
            }
        }
    }

    exec_command(name, &cmd_args, runtime)
}

/// Execute a shell command.
fn exec_command(name: &str, args: &[String], runtime: &Runtime) -> EvalResult {
    let output = Command::new(name)
        .args(args)
        .current_dir(runtime.working_dir())
        .output()
        .map_err(|e| Error::Runtime(format!("Failed to execute {}: {}", name, e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Runtime(format!(
            "Command '{}' failed with exit code {:?}: {}",
            name,
            output.status.code(),
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // For `ls -1` style commands, return as array of lines
    if name == "ls" && args.iter().any(|a| a.contains('1')) {
        let lines: Vec<Value> = stdout
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| Value::String(l.to_string()))
            .collect();
        return Ok(ControlState::Return(Value::Array(lines)));
    }

    Ok(ControlState::Return(Value::String(stdout.into_owned())))
}

/// Evaluate a shell redirect expression.
fn eval_shell_redirect(
    command: &Expr,
    op: &RedirectOp,
    target: &Expr,
    runtime: &mut Runtime,
) -> EvalResult {
    match op {
        RedirectOp::In => {
            // Read from file and use as input
            // For `json < "file.json"`, we read the file and parse as JSON
            let target_state = eval_expr(target, runtime)?;
            let target_value = try_eval!(target_state);
            let path = resolve_path(&target_value.to_string_value(), runtime);
            let contents = fs::read_to_string(&path)
                .map_err(|e| Error::Runtime(format!("Failed to read {}: {}", path.display(), e)))?;

            // Check if the command is 'json' for JSON parsing
            if let Expr::Identifier("json") = command {
                let value = Value::from_json(&contents).map_err(Error::Runtime)?;
                return Ok(ControlState::Return(value));
            }

            // Otherwise, just return the file contents
            Ok(ControlState::Return(Value::String(contents)))
        }

        RedirectOp::Out => {
            // Write command output to file
            let cmd_state = eval_expr(command, runtime)?;
            let cmd_result = try_eval!(cmd_state);

            let target_state = eval_expr(target, runtime)?;
            let target_value = try_eval!(target_state);
            let path = resolve_path(&target_value.to_string_value(), runtime);

            // If the command was cat(), write as JSON
            let content = if let Expr::Call { callee, .. } = command {
                if let Expr::Identifier("cat") = callee.as_ref() {
                    cmd_result.to_string_value()
                } else {
                    cmd_result.to_string_value()
                }
            } else {
                cmd_result.to_string_value()
            };

            fs::write(&path, content)
                .map_err(|e| Error::Runtime(format!("Failed to write {}: {}", path.display(), e)))?;

            Ok(ControlState::Return(Value::Null))
        }

        RedirectOp::Append => {
            // Append command output to file
            let cmd_state = eval_expr(command, runtime)?;
            let cmd_result = try_eval!(cmd_state);

            let target_state = eval_expr(target, runtime)?;
            let target_value = try_eval!(target_state);
            let path = resolve_path(&target_value.to_string_value(), runtime);

            let existing = fs::read_to_string(&path).unwrap_or_default();
            let content = format!("{}{}", existing, cmd_result.to_string_value());

            fs::write(&path, content)
                .map_err(|e| Error::Runtime(format!("Failed to write {}: {}", path.display(), e)))?;

            Ok(ControlState::Return(Value::Null))
        }

        RedirectOp::ErrOut | RedirectOp::ErrToOut => {
            // Stderr redirections - for now just execute and ignore stderr
            eval_expr(command, runtime)
        }
    }
}

/// Resolve a path relative to the runtime's working directory.
fn resolve_path(path: &str, runtime: &Runtime) -> std::path::PathBuf {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        runtime.working_dir().join(p)
    }
}

/// Get the type name of a value for error messages.
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Boolean(_) => "boolean",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_runtime() -> Runtime {
        Runtime::default()
    }

    #[test]
    fn test_eval_number() {
        let mut rt = make_runtime();
        let expr = Expr::Number("42");
        let state = eval_expr(&expr, &mut rt).unwrap();
        assert!(matches!(state, ControlState::Return(Value::Number(n)) if n == 42.0));
    }

    #[test]
    fn test_eval_string() {
        let mut rt = make_runtime();
        let expr = Expr::String(StringLiteral {
            parts: vec![StringPart::Text("hello")],
        });
        let state = eval_expr(&expr, &mut rt).unwrap();
        assert!(matches!(state, ControlState::Return(Value::String(s)) if s == "hello"));
    }

    #[test]
    fn test_eval_boolean() {
        let mut rt = make_runtime();
        let state = eval_expr(&Expr::True, &mut rt).unwrap();
        assert!(matches!(state, ControlState::Return(Value::Boolean(true))));

        let state = eval_expr(&Expr::False, &mut rt).unwrap();
        assert!(matches!(state, ControlState::Return(Value::Boolean(false))));
    }

    #[test]
    fn test_eval_array() {
        let mut rt = make_runtime();
        let expr = Expr::Array(vec![
            Expr::Number("1"),
            Expr::Number("2"),
            Expr::Number("3"),
        ]);
        let state = eval_expr(&expr, &mut rt).unwrap();
        if let ControlState::Return(Value::Array(arr)) = state {
            assert_eq!(arr, vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
            ]);
        } else {
            panic!("Expected Return(Array)");
        }
    }

    #[test]
    fn test_eval_add() {
        let mut rt = make_runtime();
        let expr = Expr::Binary {
            op: BinOp::Add,
            left: Box::new(Expr::Number("1")),
            right: Box::new(Expr::Number("2")),
        };
        let state = eval_expr(&expr, &mut rt).unwrap();
        assert!(matches!(state, ControlState::Return(Value::Number(n)) if n == 3.0));
    }

    #[test]
    fn test_eval_string_concat() {
        let mut rt = make_runtime();
        let expr = Expr::Binary {
            op: BinOp::Add,
            left: Box::new(Expr::String(StringLiteral {
                parts: vec![StringPart::Text("hello ")],
            })),
            right: Box::new(Expr::String(StringLiteral {
                parts: vec![StringPart::Text("world")],
            })),
        };
        let state = eval_expr(&expr, &mut rt).unwrap();
        assert!(matches!(state, ControlState::Return(Value::String(s)) if s == "hello world"));
    }

    #[test]
    fn test_eval_builtin_cat() {
        let rt = Runtime::default();
        let value = Value::Object(
            [("name".to_string(), Value::String("test".to_string()))]
                .into_iter()
                .collect(),
        );
        let state = eval_builtin("cat", &[value], &rt).unwrap();
        if let ControlState::Return(Value::String(s)) = state {
            assert!(s.contains("\"name\""));
            assert!(s.contains("\"test\""));
        } else {
            panic!("Expected Return(String)");
        }
    }

    #[test]
    fn test_eval_builtin_json() {
        let rt = Runtime::default();
        let state = eval_builtin("json", &[Value::String(r#"{"x": 1}"#.to_string())], &rt).unwrap();
        if let ControlState::Return(Value::Object(obj)) = state {
            assert_eq!(obj.get("x"), Some(&Value::Number(1.0)));
        } else {
            panic!("Expected Return(Object)");
        }
    }
}
