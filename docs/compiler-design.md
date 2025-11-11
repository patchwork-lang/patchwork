# Patchwork Compiler Design

## Overview

The Patchwork compiler transforms Patchwork source code into executable agent systems. For the MVP, it targets Claude Code plugins, with a design that allows future expansion to other agent frameworks and CLI tools.

## Core Concepts

### Dual-Mode Execution Model

Patchwork programs execute in two modes that interleave and block on each other:

- **Prompt Mode**: Markdown-formatted instructions executed by an LLM agent
  - `think { }` blocks: Agent performs reasoning and returns structured results
  - `ask { }` blocks: Agent interacts with the user and returns responses

- **Code Mode**: JavaScript-like imperative code with shell integration
  - Variables, control flow, functions
  - Shell command execution and substitution
  - Message passing between workers

The key insight: **Code blocks on prompts, prompts reference code variables**. This creates a unified execution model where computation and reasoning interleave seamlessly.

### Workers and Sessions

**Workers** are the fundamental unit of concurrent execution. Each worker:
- Runs in its own process
- Has access to a shared session context
- Communicates via message passing through mailboxes
- Can execute both code and prompt modes

**Sessions** provide the coordination infrastructure:
- Unique session ID and timestamp
- Isolated working directory (in tmp)
- Mailroom with named mailboxes for worker communication
- Created by the `delegate()` method on the Agent trait

### Plugin Model

A **plugin** is defined by a trait that inherits from `Agent`. The trait:
- Declares the plugin's public interface
- Annotates methods with `@skill` or `@command` to create entry points
- Calls `self.delegate()` to spawn worker sessions
- Can contain `think { }` blocks for prompt-based reasoning

## Compilation Model

### Entry Point

The user specifies a trait inheriting from `Agent` as the compilation entry point. This trait becomes the plugin interface.

### Annotation-Driven Code Generation

- `@skill`: Generates a Claude Code skill entry point
- `@command`: Generates a Claude Code slash command entry point

Each entry point:
1. Extracts method parameters from the Patchwork signature
2. Generates wrapper code to invoke the compiled method
3. Provides proper error handling and session cleanup

### Target Outputs

For Claude Code plugins, compilation produces:

**Markdown Documents** (from prompt mode):
- Each `think { }` or `ask { }` block compiles to a separate markdown file
- Files are static and human-readable
- Support variable interpolation via placeholder syntax
- Accessible to skill documents for context and instructions

**JavaScript Modules** (from code mode):
- Worker definitions become JS modules
- Control flow, variables, and expressions map to JS
- Shell commands integrate via child process execution
- Message passing uses IPC mechanisms

### Variable Capture in Prompts

Prompt blocks can reference variables from their lexical scope:

```patchwork
var description = "Add OAuth support"
var build_cmd = "cargo check"

think {
    The user wants to ${description}.
    Use ${build_cmd} to validate the build.
}
```

**Compilation strategy**:

1. **Compile-time**:
   - Parse the prompt block to identify all `${variable}` references
   - Validate that referenced variables are in scope (compile error if not)
   - Generate markdown template with interpolation markers intact
   - Create a prompt descriptor listing required variable bindings

2. **Runtime**:
   - When JS execution reaches the `think { }` block
   - Extract current values of all referenced variables
   - Send IPC message: `{ template_id, bindings: { description: "...", build_cmd: "..." } }`
   - Agent loads markdown template, interpolates values, executes prompt
   - Returns result to waiting JS code

This keeps markdown docs static and readable while enabling dynamic variable injection at runtime.

## Type System

For the MVP, Patchwork uses a **loose, dynamic type system** that leverages JavaScript's flexibility:

- **Optional type annotations**: Variables and parameters can have explicit types
- **Structural types**: Inline object shapes for messages and data structures
- **Type inference**: Basic inference for assignments and returns
- **Union types**: For variants like `"success" | "error"`
- **No strict validation**: Type annotations serve as documentation; runtime is dynamically typed

**Rationale**: Start simple, iterate toward precision in future versions. The dynamic target (JavaScript) makes this a natural fit for the MVP.

**Scoping**: Patchwork is lexically scoped. Variables, functions, and type declarations follow standard block scoping rules.

## Runtime Architecture

### Execution Coordination

The runtime provides the bridge between prompt mode and code mode:

1. **JS Process**: Long-lived process executing code mode
2. **IPC Protocol**: Bidirectional communication with the agent
3. **Prompt Execution**: When JS encounters `think { }` or `ask { }`:
   - Sends IPC request to agent with prompt template ID and variable bindings
   - Blocks waiting for response
   - Agent loads template, interpolates variables, executes prompt
   - Sends result back via IPC
   - JS continues with returned value

### Agent Trait and delegate()

The `Agent` trait is the runtime's contract with the plugin:

```patchwork
trait Agent {
    fun delegate(workers: [Worker]): Session
}
```

Backend-specific implementations of `delegate()`:
- Create a session object (ID, timestamp, working directory, mailroom)
- For Claude Code: Use IPC to inform the main agent to spawn subagents
- Each worker descriptor includes its entry point and configuration
- Returns a session handle for the calling code to await

### Message Passing

Workers communicate through **mailboxes**:
- Named channels accessed via `self.session.mailbox.{name}`
- `send(message)`: Non-blocking message send
- `receive(timeout)`: Blocking receive with timeout (in milliseconds)
- FIFO ordering guarantees
- Messages are dynamically typed (leveraging JS runtime)

### Session Context

Each worker has access to `self.session`:
- `id`: Unique session identifier
- `timestamp`: Session creation time (ISO 8601 string)
- `dir`: Isolated working directory path
- `mailbox`: Mailroom for accessing named mailboxes

## Shell Integration

Shell commands are first-class citizens:

- **Statement form**: `$ command args` executes and continues
- **Expression form**: `$(command args)` captures stdout as string
- **String interpolation**: Variables expand in command strings
- **Exit codes**: Accessible via `$?` after execution
- **Redirection**: Standard shell operators (`>`, `2>&1`, `|`)

**Compilation**:
- Identifies shell command boundaries in the AST
- Generates appropriate child process invocations in JS
- Handles variable substitution (preventing injection at runtime)
- Preserves shell semantics (pipes, redirects, exit codes)

**Validation**: Runtime only. Shell command strings are validated during execution, not at compile time.

## Error Handling

Patchwork provides explicit error handling via the `throw` operator:

- `throw expression`: Unary operator for raising errors
- Errors propagate up the call stack
- **Worker failures automatically fail the session** (no recovery in MVP)
- Failed workers can log status before terminating
- Session cleanup occurs even on error

Future iterations will add `try`/`catch` for error recovery.

## Future Extensibility

The design explicitly supports future backends:

**CLI Agents** (Claude Code, Cursor CLI, Codex CLI):
- Different IPC mechanisms for prompt execution
- Platform-specific session management
- Tool integration varies by platform

**Agent Frameworks** (LangGraph, CrewAI, Strands SDK):
- Workers map to framework-specific agents
- Message passing uses framework primitives
- Session management integrates with framework lifecycle

The compilation model (markdown for prompts, JS for code) remains constant. Backend-specific adapters provide the `Agent` trait implementation and runtime primitives.

## Design Principles

To maintain long-term viability:

1. **Minimal implementation details**: This design avoids specific module structures, build systems, or dependencies
2. **Backend abstraction**: The `Agent` trait and session model abstract over different execution environments
3. **Separation of concerns**: Prompt compilation, code compilation, and runtime coordination are independent phases
4. **Standard formats**: Markdown and JavaScript are universal, portable targets
5. **Progressive refinement**: Start with loose typing and simple error handling; iterate toward precision and robustness
