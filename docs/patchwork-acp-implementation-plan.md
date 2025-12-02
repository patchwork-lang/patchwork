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

- [x] Create `crates/patchwork-acp/` binary crate
  - [x] Add to workspace `Cargo.toml`
  - [x] Set up `Cargo.toml` with dependencies:
    - [x] `sacp` crate
    - [x] `sacp-proxy` crate
    - [x] `tokio` for async runtime
    - [x] `patchwork-eval` (local dependency)
  - [x] Create `src/main.rs` entry point

- [x] Create `crates/patchwork-eval/` library crate
  - [x] Add to workspace `Cargo.toml`
  - [x] Set up `Cargo.toml` with dependencies:
    - [x] `patchwork-parser` (local dependency)
    - ~~`patchwork-lexer` (local dependency)~~ (not needed - parser handles lexing)
  - [x] Create `src/lib.rs` with module structure

### Proxy Implementation

- [x] Implement basic SACP proxy structure
  - [x] Set up `JrHandlerChain` with stdio connection
  - [x] Register as proxy component (`.proxy()`)
  - [x] Handle `initialize` request (capability handshake)
  - [x] Forward all requests to successor by default

- [x] Implement code detection
  - [x] Intercept `prompt` requests
  - [x] Extract text content from messages
  - [x] Check if text starts with `{` (trim whitespace first)
  - [x] Log detected code blocks to stderr
  - [x] Forward non-code prompts unchanged

- [x] Session management skeleton
  - [x] Define `PatchworkProxy` struct with session storage
  - [x] Implement `has_active_evaluation()`, `store_interpreter()`, `retrieve_interpreter()`, `remove_interpreter()`
  - [x] Return error if code submitted while eval in progress

### Interpreter Skeleton

- [x] Define core types in `patchwork-eval`
  - [x] `pub struct Interpreter` with `state: ControlState`
  - [x] `pub enum ControlState { Eval, Yield { ... }, Return(Value), Throw(Value) }`
  - [x] `pub enum LlmOp { Think, Ask }`
  - [x] `pub type Bindings = HashMap<String, Value>`

- [x] Implement `Interpreter::new()`
  - [x] Set initial state to `ControlState::Eval`

- [x] Implement `Interpreter::eval(code: &str)` stub
  - [x] Parse code using `patchwork-parser`
  - [x] Return `ControlState::Return(Value::Null)` for now
  - [x] Log AST to stderr for debugging

- [x] Implement `Interpreter::resume(value: Value)` stub
  - [x] Return error "not yet implemented"

### Testing

- [x] Manual test: Run proxy with `sacp-conductor`
  - [x] Verify proxy starts without errors
  - [x] Send normal prompt, verify forwarding works
  - [x] Send `{ }` code block, verify detection and logging

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

- [x] Define `Value` enum in `patchwork-eval`
  - [x] `Null`, `String(String)`, `Number(f64)`, `Boolean(bool)`
  - [x] `Array(Vec<Value>)`, `Object(HashMap<String, Value>)`
  - [x] Implement `Display` for debugging

- [x] Implement type conversions
  - [x] `Value::to_string()` for string coercion
  - [x] `Value::to_bool()` for boolean coercion
  - [x] `Value::from_json()` for parsing JSON
  - [x] `Value::to_json()` for serializing to JSON

### Runtime Environment

- [x] Implement `Runtime` struct
  - [x] `scopes: Vec<HashMap<String, Value>>` - scoped variable bindings
  - [x] `working_dir: PathBuf` - current directory
  - [x] Helper methods: `get_var()`, `set_var()`, `define_var()`
  - [x] Scope management: `push_scope()`, `pop_scope()`

- [x] Implement shell command execution
  - [x] Execute `std::process::Command` with captured output
  - [x] Parse stdout as array of lines for `ls -1`
  - [x] Handle exit codes (non-zero = exception)
  - [x] Implement string interpolation in command strings

- [x] Implement file I/O
  - [x] `json < file` - read file, parse as JSON, return Value (via ShellRedirect)
  - [x] `cat(value) > file` - serialize value to JSON, write to file (via ShellRedirect)
  - [x] `read(path)` - read file contents as string
  - [x] `write(path, content)` - write string to file
  - [x] Handle file errors (not found, permission denied, etc.)

### Expression Evaluation

- [x] Implement expression evaluator
  - [x] Literals: strings, numbers, booleans, null
  - [x] Variables: lookup in runtime environment
  - [x] Shell commands: execute and return Value (via BareCommand)
  - [x] Function calls: `cat(...)`, `json(...)` builtins
  - [x] String interpolation: `$var` and `${expr}`
  - [x] Binary operations: arithmetic, comparison, logical
  - [x] Unary operations: not, negation, throw
  - [x] Member access: `obj.field`
  - [x] Index access: `arr[i]`
  - [x] Object and array literals

### Statement Evaluation

- [x] Implement statement evaluator
  - [x] Variable declarations: `var name = expr`
  - [x] Destructuring: `var { a, b } = expr`
  - [x] For-loops: `for var x in expr { ... }`
  - [x] Block statements: `{ stmt; stmt; ... }`
  - [x] Expression statements
  - [x] If/else statements
  - [x] While loops

- [x] Implement control flow
  - [x] Return values from blocks
  - [x] Break statement (basic)
  - [x] Exception propagation (via Error)

### Builtin Functions

- [x] Implement `cat(value: Value) -> String`
  - [x] Serialize to pretty-printed JSON
  - [x] Return as string value

- [x] Implement `json(text: String) -> Value`
  - [x] Parse JSON string
  - [x] Return parsed value
  - [x] Throw exception on parse error

- [x] Additional builtins: `print`, `len`, `keys`, `values`, `typeof`, `read`, `write`

### Testing

- [x] Unit tests for value conversions
- [x] Unit tests for shell command execution (via bare command tests)
- [x] Unit tests for file I/O operations
- [x] Unit tests for expression evaluation
- [x] Unit tests for statement evaluation
- [x] Integration test: Run simplified demo (no think blocks)
  - [x] Create test directory with interview folders
  - [x] Create metadata.json files
  - [x] Run Patchwork code
  - [x] Verify output.json files created correctly

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
- [ ] Verify error when sending code while eval in progress (from Phase 1)

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
