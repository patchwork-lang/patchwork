# Patchwork ACP Implementation Plan

This document outlines the phased implementation strategy for the Patchwork ACP interpreter. See [patchwork-acp-design.md](./patchwork-acp-design.md) for architectural details.

## Architecture Note

**Major Design Update (Phase 3+)**: The implementation now uses a synchronous blocking model inspired by [Niko Matsakis's threadbare prototype](https://github.com/nikomatsakis/threadbare/). The interpreter runs on dedicated OS threads and blocks at `think` blocks, with the OS thread stack automatically preserving all execution context. This eliminates the need for `ControlState` yield/resume and manual continuation management that was planned in the original Phase 3.

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

## Phase 3: Threading Infrastructure Refactor ✅

**Goal**: Refactor evaluation to use synchronous blocking model with threading

**Success Criteria**: Evaluation engine uses `Result<Value, Error>`, ready for agent integration

### Remove ControlState System

- [x] Simplify evaluation return types
  - [x] Change `eval_expr()` from `Result<ControlState, Error>` to `Result<Value, Error>`
  - [x] Change `eval_statement()` to return `Result<Value, Error>`
  - [x] Change `eval_block()` to return `Result<Value, Error>`
  - [x] Remove `ControlState` enum entirely
  - [x] Remove `try_eval!` macro

- [x] Update exception handling
  - [x] Add `Error::Exception(Value)` variant
  - [x] Change `throw` evaluation to return `Err(Error::Exception(value))`
  - [x] Update all error propagation to use `?` operator
  - [x] Verify exceptions propagate correctly through call stack

- [x] Update interpreter API
  - [x] Change `Interpreter::eval()` to return `Result<Value, Error>`
  - [x] Remove `Interpreter::resume()` method
  - [x] Remove `state` field from `Interpreter`
  - [x] Update all tests to use new signatures

### Testing

- [x] Update all existing tests for new signatures
  - [x] Change assertions from `ControlState::Return(value)` to `Ok(value)`
  - [x] Update error handling tests for `Error::Exception`
- [x] Verify all tests still pass (279 tests across workspace)
- [x] Add test for exception propagation through nested calls

---

## Phase 4: Agent Infrastructure ✅

**Goal**: Build agent thread infrastructure for LLM communication

**Success Criteria**: Agent can create sessions, send prompts, and communicate with interpreter threads

**Note**: Agent lives in `patchwork-acp` (not `patchwork-eval`) because it's deeply integrated with SACP.

### Agent Core Types

- [x] Create `agent.rs` module in `patchwork-acp`
  - [x] Define `Agent` struct with `UnboundedSender<AgentRequest>`
  - [x] Define `AgentRequest` enum with Think variant
  - [x] Define `ThinkResponse` enum for internal agent use
  - [x] Implement `Agent::new(cx, mcp_registry)` to start agent tasks

### Agent Client Loop

- [x] Implement agent request loop
  - [x] Uses proxy's Tokio runtime (not separate thread)
  - [x] Uses proxy's connection context for SACP
  - [x] Create unbounded channel for receiving `AgentRequest`s
  - [x] Loop: receive requests, spawn `think_message` tasks

### Think Message Task

- [x] Implement `think_message()` async function
  - [x] Create new SACP session with MCP server
  - [x] Send prompt request to successor
  - [x] Wait for prompt response
  - [x] Extract typed value from response (markdown code fence)
  - [x] Send value back through `response_tx` oneshot channel

### Redirect Actor (for nested thinks)

- [x] Implement redirect actor
  - [x] Maintain `Vec<Sender<PerSessionMessage>>` stack
  - [x] Handle `PushThinker` / `PopThinker` messages
  - [x] Route incoming SACP messages to top of stack
  - [x] Forward session notifications to active thinker
  - [x] Forward MCP tool calls to active thinker

### MCP Do Tool

- [x] Implement MCP server for `do` tool
  - [x] `Agent::create_mcp_server()` factory method
  - [x] Handle `do(number)` invocations from LLM
  - [x] Create oneshot channel for result
  - [x] Send `DoInvocation` through redirect actor
  - [x] Wait for result, return to LLM
  - [ ] Recursive evaluation (Phase 5)

### Response Extraction

- [x] Implement markdown code fence parsing
  - [x] `extract_code_fence()` function
  - [x] Look for ` ```text ... ``` ` or ` ```json ... ``` ` markers
  - [x] Extract content between fences
  - [x] Trim whitespace from extracted content
  - [x] Parse JSON if expect type is not string
  - [x] Fallback to full response text if no markers found

### Testing

- [x] Unit test: Code fence extraction
  - [x] Test with text fences
  - [x] Test with json fences
  - [x] Test fallback when no fences
- [ ] Unit test: Agent message routing (requires mock SACP - deferred)

---

## Phase 5: Interpreter-Agent Integration

**Goal**: Connect interpreter to agent using blocking channels

**Success Criteria**: Think blocks work end-to-end with synchronous blocking

### Think Block Evaluation

- [ ] Update `eval_think_block()` implementation
  - [ ] Interpolate prompt text with variable values
  - [ ] Collect variable bindings for LLM context
  - [ ] Create oneshot channel for response
  - [ ] Send `AgentRequest::Think` to agent via mpsc
  - [ ] Block on `response_rx.recv()`
  - [ ] Return received `Value` as result

- [ ] Update `Interpreter` struct
  - [ ] Add `agent_tx: UnboundedSender<AgentRequest>` field
  - [ ] Update `Interpreter::new(agent: Agent)` to take agent handle
  - [ ] Remove all `ControlState` references

### Proxy Thread Spawning

- [ ] Update proxy to spawn interpreter threads
  - [ ] Create shared `Agent` instance on proxy startup
  - [ ] On code execution: spawn OS thread for interpreter
  - [ ] In spawned thread: create interpreter with agent handle
  - [ ] Call `interpreter.eval(code)` (blocks until complete)
  - [ ] Send result back to proxy via oneshot channel
  - [ ] Proxy awaits result and returns ACP response

- [ ] Update session tracking
  - [ ] Change from `HashMap<SessionId, Interpreter>` to `HashSet<SessionId>`
  - [ ] Mark session active when spawning interpreter thread
  - [ ] Mark session inactive when interpreter completes
  - [ ] Return error if session already has active evaluation

### Type Hint Generation

- [ ] Implement type hint formatting
  - [ ] For string types: append text fence instructions to prompt
  - [ ] For future types: support json fence instructions
  - [ ] Make hints clear and concise for LLM

### Testing

- [ ] Integration test: Single think block
  - [ ] Create test with think block returning string
  - [ ] Mock agent response with markdown fence
  - [ ] Verify value extracted correctly
  - [ ] Verify interpreter completes successfully

- [ ] Integration test: Think block in for-loop
  - [ ] Test multiple think blocks in iterations
  - [ ] Verify each iteration gets correct response
  - [ ] Verify loop state preserved across blocks
  - [ ] Verify all output files created

- [ ] Integration test: Nested think blocks
  - [ ] Outer think calls do(0), inner has another think
  - [ ] Verify redirect actor routes messages correctly
  - [ ] Verify both sessions complete successfully
  - [ ] Verify call stack unwinding works

---

## Phase 6: Full Demo and Polish

**Goal**: Complete interview sanitization demo and production polish

**Success Criteria**: Full demo works end-to-end, robust error handling, good diagnostics

### End-to-End Demo

- [ ] Create interview sanitization test data
  - [ ] Multiple interview directories with transcripts
  - [ ] Realistic metadata.json files
  - [ ] Transcripts with filler words and misspellings

- [ ] Test full demo manually
  - [ ] Run through Zed with real LLM
  - [ ] Verify sanitized transcripts generated
  - [ ] Verify output quality
  - [ ] Test error handling (missing files, etc.)

### Error Handling

- [ ] Improve parse error messages
  - [ ] Include source location (line/column)
  - [ ] Show snippet of problematic code
  - [ ] Suggest fixes where possible

- [ ] Improve runtime error messages
  - [ ] Clear error categories (type error, file not found, etc.)
  - [ ] Include relevant context (variable values, file paths)
  - [ ] Helpful suggestions for common errors

- [ ] Handle thread/channel errors gracefully
  - [ ] Agent disconnection
  - [ ] Channel send/receive failures
  - [ ] Thread panic recovery

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

- [ ] Handle concurrent evaluation attempts
  - [ ] Return clear error if session already active
  - [ ] Clean up properly on error or completion

### Documentation

- [ ] Write README for patchwork-acp crate
  - [ ] Installation instructions
  - [ ] Zed configuration example
  - [ ] Basic usage examples
  - [ ] Threading architecture overview

- [ ] Write README for patchwork-eval crate
  - [ ] API documentation
  - [ ] Example of using interpreter directly
  - [ ] Explanation of threading model
  - [ ] Agent integration guide

- [ ] Create examples directory
  - [ ] Interview sanitization demo (with test data)
  - [ ] Simple think block examples
  - [ ] Loop examples
  - [ ] File I/O examples
  - [ ] Nested think block example

### Integration Tests

- [ ] Create mock agent for testing
  - [ ] Responds with predictable formatted responses
  - [ ] Can be configured per test case
  - [ ] Simulates do() tool calls

- [ ] Test suite for full scenarios
  - [ ] Various loop patterns
  - [ ] Nested data structures
  - [ ] Error conditions (file errors, parse errors, etc.)
  - [ ] Edge cases (empty loops, missing variables, etc.)
  - [ ] Concurrent session handling

### Performance

- [ ] Profile interpreter execution
  - [ ] Identify bottlenecks
  - [ ] Optimize hot paths if needed

- [ ] Memory usage analysis
  - [ ] Verify session cleanup works correctly
  - [ ] Check for channel/thread leaks
  - [ ] Monitor agent thread resource usage

---

## Success Metrics

**Phase 1 Complete**: Proxy runs in conductor, detects code, logs AST, forwards prompts ✅
**Phase 2 Complete**: Deterministic demo works (loops, file I/O, shell commands) ✅
**Phase 3 Complete**: Evaluation uses `Result<Value, Error>`, ready for threading ✅
**Phase 4 Complete**: Agent infrastructure built, can create sessions and send prompts ✅
**Phase 5 Complete**: Interpreter threads block on agent, think blocks work end-to-end
**Phase 6 Complete**: Production-ready with robust errors, docs, and tests

**Final Demo**: User runs interview sanitization in Zed, gets sanitized transcripts in files
