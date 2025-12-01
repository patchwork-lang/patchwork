# Patchwork ACP Design

## Overview

`patchwork-acp` is an ACP proxy that interprets Patchwork code embedded in user prompts, enabling a "supercharged prompting language" that blends deterministic control flow with nondeterministic LLM reasoning.

**Architecture**: Interpreter-as-proxy middleware in SACP chain

```
Zed (ACP client)
  ↓
sacp-conductor
  ↓
patchwork-acp (proxy) ← interprets { code } blocks
  ↓
claude-code-acp (agent) ← handles think/ask prompts
```

This design is focused on the **interview sanitization demo** as the first milestone.

## Demo Goal: Interview Sanitization

### User Input (Patchwork code)

```patchwork
{
  for var interview in ($ ls -1 ./) {
    var { interviewees, interviewer, date, url } = json < "$interview/metadata.json"
    var sanitized: string = think {
      ${interview}/transcript.txt is a transcript of an interview about the
      Rust programming language on ${date}. The interviewer is ${interviewer}
      And the interviewees are $@{interviewees}.

      Read the transcript, correct misspellings of the participants' names,
      and remove filler words like "um" and "uh."
    }
    cat(sanitized) > "$interview/sanitized.txt"
  }
}
```

### Expected Behavior

1. **Patchwork code detection**: Input starts with `{` → execute as Patchwork
2. **Filesystem operations**: `$ ls -1 ./` runs in user's actual directory
3. **Variable binding**: Extract JSON fields into Patchwork variables
4. **Loop iteration**: Process each interview directory
5. **LLM prompt generation**: Each `think { }` block becomes an ACP prompt request
6. **Type-aware extraction**: LLM response extracted as `string` type
7. **Shell redirection**: Write sanitized output to file
8. **Silent execution**: Only think/ask blocks generate conversation messages

## Core Components

### 1. patchwork-acp (Binary Crate)

**Responsibilities:**
- ACP proxy using `sacp-proxy` framework
- Intercept user prompts from editor
- Detect Patchwork code (starts with `{`)
- Execute code via `patchwork-eval` interpreter
- Transform `think`/`ask` blocks into ACP prompt requests
- Extract LLM responses back into Patchwork variables
- Forward non-code prompts unchanged

**Structure:**
- Binary using `sacp-proxy` framework for ACP proxy implementation
- Handles prompt requests by detecting code mode vs pass-through
- Manages session-scoped interpreter storage
- Bridges between ACP messages and interpreter suspend/resume

### 2. patchwork-eval (Library Crate)

**Responsibilities:**
- In-memory Patchwork interpreter
- AST evaluation (uses `patchwork-parser` directly - shared AST)
- Runtime state management (variables, loop contexts)
- Shell command execution (via `std::process::Command`)
- Suspend execution on `think`/`ask` blocks
- Resume with LLM response values

**Parser Integration:**
- Depends on `patchwork-parser` crate from compiler
- Walks the same AST nodes (expressions, statements, types)
- Benefits from existing spans/locations for error messages
- Syntax changes automatically propagate to interpreter
- Single source of truth for Patchwork grammar

**Key API:**
```rust
pub struct Interpreter {
    runtime: Runtime,
    state: ControlState,
}

pub enum ControlState {
    Eval,
    Yield { op: LlmOp, prompt: String, bindings: Bindings, expect: Type },
    Return(Value),
    Throw(Value),
}

pub enum LlmOp {
    Think,
    Ask,
}

impl Interpreter {
    pub fn new() -> Self;
    pub fn eval(&mut self, code: &str) -> Result<ControlState>;
    pub fn resume(&mut self, value: Value) -> Result<ControlState>;
}
```

### 3. Runtime Implementation

**Interpreter runtime** (part of patchwork-eval):
- Implements builtin functions (`cat()`, etc.) in Rust
- Executes shell commands directly via `std::process::Command`
- Manages file I/O in-process
- Conceptually equivalent to compiler's JavaScript runtime, but native

**No shared runtime crate**: The compiler's runtime (JavaScript file + codegen module) stays in `patchwork-compiler`. The interpreter's runtime (Rust implementations) lives in `patchwork-eval`. They share *semantics* but not code.

## Message Flow

### 1. User Sends Patchwork Code

```json
{
  "method": "prompt",
  "params": {
    "messages": [
      {
        "role": "user",
        "content": [{"type": "text", "text": "{ for var interview in ($ ls -1 ./) { ... } }"}]
      }
    ]
  }
}
```

### 2. Proxy Detects Code Mode

The proxy intercepts prompt requests from the editor:

- If the user input starts with `{`, treat as Patchwork code and execute via interpreter
- Otherwise, forward the prompt unchanged to the successor agent
- All other ACP requests (initialize, newSession, etc.) are forwarded transparently

### 3. Interpreter Executes Until Suspension

When executing Patchwork code:

1. Check if session already has active evaluation (error if yes)
2. Create new interpreter instance
3. Call `interpreter.eval(code)` which returns an `ControlState`:
   - **Yield**: Hit a `think`/`ask` block - needs LLM
   - **Return**: Code finished without needing LLM
   - **Throw**: Runtime exception occurred

4. On suspension:
   - Interpolate variables into prompt text
   - Add type hint formatting instructions
   - Store interpreter state (session-scoped)
   - Forward augmented prompt to successor agent

5. On completion or error:
   - Return appropriate ACP response

### 4. Agent Processes Think Block

The agent (e.g., claude-code-acp) receives a normal prompt request:

```json
{
  "method": "prompt",
  "params": {
    "messages": [
      {
        "role": "user",
        "content": [{
          "type": "text",
          "text": "interview-001/transcript.txt is a transcript of an interview about the Rust programming language on 2024-03-15. The interviewer is Jane Doe and the interviewees are John Smith, Alice Johnson.\n\nRead the transcript, correct misspellings of the participants' names, and remove filler words like \"um\" and \"uh.\"\n\nRespond with a string value. Format your response as:\n```text\nyour response here\n```"
        }]
      }
    ]
  }
}
```

### 5. Proxy Extracts Response and Resumes

When the agent responds to the think block:

1. Retrieve stored interpreter state for this session
2. Extract typed value from agent response:
   - Look for markdown code fence with appropriate language marker (` ```text`, ` ```json`)
   - Parse content between fence markers
   - Fallback to full response text if no markers found
3. Resume interpreter with extracted value
4. Handle next state:
   - **Yield**: Another think/ask block - store state and repeat cycle
   - **Return**: All done - clean up session state, return success
   - **Throw**: Clean up session state, return error response

## Design Principles

### In-Memory Execution

**Avoid filesystem pollution**: The interpreter keeps all state in memory. Shell commands execute directly (unavoidable for `$ ls`), but intermediate values stay in Patchwork variables.

**No IPC machinery**: Unlike the compiled plugin approach (which uses filesystem mailboxes and Claude Code's plugin runtime), the interpreter runs as a synchronous Rust process that suspends on LLM calls.

### Variable Interpolation

**Decision**: Pre-interpolate variables before sending to LLM

When a `think { }` block contains variable interpolation, the interpreter evaluates all `${...}` expressions and produces plain text:

```patchwork
var date = "2024-03-15"
var interviewer = "Jane Doe"
var interviewees = ["John Smith", "Alice Johnson"]

var result = think {
  The interview on ${date} was conducted by ${interviewer}
  with interviewees $@{interviewees}.
}
```

Agent receives:
```
The interview on 2024-03-15 was conducted by Jane Doe
with interviewees John Smith, Alice Johnson.
```

**Rationale:**
- Natural reading flow for LLM
- Matches standard template string behavior
- Simple implementation (string substitution)
- Agent doesn't need Patchwork syntax knowledge
- Array spread `$@{...}` joins with comma-space

**Array interpolation**:
- `${array}` → JSON representation: `["a", "b", "c"]`
- `$@{array}` → Spread join: `a, b, c`

### Response Extraction with Guided Format

**Decision**: Use marker-based extraction with type-specific formatting instructions

When suspending on `think { }` with a typed variable, the proxy augments the prompt with instructions to format the response for clean extraction.

#### String Type

```patchwork
var sanitized: string = think {
  Read the transcript and remove filler words.
}
```

Augmented prompt:
````
Read the transcript and remove filler words.

Respond with a string value. Format your response as:
```text
your response here
```
````

Agent response:
````
Sure, here's the cleaned transcript:

```text
John Smith: So the thing about Rust is...
Alice Johnson: Right, and the ownership model...
```
````

The proxy extracts content between ` ```text` and ` ``` ` markers. If no markers found, falls back to full response text.

#### Future: Complex Types

For compound types (deferred to later phases), use JSON markers:

```patchwork
var data: { name: string, count: number } = think { ... }
```

Augmented prompt:
````
Original prompt text here.

Respond with a JSON object. Format your response as:
```json
{
  "name": "string",
  "count": number
}
```
````

**Extraction algorithm** (simple version for demo):
1. Look for code fence with type marker (` ```text`, ` ```json`)
2. Extract content between opening and closing fence
3. For string: use text as-is
4. For other types: parse as JSON
5. Fallback: use full response text if no markers found

**Rationale:**
- Clean extraction without preambles
- Familiar markdown code fence convention
- Graceful degradation if agent doesn't follow format
- Similar to tool calling response formats
- Simple to implement for demo

### Error Handling

**Exception Semantics**: Patchwork uses exception-based error handling

- **Parse errors**: Return ACP error response immediately
- **Runtime errors**: Throw exception, propagate up call stack
- **Uncaught exceptions**: Convert to brief response message as if from LLM, conveying error to user
- **Loop iteration failures**: Abort entire evaluation (exception semantics)
- **Future**: try/catch blocks for explicit error handling

**No Resource Limits**: Patchwork is Turing-complete
- No limits on loop iterations, file sizes, or shell command runtime
- User can press stop button in agent UI to cancel execution
- Abandoned evaluations cleaned up on session end

### No Permission System

Unlike Claude Code's tool approval system, Patchwork code runs with full user authority. This matches the design goal: Patchwork is an *explicit programming language* where the user wrote the code.

Shell commands execute immediately in the user's working directory. If the user writes `$ rm -rf /`, that's a programming bug, not a security issue requiring approval.

## Integration Example

### Zed Configuration

```json
{
  "agent_servers": {
    "Patchwork": {
      "default_mode": "bypassPermissions",
      "command": "/Users/dherman/.cargo/bin/sacp-conductor",
      "args": [
        "agent",
        "/Users/dherman/Code/patchwork/target/debug/patchwork-acp",
        "npx -y '@zed-industries/claude-code-acp'"
      ],
      "env": {}
    }
  }
}
```

### Usage Flow

1. User opens Patchwork agent in Zed
2. User types Patchwork code starting with `{`
3. Patchwork-acp interprets code, sends think blocks to Claude Code
4. Claude Code processes prompts, returns results
5. Patchwork-acp extracts results, continues execution
6. Final state written to files
7. User sees conversation messages only from think/ask blocks

## State Management

### Session-Scoped Interpreter Storage

**Decision**: One active evaluation per ACP session

The proxy maintains a `HashMap<SessionId, Interpreter>` to track suspended evaluations. Each session can have at most one active Patchwork evaluation.

**Rationale:**
- Avoids implicit concurrency (aligns with scripting language philosophy)
- Prevents filesystem conflicts from concurrent writes
- Simple, predictable behavior
- Easy cleanup on session end

**Behavior:**
- If user sends new Patchwork code while an evaluation is suspended, return error:
  ```
  Error: Cannot start new Patchwork evaluation while another is in progress
  ```
- On completion or error, clean up session state immediately
- On session end (user closes agent), clean up any suspended evaluations

**Future consideration**: As Patchwork evolves, we may revisit this to support a stack of nested evaluations, as well as explicit concurrency primitives. For now, we are limiting the design to the minimal functionality necessary for a first demo.

### Proxy State Structure

```rust
struct PatchworkProxy {
    // Session-scoped interpreter storage
    active_evaluations: Arc<Mutex<HashMap<SessionId, Interpreter>>>,
}

impl PatchworkProxy {
    fn has_active_evaluation(&self, session_id: SessionId) -> bool;
    fn store_interpreter(&self, session_id: SessionId, interp: Interpreter);
    fn retrieve_interpreter(&self, session_id: SessionId) -> Result<Interpreter>;
    fn remove_interpreter(&self, session_id: SessionId);
}
```

## Future Extensions

### Streaming Results

For long-running loops, stream progress back to editor:

```patchwork
for var interview in interviews {
  var sanitized = think { ... }
  print("Processed $interview")  // Sends progress notification
  cat(sanitized) > "$interview/sanitized.txt"
}
```

### Ask Blocks

Implement interactive prompts that require user input:

```patchwork
var confirm = ask {
  About to process ${len(interviews)} interviews. Continue?
}
if confirm {
  // ... proceed
}
```

### Standard Library

Expand beyond `cat()`:
- `json.parse()`, `json.stringify()`
- `string.trim()`, `string.split()`
- Array operations: `map()`, `filter()`, `reduce()`
- File operations: `read()`, `write()`, `exists()`

### Debugging Support

- Breakpoints via `debug()` function
- Inspector protocol for live variable inspection
- Step-through execution

## Open Questions

None currently - initial design decisions documented above.
