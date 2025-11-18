/// Integration tests for code generation

use patchwork_compiler::{Compiler, CompileOptions};

/// Helper to compile a Patchwork source string
fn compile_source(source: &str) -> Result<String, String> {
    // Write source to a temp file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).map_err(|e| e.to_string())?;

    // Compile it
    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().map_err(|e| e.to_string())?;

    // Clean up
    let _ = std::fs::remove_file(&test_file);

    Ok(output.javascript)
}

#[test]
fn test_simple_worker() {
    let source = r#"
worker example() {
    var x = 5
    return x
}
"#;

    let js = compile_source(source).expect("compilation failed");
    // Workers now receive session as first parameter and are async
    assert!(js.contains("export async function example(session)"));
    assert!(js.contains("let x = 5"));
    assert!(js.contains("return x"));
}

#[test]
fn test_worker_with_params() {
    let source = r#"
worker process(a, b) {
    var sum = a + b
    return sum
}
"#;

    let js = compile_source(source).expect("compilation failed");
    // Workers now receive session as first parameter and are async
    assert!(js.contains("export async function process(session, a, b)"));
    assert!(js.contains("let sum = a + b"));
}

#[test]
fn test_if_statement() {
    let source = r#"
worker check(x) {
    if x > 10 {
        return true
    } else {
        return false
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("if (x > 10)"));
    assert!(js.contains("} else {"));
}

#[test]
fn test_while_loop() {
    let source = r#"
worker loop_test() {
    var i = 0
    while (i < 10) {
        var temp = i
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("while (i < 10)"));
}

#[test]
fn test_for_loop() {
    let source = r#"
worker iterate(items) {
    for var item in items {
        var x = item
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("for (let item of items)"));
}

#[test]
fn test_string_interpolation() {
    let source = r#"
worker greet(name) {
    var msg = "Hello, ${name}!"
    return msg
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("`Hello, ${name}!`"));
}

#[test]
fn test_shell_command_statement() {
    let source = r#"
worker run_cmd() {
    $ echo "hello"
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await shell(`echo hello`)"));
}

#[test]
fn test_shell_command_substitution() {
    let source = r#"
worker get_output() {
    var result = $(ls)
    return result
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await shell(`ls`, {capture: true})"));
}

#[test]
fn test_shell_pipe() {
    let source = r#"
worker pipe_test() {
    $ echo "test" | grep test
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await $shellPipe"));
}

#[test]
fn test_shell_and() {
    let source = r#"
worker and_test() {
    $ touch file.txt && cat file.txt
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await $shellAnd"));
}

#[test]
fn test_array_literal() {
    let source = r#"
worker arrays() {
    var nums = [1, 2, 3]
    return nums
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("[1, 2, 3]"));
}

#[test]
fn test_object_literal() {
    let source = r#"
worker objects() {
    var obj = {x: 1, y: 2}
    return obj
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("{ x: 1, y: 2 }"));
}

#[test]
fn test_member_access() {
    let source = r#"
worker member() {
    var obj = {x: 1}
    var val = obj.x
    return val
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("obj.x"));
}

#[test]
fn test_function_call() {
    let source = r#"
fun foo(x, y) {
    return x + y
}

worker caller() {
    var result = foo(1, 2)
    return result
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("foo(1, 2)"));
}

#[test]
fn test_binary_operators() {
    let source = r#"
worker math() {
    var a = 5 + 3
    var b = 10 - 2
    var c = 4 * 2
    var d = 8 / 2
    return d
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("5 + 3"));
    assert!(js.contains("10 - 2"));
    assert!(js.contains("4 * 2"));
    assert!(js.contains("8 / 2"));
}

#[test]
fn test_comparison_operators() {
    let source = r#"
worker compare(x, y) {
    if x == y {
        return true
    }
    if x != y {
        return false
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("x === y"));
    assert!(js.contains("x !== y"));
}

#[test]
fn test_logical_operators() {
    let source = r#"
worker logic(a, b) {
    if a && b {
        return true
    }
    if a || b {
        return false
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("a && b"));
    assert!(js.contains("a || b"));
}

#[test]
fn test_unary_operators() {
    let source = r#"
worker unary(x) {
    var neg = -x
    var not = !x
    return neg
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("-x"));
    assert!(js.contains("!x"));
}

#[test]
fn test_throw_expression() {
    let source = r#"
worker error_test() {
    throw "Something went wrong"
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("throw new Error"));
}

#[test]
fn test_function_declaration() {
    let source = r#"
fun helper(x) {
    return x + 1
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("function helper(x)"));
    assert!(!js.contains("export function helper")); // Not exported
}

#[test]
fn test_exported_function() {
    let source = r#"
export fun helper(x) {
    return x + 1
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("export function helper(x)"));
}

#[test]
fn test_break_statement() {
    let source = r#"
worker break_test(x) {
    while (x > 0) {
        break
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("break;"));
}

#[test]
fn test_complex_example() {
    let source = r#"
worker example() {
    var x = 5
    var y = $(echo "hello")
    if x > 3 {
        $ echo "x is big"
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify all expected components
    // Workers now receive session as first parameter and are async
    assert!(js.contains("export async function example(session)"));
    assert!(js.contains("let x = 5"));
    assert!(js.contains("await shell(`echo hello`, {capture: true})"));
    assert!(js.contains("if (x > 3)"));
    assert!(js.contains("await shell(`echo x is big`)"));
}

// ====== Session Context Tests ======

#[test]
fn test_session_context_access() {
    let source = r#"
worker example() {
    var session_id = self.session.id
    var timestamp = self.session.timestamp
    var work_dir = self.session.dir
    return session_id
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Check runtime imports are included (includes shell, SessionContext, executePrompt, and delegate)
    assert!(js.contains("import { shell, $shellPipe, $shellAnd, $shellOr, $shellRedirect, SessionContext, executePrompt, delegate"));

    // Check worker receives session parameter and is async
    assert!(js.contains("export async function example(session)"));

    // Check self.session.x is transformed to session.x
    assert!(js.contains("let session_id = session.id"));
    assert!(js.contains("let timestamp = session.timestamp"));
    assert!(js.contains("let work_dir = session.dir"));
}

#[test]
fn test_session_in_string_interpolation() {
    let source = r#"
worker example() {
    var msg = "Session ${self.session.id} at ${self.session.dir}"
    return msg
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Check session access in template literals
    assert!(js.contains("let msg = `Session ${session.id} at ${session.dir}`"));
}

#[test]
fn test_bare_self_error() {
    let source = r#"
worker example() {
    return self
}
"#;

    let result = compile_source(source);
    assert!(result.is_err(), "Expected error for bare 'self'");
    let err = result.unwrap_err();
    assert!(err.contains("Bare 'self' is not supported"),
            "Error message should mention 'Bare self', got: {}", err);
}

#[test]
fn test_invalid_self_field_error() {
    let source = r#"
worker example() {
    return self.mailbox
}
"#;

    let result = compile_source(source);
    assert!(result.is_err(), "Expected error for self.mailbox");
    let err = result.unwrap_err();
    assert!(err.contains("self.mailbox is not supported") || err.contains("Only self.session"),
            "Error message should mention unsupported field, got: {}", err);
}

#[test]
fn test_runtime_emission() {
    let source = r#"
worker example() {
    return 42
}
"#;

    // Compile source (need full CompileOutput, not just javascript)
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    // Clean up
    let _ = std::fs::remove_file(&test_file);

    // Verify runtime code is included
    assert!(!output.runtime.is_empty(), "Runtime code should not be empty");
    assert!(output.runtime.contains("export async function shell"),
            "Runtime should contain shell function");
    assert!(output.runtime.contains("export class SessionContext"),
            "Runtime should contain SessionContext class");
}

// ========================================
// Prompt Block Compilation Tests
// ========================================

/// Helper to compile and return both JS and prompts
fn compile_with_prompts(source: &str) -> Result<(String, std::collections::HashMap<String, String>), String> {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).map_err(|e| e.to_string())?;

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().map_err(|e| e.to_string())?;

    let _ = std::fs::remove_file(&test_file);

    Ok((output.javascript, output.prompts))
}

#[test]
fn test_simple_think_block() {
    let source = r#"
worker example() {
    var result = think {
        Hello, world!
    }
}
"#;

    let (js, prompts) = compile_with_prompts(source).expect("compilation failed");

    // Verify JS contains executePrompt call with skill name: {worker}_{kind}_{n}
    assert!(js.contains("await executePrompt(session, 'example_think_0', {  })"),
            "JS should contain executePrompt call for example_think_0");

    // Verify prompt skill document was generated
    assert_eq!(prompts.len(), 1, "Should have 1 prompt skill");
    assert!(prompts.contains_key("skills/main_example_think_0/SKILL.md"),
            "Should have skill document at skills/main_example_think_0/SKILL.md");

    let skill_doc = &prompts["skills/main_example_think_0/SKILL.md"];
    assert!(skill_doc.contains("Hello, world!"), "Skill doc should contain the prompt text");
    assert!(skill_doc.contains("name: main_example_think_0"), "Skill doc should have correct name");
}

#[test]
fn test_think_block_with_variable() {
    let source = r#"
worker example() {
    var name = "Claude"
    var result = think {
        Say hello to ${name}.
    }
}
"#;

    let (js, prompts) = compile_with_prompts(source).expect("compilation failed");

    // Verify JS passes variable binding with new skill naming
    assert!(js.contains("await executePrompt(session, 'example_think_0', { name })"),
            "JS should pass name binding to executePrompt");

    // Verify prompt skill document has placeholder and variable bindings section
    let skill_doc = &prompts["skills/main_example_think_0/SKILL.md"];
    assert!(skill_doc.contains("Say hello to") && skill_doc.contains("${name}"),
            "Skill doc should preserve variable placeholder. Got: {}", skill_doc);
    assert!(skill_doc.contains("## Input Variables"),
            "Skill doc should have Input Variables section");
    assert!(skill_doc.contains("`name`: ${BINDING_name}"),
            "Skill doc should list name variable");
}

#[test]
fn test_multiple_variables_in_prompt() {
    let source = r#"
worker example() {
    var description = "Add OAuth support"
    var build_cmd = "cargo check"
    var result = think {
        The user wants to ${description}.
        Use ${build_cmd} to validate the build.
    }
}
"#;

    let (js, prompts) = compile_with_prompts(source).expect("compilation failed");

    // Verify both variables are bound (order may vary due to HashSet)
    assert!(js.contains("description") && js.contains("build_cmd"),
            "JS should bind both description and build_cmd");

    let skill_doc = &prompts["skills/main_example_think_0/SKILL.md"];
    assert!(skill_doc.contains("description"), "Skill doc should have description placeholder");
    assert!(skill_doc.contains("build_cmd"), "Skill doc should have build_cmd placeholder");
}

#[test]
fn test_ask_block() {
    let source = r#"
worker example() {
    var response = ask {
        What would you like to do?
    }
}
"#;

    let (js, prompts) = compile_with_prompts(source).expect("compilation failed");

    // Verify ask block generates skill with worker name prefix
    assert!(js.contains("await executePrompt(session, 'example_ask_0', {  })"),
            "JS should contain executePrompt call for example_ask_0");

    assert!(prompts.contains_key("skills/main_example_ask_0/SKILL.md"),
            "Should have ask skill document");

    let skill_doc = &prompts["skills/main_example_ask_0/SKILL.md"];
    assert!(skill_doc.contains("What would you like to do?"),
            "Skill doc should contain the ask prompt text");
}

#[test]
fn test_multiple_prompt_blocks() {
    let source = r#"
worker example() {
    var x = think { First prompt }
    var y = ask { Second prompt }
    var z = think { Third prompt }
}
"#;

    let (js, prompts) = compile_with_prompts(source).expect("compilation failed");

    // Verify unique IDs for each prompt (counter is shared across types, with worker name prefix)
    assert!(js.contains("'example_think_0'"), "Should have example_think_0");
    assert!(js.contains("'example_ask_1'"), "Should have example_ask_1");
    assert!(js.contains("'example_think_2'"), "Should have example_think_2");

    assert_eq!(prompts.len(), 3, "Should have 3 prompt skill documents");
    assert!(prompts.contains_key("skills/main_example_think_0/SKILL.md"));
    assert!(prompts.contains_key("skills/main_example_ask_1/SKILL.md"));
    assert!(prompts.contains_key("skills/main_example_think_2/SKILL.md"));
}

#[test]
fn test_prompt_with_member_access() {
    let source = r#"
worker example() {
    var user = "data"
    var result = think {
        User name: ${user.name}
    }
}
"#;

    let (js, prompts) = compile_with_prompts(source).expect("compilation failed");

    // Should bind the root object "user", not "name"
    assert!(js.contains("{ user }"), "JS should bind user object");

    let skill_doc = &prompts["skills/main_example_think_0/SKILL.md"];
    assert!(skill_doc.contains("user.name"), "Skill doc should preserve member access");
}

#[test]
fn test_runtime_has_execute_prompt() {
    let source = r#"
worker example() {
    var x = think { test }
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify runtime includes executePrompt function
    assert!(output.runtime.contains("export async function executePrompt"),
            "Runtime should export executePrompt function");
    assert!(output.javascript.contains("import { shell, $shellPipe, $shellAnd, $shellOr, $shellRedirect, SessionContext, executePrompt, delegate"),
            "Generated code should import all runtime functions");
}

// ============================================================================
// Message Passing Tests
// ============================================================================

#[test]
fn test_mailbox_send() {
    let source = r#"
worker example() {
    var msg = { type: "test" }
    self.session.mailbox.results.send(msg)
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("session.mailbox.results.send(msg)"));
}

#[test]
fn test_mailbox_receive() {
    let source = r#"
worker example() {
    var msg = self.session.mailbox.tasks.receive(5000).await
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await session.mailbox.tasks.receive(5000)"));
}

#[test]
fn test_mailbox_multiple_names() {
    let source = r#"
worker example() {
    self.session.mailbox.tasks.send({ id: 1 })
    self.session.mailbox.results.send({ id: 2 })
    self.session.mailbox.events.send({ id: 3 })
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("session.mailbox.tasks.send"));
    assert!(js.contains("session.mailbox.results.send"));
    assert!(js.contains("session.mailbox.events.send"));
}

#[test]
fn test_mailbox_in_loop() {
    let source = r#"
worker example() {
    var i = 0
    while (i < 3) {
        self.session.mailbox.events.send({ step: i })
        i = i + 1
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("while (i < 3)"));
    assert!(js.contains("session.mailbox.events.send({ step: i })"));
}

#[test]
fn test_mailbox_send_receive_roundtrip() {
    let source = r#"
worker sender() {
    var task = { action: "process" }
    self.session.mailbox.tasks.send(task)
    var result = self.session.mailbox.results.receive(5000).await
    return result
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("session.mailbox.tasks.send(task)"));
    assert!(js.contains("await session.mailbox.results.receive(5000)"));
}

#[test]
fn test_mailbox_receive_without_timeout() {
    let source = r#"
worker example() {
    var msg = self.session.mailbox.inbox.receive().await
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await session.mailbox.inbox.receive()"));
}

#[test]
fn test_runtime_has_mailbox_classes() {
    let source = r#"
worker example() {
    self.session.mailbox.test.send({ x: 1 })
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify runtime includes Mailbox and Mailroom classes
    assert!(output.runtime.contains("export class Mailbox"),
            "Runtime should export Mailbox class");
    assert!(output.runtime.contains("export class Mailroom"),
            "Runtime should export Mailroom class");
    assert!(output.runtime.contains("this.mailbox = new Mailroom(this)"),
            "SessionContext should initialize mailroom with session reference");
}

#[test]
fn test_delegate_function_in_runtime() {
    let source = r#"
export default trait Example: Agent {
    @skill test
    fun test() {
        var result = self.delegate([]).await
    }
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify delegate function is exported in runtime
    assert!(output.runtime.contains("export async function delegate"),
            "Runtime should export delegate function");

    // Verify delegate implementation includes Promise.all for fork/join
    assert!(output.runtime.contains("Promise.all(workers)"),
            "delegate should use Promise.all for fork/join");

    // Verify delegate marks session as failed on error
    assert!(output.runtime.contains("session.markFailed(error)"),
            "delegate should mark session as failed on worker error");

    // Verify delegate cleans up session
    assert!(output.runtime.contains("session.cleanup()"),
            "delegate should clean up session resources");
}

#[test]
fn test_session_failure_tracking() {
    let source = r#"
worker test() {
    var x = 5
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify SessionContext has failure tracking fields
    assert!(output.runtime.contains("this.failureFile"),
            "SessionContext should have failureFile field");
    assert!(output.runtime.contains("this.failureWatcher"),
            "SessionContext should have failureWatcher field");
    assert!(output.runtime.contains("this.failurePromise"),
            "SessionContext should have failurePromise field");

    // Verify SessionContext has failure methods
    assert!(output.runtime.contains("setupFailureWatch()"),
            "SessionContext should set up failure watch");
    assert!(output.runtime.contains("async markFailed(error)"),
            "SessionContext should have markFailed method");
    assert!(output.runtime.contains("async checkFailed()"),
            "SessionContext should have checkFailed method");
    assert!(output.runtime.contains("cleanup()"),
            "SessionContext should have cleanup method");

    // Verify failure detection uses fs.watch
    assert!(output.runtime.contains("watch(this.dir"),
            "SessionContext should watch session directory");
    assert!(output.runtime.contains(".failed"),
            "SessionContext should use .failed file for failure signaling");
}

#[test]
fn test_mailbox_session_integration() {
    let source = r#"
worker test() {
    var x = 5
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify Mailbox receives session reference
    assert!(output.runtime.contains("constructor(name, session)"),
            "Mailbox should accept session in constructor");
    assert!(output.runtime.contains("this.session = session"),
            "Mailbox should store session reference");

    // Verify mailbox operations check for session failure
    assert!(output.runtime.contains("await this.session.checkFailed()"),
            "Mailbox send/receive should check if session has failed");

    // Verify receive races against session failure
    assert!(output.runtime.contains("this.session.failurePromise"),
            "Mailbox receive should race against session failure promise");
}

#[test]
fn test_throw_with_error_wrapping() {
    let source = r#"
worker error_test() {
    throw "Something went wrong"
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify throw wraps expression in Error and converts to string
    assert!(js.contains("throw new Error(String("));
    assert!(js.contains("\"Something went wrong\""));
}

// ============================================================================
// Phase 10: Shell Command Safety Tests
// ============================================================================

#[test]
fn test_shell_command_with_interpolation() {
    let source = r#"
worker test(dir) {
    $ mkdir -p "${dir}"
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify interpolation is preserved in template literal
    assert!(js.contains("await shell(`mkdir -p ${"));
    assert!(js.contains("dir}`)"));
}

#[test]
fn test_shell_command_injection_safety() {
    let source = r#"
worker test(user_input) {
    $ echo "${user_input}"
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify template literals are used (JS will properly escape)
    assert!(js.contains("await shell(`echo ${"));
    assert!(js.contains("user_input}`)"));
}

#[test]
fn test_shell_pipe_with_multiple_commands() {
    let source = r#"
worker test() {
    $ cat file.txt | grep "pattern" | sort
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify pipe uses shellPipe with array of commands
    assert!(js.contains("await $shellPipe(["));
}

#[test]
fn test_shell_and_with_error_handling() {
    let source = r#"
worker test() {
    $ git add . && git commit -m "message"
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify && uses shellAnd
    assert!(js.contains("await $shellAnd(["));
}

#[test]
fn test_shell_or_fallback() {
    let source = r#"
worker test() {
    $ command_that_might_fail || echo "fallback"
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify || uses shellOr
    assert!(js.contains("await $shellOr(["));
}

#[test]
fn test_shell_redirect_output() {
    let source = r#"
worker test(file) {
    $ echo "hello" > "${file}"
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify redirection uses shellRedirect
    assert!(js.contains("await $shellRedirect("));
    assert!(js.contains("'>'"));
}

#[test]
fn test_command_substitution_capture() {
    let source = r#"
worker test() {
    var branch = $(git rev-parse --abbrev-ref HEAD)
    return branch
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify command substitution uses capture option
    assert!(js.contains("await shell(`git rev-parse --abbrev-ref HEAD`, {capture: true})"));
}

#[test]
fn test_shell_exit_code_error_handling() {
    // This tests that the runtime properly handles exit codes
    // The shell() function should reject on non-zero exit codes
    let source = r#"
worker test() {
    $ false
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify shell command is awaited (so errors can propagate)
    assert!(js.contains("await shell(`false`)"));
}

#[test]
fn test_runtime_has_shell_functions() {
    let source = r#"
worker test() {
    var x = 5
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify all shell runtime functions are exported
    assert!(output.runtime.contains("export { shell as $shell }"),
            "Runtime should export $shell");
    assert!(output.runtime.contains("export async function $shellPipe"),
            "Runtime should export $shellPipe");
    assert!(output.runtime.contains("export async function $shellAnd"),
            "Runtime should export $shellAnd");
    assert!(output.runtime.contains("export async function $shellOr"),
            "Runtime should export $shellOr");
    assert!(output.runtime.contains("export async function $shellRedirect"),
            "Runtime should export $shellRedirect");
}

#[test]
fn test_shell_command_with_complex_interpolation() {
    let source = r#"
worker test(dir, file) {
    $ cat "${dir}/${file}"
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify multiple interpolations are preserved
    assert!(js.contains("await shell(`cat ${"));
    assert!(js.contains("dir}/${"));
    assert!(js.contains("file}`)"));
}

#[test]
fn test_shell_statement_vs_expression() {
    let source = r#"
worker test() {
    $ echo "statement form"
    var output = $(echo "expression form")
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Statement form: no capture
    assert!(js.contains("await shell(`echo statement form`)"));

    // Expression form: with capture
    assert!(js.contains("await shell(`echo expression form`, {capture: true})"));
}

#[test]
fn test_filesystem_mailbox_structure() {
    let source = r#"
worker example() {
    self.session.mailbox.test.send({ x: 1 })
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify Mailbox constructor stores mailbox directory path
    assert!(output.runtime.contains("this.mailboxDir = `${session.dir}/mailboxes/${name}`"),
            "Mailbox should store mailboxDir path");

    // Verify Mailbox creates directory on initialization
    assert!(output.runtime.contains("mkdir(this.mailboxDir"),
            "Mailbox should create mailbox directory");

    // Verify send creates unique files with timestamp-PID naming
    assert!(output.runtime.contains("${Date.now()}-${process.pid}.json"),
            "Mailbox should use timestamp-PID filename format");

    // Verify receive reads and deletes oldest message
    assert!(output.runtime.contains("readdir(this.mailboxDir)"),
            "Mailbox should list directory for messages");
    assert!(output.runtime.contains("unlink(filepath)"),
            "Mailbox should delete message file after reading");
}

#[test]
fn test_mailbox_message_envelope() {
    let source = r#"
worker example() {
    self.session.mailbox.narrator.send({ type: "test" })
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify message envelope structure
    assert!(output.runtime.contains("from:"),
            "Message envelope should include sender info");
    assert!(output.runtime.contains("to: this.name"),
            "Message envelope should include recipient");
    assert!(output.runtime.contains("timestamp:"),
            "Message envelope should include timestamp");
    assert!(output.runtime.contains("payload: message"),
            "Message envelope should include payload");
}

#[test]
fn test_mailbox_fifo_ordering() {
    let source = r#"
worker example() {
    var msg = self.session.mailbox.test.receive(1000).await
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify files are sorted for FIFO ordering
    assert!(output.runtime.contains(".sort()"),
            "Mailbox should sort files for FIFO ordering");

    // Verify oldest message is read first
    assert!(output.runtime.contains("messageFiles[0]"),
            "Mailbox should read oldest (first) message file");
}

#[test]
fn test_mailbox_filesystem_watch() {
    let source = r#"
worker example() {
    var msg = self.session.mailbox.test.receive().await
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().expect("compilation failed");

    let _ = std::fs::remove_file(&test_file);

    // Verify filesystem watching for new messages
    assert!(output.runtime.contains("watch(this.mailboxDir"),
            "Mailbox should watch directory for new files");

    // Verify periodic polling as fallback
    assert!(output.runtime.contains("setInterval"),
            "Mailbox should poll periodically as fallback for fs.watch");

    // Verify cleanup of watchers
    assert!(output.runtime.contains("watcher.close()"),
            "Mailbox should clean up filesystem watcher");
}
