# Patchwork: Behind the Stitches

Design document for the Patchwork interpreter architecture book.

## Overview

An mdbook explaining the Patchwork interpreter architecture, inspired by Niko Matsakis's threadbare prototype documentation. The book sources will live in `docs/arch/`.

## Title

**Patchwork: Behind the Stitches**

## Inspiration

Niko's threadbare documentation (`docs/tmp/threadbare/md/`) demonstrates effective patterns:
- Concise introduction explaining the core idea and why it matters
- AST/data structures defined before diving into execution
- Concrete example shown before explaining how it runs
- Separate chapters for different perspectives (interpreter vs agent)
- Mermaid diagrams for architecture and control flow

## Chapter Structure

### 1. Introduction

**Goal**: Explain what Patchwork is and why the architecture matters.

**Content**:
- The core idea: mixing deterministic code with LLM "thinking"
- Why this execution model enables auditability, composition, and recursion
- Brief overview of what the book covers

### 2. The Value System

**Goal**: Establish the foundation - what values exist at runtime.

**Content**:
- `Value` enum: Null, String, Number, Boolean, Array, Object
- JSON interop (from_json, to_json)
- Type coercion (to_bool, to_string_value)

**Diagrams**:
- Simple type hierarchy (graph)

### 3. The Runtime

**Goal**: Explain the execution environment.

**Content**:
- Scoped variable bindings (push_scope/pop_scope)
- Working directory context
- PrintSink for output redirection

**Diagrams**:
- Scope stack visualization (graph)

### 4. An Example Program

**Goal**: Provide a concrete program before diving into execution details.

**Content**:
- A simple Patchwork program with variables, loops, and a think block
- Walk through what it does conceptually before showing how
- Similar to Niko's "An Example" chapter - ground the reader before mechanics

### 5. The Evaluator

**Goal**: How expressions and statements are evaluated.

**Content**:
- Pattern matching on AST nodes
- `eval_expr`, `eval_statement`, `eval_block`
- Exception propagation via `Error::Exception`
- Builtin functions

**Diagrams**:
- Simple expression evaluation flow (sequence diagram)

### 6. Think Blocks

**Goal**: The core innovation - blocking on LLM responses.

**Content**:
- `eval_think_block` and prompt interpolation
- Channel architecture: `ThinkRequest` and `ThinkResponse`
- How the interpreter blocks waiting for LLM

**Diagrams**:
- Interpreter blocking on channel (sequence diagram)

### 7. The Agent

**Goal**: Bridge between sync interpreter and async LLM sessions.

**Content**:
- `AgentHandle` and the channel bridge
- The redirect actor and thinker stack
- `think_message` flow
- Session creation with successor agent

**Diagrams**:
- Full think block execution (sequence diagram with multiple participants)
- Agent component architecture (graph)

### 8. The ACP Proxy

**Goal**: How Patchwork integrates with the ACP protocol.

**Content**:
- `PatchworkProxy` and prompt detection (block mode, shell shorthand)
- `run_patchwork_evaluation` spawning pattern
- Print forwarding to notifications
- Why evaluation must be spawned (avoiding deadlock)

**Diagrams**:
- Component architecture showing proxy in the message chain (graph)

### 9. Nested Think Blocks (Advanced)

**Goal**: Deep dive into the recursive interplay between interpreter and LLM.

**Content**:
- Stack-based routing in the redirect actor
- Call stack visualization at deepest nesting
- Why nested thinks work without deadlock
- The channel dance (outer think blocked while inner executes)

**Diagrams**:
- Nested execution with colored rect boxes showing call frames (sequence diagram)
- Call stack at deepest point (text diagram like Niko's)

## Key Differences from Threadbare

1. **More layers**: Patchwork has a full language (values, runtime, evaluator) vs threadbare's minimal 3-node AST
2. **ACP integration**: The proxy layer is new and worth a dedicated chapter
3. **Richer examples**: We can show actual Patchwork syntax vs JSON AST
4. **Real language features**: Variables, loops, builtins, shell commands

## Diagram Strategy

Following Niko's effective approach:

- **`graph TD`** for architecture overviews (component relationships)
- **`sequenceDiagram`** for execution flow (message passing, call/response patterns)
- **Colored `rect` boxes** in sequence diagrams to show recursive call frames
- **`Note over`** annotations to show state changes (like stack contents)

## File Structure

```
docs/arch/
├── book.toml
├── src/
│   ├── SUMMARY.md
│   ├── introduction.md
│   ├── values.md
│   ├── runtime.md
│   ├── example.md
│   ├── evaluator.md
│   ├── think-blocks.md
│   ├── agent.md
│   ├── acp-proxy.md
│   └── nested-thinks.md
```

## Audience

Developers who want to:
- Understand how Patchwork executes code
- Extend the interpreter with new features
- Debug issues in the think block / agent communication
- Understand the ACP proxy integration
