# Patchwork ACP Implementation Plan

This document outlines the phased implementation strategy for the Patchwork ACP interpreter. See [patchwork-acp-design.md](./patchwork-acp-design.md) for architectural details.

## Goal

Build an ACP proxy that interprets Patchwork code in real-time, enabling a "supercharged prompting language" that blends deterministic control flow with nondeterministic LLM reasoning.

**Demo Target**: Interview sanitization script that processes multiple transcripts in a loop, using think blocks to clean up each one.

---

## Phase 1: Minimal Infrastructure

**Goal**: Establish proxy skeleton and plumbing

**Success Criteria**: Proxy runs in conductor chain, detects code blocks, logs them, forwards normal prompts

### Setup Tasks

- [ ] Create `crates/patchwork-acp/` binary crate
  - [ ] Add to workspace `Cargo.toml`
  - [ ] Set up `Cargo.toml` with dependencies:
    - [ ] `sacp` crate
    - [ ] `sacp-proxy` crate
    - [ ] `tokio` for async runtime
    - [ ] `patchwork-eval` (local dependency)
  - [ ] Create `src/main.rs` entry point

- [ ] Create `crates/patchwork-eval/` library crate
  - [ ] Add to workspace `Cargo.toml`
  - [ ] Set up `Cargo.toml` with dependencies:
    - [ ] `patchwork-parser` (local dependency)
    - [ ] `patchwork-lexer` (local dependency)
  - [ ] Create `src/lib.rs` with module structure

### Proxy Implementation

- [ ] Implement basic SACP proxy structure
  - [ ] Set up `JrHandlerChain` with stdio connection
  - [ ] Register as proxy component (`.proxy()`)
  - [ ] Handle `initialize` request (capability handshake)
  - [ ] Forward all requests to successor by default

- [ ] Implement code detection
  - [ ] Intercept `prompt` requests
  - [ ] Extract text content from messages
  - [ ] Check if text starts with `{` (trim whitespace first)
  - [ ] Log detected code blocks to stderr
  - [ ] Forward non-code prompts unchanged

- [ ] Session management skeleton
  - [ ] Define `PatchworkProxy` struct with session storage
  - [ ] Implement `has_active_evaluation()`, `store_interpreter()`, `retrieve_interpreter()`, `remove_interpreter()`
  - [ ] Return error if code submitted while eval in progress

### Interpreter Skeleton

- [ ] Define core types in `patchwork-eval`
  - [ ] `pub struct Interpreter` with `runtime: Runtime` and `state: ControlState`
  - [ ] `pub enum ControlState { Eval, Yield { ... }, Return(Value), Throw(Value) }`
  - [ ] `pub enum LlmOp { Think, Ask }`
  - [ ] `pub type Bindings = HashMap<String, Value>`

- [ ] Implement `Interpreter::new()`
  - [ ] Initialize runtime
  - [ ] Set initial state to `ControlState::Eval`

- [ ] Implement `Interpreter::eval(code: &str)` stub
  - [ ] Parse code using `patchwork-parser`
  - [ ] Return `ControlState::Return(Value::Null)` for now
  - [ ] Log AST to stderr for debugging

- [ ] Implement `Interpreter::resume(value: Value)` stub
  - [ ] Return error "not yet implemented"

### Testing

- [ ] Manual test: Run proxy with `sacp-conductor`
  - [ ] Verify proxy starts without errors
  - [ ] Send normal prompt, verify forwarding works
  - [ ] Send `{ }` code block, verify detection and logging
  - [ ] Verify error when sending code while eval in progress

---

## Phase 2: Shell Command Execution

**Goal**: Execute deterministic parts of demo

**Success Criteria**: Execute simplified demo without think blocks:
```patchwork
{
  for var interview in ($ ls -1 ./) {
    var data = json < "$interview/metadata.json"
    cat(data) > "$interview/output.json"
  }
}
```

### Value System

- [ ] Define `Value` enum in `patchwork-eval`
  - [ ] `Null`, `String(String)`, `Number(f64)`, `Boolean(bool)`
  - [ ] `Array(Vec<Value>)`, `Object(HashMap<String, Value>)`
  - [ ] Implement `Display` for debugging

- [ ] Implement type conversions
  - [ ] `Value::to_string()` for string coercion
  - [ ] `Value::to_bool()` for boolean coercion
  - [ ] `Value::from_json()` for parsing JSON

### Runtime Environment

- [ ] Implement `Runtime` struct
  - [ ] `variables: HashMap<String, Value>` - variable bindings
  - [ ] `working_dir: PathBuf` - current directory
  - [ ] Helper methods: `get_var()`, `set_var()`, `define_var()`

- [ ] Implement shell command execution
  - [ ] Execute `std::process::Command` with captured output
  - [ ] Parse stdout as array of lines for `$ ls -1`
  - [ ] Handle exit codes (non-zero = exception)
  - [ ] Implement string interpolation in command strings

- [ ] Implement file I/O
  - [ ] `json < file` - read file, parse as JSON, return Value
  - [ ] `cat(value) > file` - serialize value to JSON, write to file
  - [ ] Handle file errors (not found, permission denied, etc.)

### Expression Evaluation

- [ ] Implement expression evaluator
  - [ ] Literals: strings, numbers, booleans, null
  - [ ] Variables: lookup in runtime environment
  - [ ] Shell commands: execute and return Value
  - [ ] Function calls: `cat(...)`, `json(...)` builtins
  - [ ] String interpolation: `${expr}` and `$@{expr}`

### Statement Evaluation

- [ ] Implement statement evaluator
  - [ ] Variable declarations: `var name = expr`
  - [ ] Destructuring: `var { a, b } = expr`
  - [ ] For-loops: `for var x in expr { ... }`
  - [ ] Block statements: `{ stmt; stmt; ... }`
  - [ ] Expression statements

- [ ] Implement control flow
  - [ ] Return values from blocks
  - [ ] Break/continue (if needed for loops)
  - [ ] Exception propagation

### Builtin Functions

- [ ] Implement `cat(value: Value) -> String`
  - [ ] Serialize to pretty-printed JSON
  - [ ] Return as string value

- [ ] Implement `json(text: String) -> Value`
  - [ ] Parse JSON string
  - [ ] Return parsed value
  - [ ] Throw exception on parse error

### Testing

- [ ] Unit tests for value conversions
- [ ] Unit tests for shell command execution
- [ ] Unit tests for file I/O operations
- [ ] Unit tests for expression evaluation
- [ ] Unit tests for statement evaluation
- [ ] Integration test: Run simplified demo (no think blocks)
  - [ ] Create test directory with interview folders
  - [ ] Create metadata.json files
  - [ ] Run Patchwork code
  - [ ] Verify output.json files created correctly

---

## Phase 3: LLM Integration

**Goal**: Complete the demo with think blocks

**Success Criteria**: Full interview sanitization demo works end-to-end

### Think Block Parsing

- [ ] Handle `think { ... }` expressions
  - [ ] Parse think block prompt text
  - [ ] Extract variable references for interpolation
  - [ ] Preserve prompt structure for ACP message

- [ ] Implement variable interpolation
  - [ ] Evaluate `${expr}` in prompt text
  - [ ] Convert values to strings for embedding
  - [ ] Handle array spread `$@{expr}` (comma-space join)

### Interpreter Suspension

- [ ] Implement `Yield` control state
  - [ ] When evaluating think block, return `ControlState::Yield`
  - [ ] Include: `op: LlmOp::Think`, interpolated prompt, variable bindings, expected type
  - [ ] Store interpreter execution context (call stack, loop state, etc.)

- [ ] Make interpreter serializable/clonable
  - [ ] Ensure all state can be stored in session map
  - [ ] Consider using `Arc` or similar for shared data

### Proxy Integration

- [ ] Handle `Yield` state in proxy
  - [ ] Extract prompt and expected type from yield
  - [ ] Generate type hint instructions (e.g., "Respond with a string value")
  - [ ] Create ACP prompt request with formatting instructions
  - [ ] Store interpreter in session map
  - [ ] Forward prompt request to successor agent

- [ ] Handle agent response
  - [ ] Retrieve interpreter from session map
  - [ ] Extract typed value from response:
    - [ ] Look for ` ```text ... ``` ` fence
    - [ ] Extract content between markers
    - [ ] Fallback to full response text if no markers
  - [ ] Call `interpreter.resume(extracted_value)`
  - [ ] Handle next control state (Yield, Return, Throw)

### Type Hint Generation

- [ ] Implement type hint formatter
  - [ ] For `Type::String`: "Respond with a string value. Format your response as:\n\`\`\`text\nyour response here\n\`\`\`"
  - [ ] Store expected type in Yield state for extraction

- [ ] Implement response extraction
  - [ ] Parse markdown code fences
  - [ ] Extract content between ` ```text` and ` ``` `
  - [ ] Trim whitespace from extracted content
  - [ ] Return Value::String with extracted text

### Loop Integration

- [ ] Handle multiple suspension points
  - [ ] Test think block inside for-loop
  - [ ] Verify state preservation across iterations
  - [ ] Ensure loop counter and bindings persist

### Testing

- [ ] Unit test: Think block interpolation
- [ ] Unit test: Response extraction from markdown
- [ ] Integration test: Single think block
  - [ ] Mock agent response
  - [ ] Verify extracted value
  - [ ] Verify interpreter resumes correctly
- [ ] Integration test: Think block in loop
  - [ ] Mock agent responses for each iteration
  - [ ] Verify all iterations complete
  - [ ] Verify output files created
- [ ] End-to-end test: Full demo with real agent
  - [ ] Set up test interview directories
  - [ ] Run demo code through Zed or conductor
  - [ ] Verify sanitized transcripts written correctly

---

## Phase 4: Type Hints and Polish

**Goal**: Production quality

**Success Criteria**: Robust error handling, good diagnostics, documented examples

### Error Handling

- [ ] Improve parse error messages
  - [ ] Include source location (line/column)
  - [ ] Show snippet of problematic code
  - [ ] Suggest fixes where possible

- [ ] Improve runtime error messages
  - [ ] Include stack trace showing call path
  - [ ] Show variable values at error site
  - [ ] Clear error categories (type error, file not found, etc.)

- [ ] Convert uncaught exceptions to user messages
  - [ ] Format exception as brief, clear error text
  - [ ] Return as ACP response (not just logging)
  - [ ] Clean up session state on error

### Edge Cases

- [ ] Handle empty arrays in loops
  - [ ] Loop executes zero times (expected)
  - [ ] No error, just no iterations

- [ ] Handle malformed JSON
  - [ ] Clear parse error with location
  - [ ] Show problematic JSON snippet

- [ ] Handle file not found
  - [ ] Clear error message with file path
  - [ ] Suggest checking path or creating file

- [ ] Handle shell command failures
  - [ ] Show exit code and stderr
  - [ ] Include command that failed

### Documentation

- [ ] Write README for patchwork-acp crate
  - [ ] Installation instructions
  - [ ] Zed configuration example
  - [ ] Basic usage examples

- [ ] Write README for patchwork-eval crate
  - [ ] API documentation
  - [ ] Example of using interpreter directly
  - [ ] Explanation of control states

- [ ] Create examples directory
  - [ ] Interview sanitization demo (with test data)
  - [ ] Simple think block examples
  - [ ] Loop examples
  - [ ] File I/O examples

### Integration Tests

- [ ] Create mock agent for testing
  - [ ] Responds with predictable formatted responses
  - [ ] Can be configured per test case

- [ ] Test suite for full scenarios
  - [ ] Various loop patterns
  - [ ] Nested data structures
  - [ ] Error conditions (file errors, parse errors, etc.)
  - [ ] Edge cases (empty loops, missing variables, etc.)

### Performance

- [ ] Profile interpreter execution
  - [ ] Identify bottlenecks
  - [ ] Optimize hot paths if needed

- [ ] Memory usage analysis
  - [ ] Ensure interpreter state doesn't grow unbounded
  - [ ] Verify session cleanup works correctly

---

## Success Metrics

**Phase 1 Complete**: Proxy runs in conductor, detects code, logs AST, forwards prompts
**Phase 2 Complete**: Deterministic demo works (loops, file I/O, shell commands)
**Phase 3 Complete**: Full demo works with think blocks and LLM integration
**Phase 4 Complete**: Production-ready with robust errors, docs, and tests

**Final Demo**: User runs interview sanitization in Zed, gets sanitized transcripts in files
