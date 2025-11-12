# Patchwork Compiler Implementation Plan

## Philosophy

This plan prioritizes **getting to a working compilation as quickly as possible**. We'll build in incremental layers, deferring complexity until we have an end-to-end pipeline working.

**Guiding principles**:
- Start with the minimal viable subset of the language
- Defer type checking and validation where possible
- Focus on codegen before optimization
- Test each layer before adding the next

## Phase 1: Core Infrastructure ✅

**Goal**: Set up the compiler pipeline structure without worrying about completeness.

**Deliverables**:
- [x] Compiler driver that orchestrates compilation phases
- [x] AST representation (already have from parser)
- [x] Basic error reporting infrastructure
- [x] Command-line interface for invoking the compiler

**Success criteria**: Can invoke the compiler on a Patchwork file and get structured output (even if it's just a debug dump).

**Deferred**: Code generation, semantic analysis, type checking.

## Phase 2: Simple Worker Codegen (Code Mode Only) ✅

**Goal**: Compile a single worker with only code mode features to executable JavaScript.

**Subset of language**:
- [x] Variable declarations and assignments
- [x] Basic expressions (arithmetic, string concatenation, member access)
- [x] Function calls
- [x] Control flow (if, while, for)
- [x] Shell commands (both statement and expression forms)
- [x] Return statements

**Explicitly excluded** (for now):
- Prompt blocks (`think { }`, `ask { }`)
- Message passing (mailboxes)
- Session context (`self.session`)
- Type annotations (parse but ignore)
- Imports/exports

**Success criteria**: A simple worker with variables, conditionals, and shell commands compiles to runnable JS that executes correctly. ✅

**Example input**:
```patchwork
worker example() {
    var x = 5
    var y = $(echo "hello")
    if x > 3 {
        $ echo "x is big"
    }
}
```

**Generated output**:
```javascript
export function example() {
  let x = 5;
  let y = await $shell(`echo hello`, {capture: true});
  if (x > 3) {
    await $shell(`echo x is big`);
  }
}
```

## Phase 3: Session Context and Runtime Primitives ✅

**Goal**: Add the runtime infrastructure that workers need to interact with their environment.

**Additions**:
- [x] `self.session.{id, timestamp, dir}` context object
- [x] Runtime library with session management
- [x] IPC protocol scaffolding (even if not fully functional yet)

**Success criteria**: Workers can access session context and the generated code includes proper runtime imports. ✅

**Deferred**: Full IPC implementation, mailboxes, actual subagent spawning.

**Completion details**: See [phase3-completion-summary.md](phase3-completion-summary.md)

## Phase 4: Prompt Block Compilation ✅

**Goal**: Compile `think { }` and `ask { }` blocks to markdown files and generate the runtime coordination code.

**Additions**:
- [x] Parse prompt block contents as markdown
- [x] Extract variable references via lexical analysis
- [x] Generate markdown template files
- [x] Generate JS code that sends IPC requests with variable bindings
- [x] Implement the blocking behavior (await IPC response)

**Success criteria**: A worker with a `think { }` block compiles to JS + markdown, and the JS code properly captures variables and sends them via IPC. ✅

**Example input**:
```patchwork
worker example() {
    var name = "Claude"
    think {
        Say hello to ${name}.
    }
}
```

**Deferred**: Actual IPC transport implementation (can mock for testing).

**Completion details**: See [phase4-completion-summary.md](phase4-completion-summary.md)

## Phase 5: Message Passing Between Workers ✅

**Goal**: Enable workers to communicate via mailboxes.

**Additions**:
- [x] Mailbox access via `self.session.mailbox.{name}`
- [x] `send()` and `receive()` method compilation
- [x] Message serialization/deserialization
- [x] Mailroom infrastructure in the runtime

**Success criteria**: Multiple workers can send and receive messages in a compiled program. ✅

**Deferred**: Advanced message patterns, type validation of messages.

**Completion details**: See [phase5-completion-summary.md](phase5-completion-summary.md)

## Phase 6: Trait Definitions and Plugin Entry Points ✅

**Goal**: Support the plugin model with traits and annotation-driven entry point generation.

**Additions**:
- [x] Trait declarations with `Agent` inheritance
- [x] Method definitions in traits
- [x] `@skill` and `@command` annotation parsing
- [x] `self.delegate()` compilation
- [x] Array and object destructuring support
- [ ] Plugin manifest generation (for Claude Code) - **Deferred to Phase 6.5**

**Success criteria**: A complete plugin (trait + workers) compiles to a valid Claude Code plugin structure with skill/command entry points. ✅ (Core compilation working; manifest generation deferred)

**Completion details**: See [phase6-completion-summary.md](phase6-completion-summary.md)

**Example input**:
```patchwork
trait Example: Agent {
    @skill example
    fun example(input: string) {
        self.delegate([worker1(input)]).await
    }
}
```

## Phase 7: Import/Export System

**Goal**: Support multi-file projects with imports and exports.

**Additions**:
- [ ] Module resolution
- [ ] Import statement compilation
- [ ] Export declarations (workers, traits, functions)
- [ ] Cross-file dependency tracking

**Success criteria**: The historian example (4 files with imports) compiles successfully.

**Deferred**: Package management, external dependencies.

## Phase 8: Type System Foundation

**Goal**: Add basic type checking without full precision.

**Additions**:
- [ ] Symbol table construction
- [ ] Scope analysis and variable binding validation
- [ ] Basic type inference (for simple cases)
- [ ] Type annotation validation (check declared types are well-formed)
- [ ] Compile-time error for undefined variables

**Success criteria**: Common errors (typos, undefined variables) are caught at compile time.

**Deferred**: Structural type checking, union type validation, message schema validation.

## Phase 9: Error Handling

**Goal**: Compile `throw` expressions and ensure proper error propagation.

**Additions**:
- [ ] `throw` expression compilation
- [ ] Error propagation in generated JS
- [ ] Session cleanup on errors
- [ ] Error context in IPC protocol

**Success criteria**: A worker that throws an error properly terminates and cleans up its session.

## Phase 10: Shell Command Safety

**Goal**: Add runtime safety mechanisms for shell commands.

**Additions**:
- [ ] Variable substitution that prevents injection
- [ ] Exit code handling
- [ ] Error reporting for failed commands
- [ ] Stream redirection support

**Success criteria**: Shell commands with interpolated variables execute safely without injection vulnerabilities.

## Phase 11: End-to-End Integration Testing

**Goal**: Validate the entire pipeline with real Claude Code plugin execution.

**Additions**:
- [ ] Full IPC transport implementation (not mocked)
- [ ] Claude Code plugin runtime integration
- [ ] Session management with actual subagent spawning
- [ ] Complete mailroom implementation

**Success criteria**: The compiled historian plugin runs successfully in Claude Code and rewrites git commits.

## Phase 12: Polish and Refinement

**Goal**: Improve developer experience and code quality.

**Additions**:
- [ ] Better error messages with source locations
- [ ] Optimization passes (dead code elimination, constant folding)
- [ ] Generated code formatting and readability
- [ ] Compiler diagnostics and warnings
- [ ] Documentation generation from annotations

**Success criteria**: The compiler produces helpful errors and generates clean, readable output.

## Testing Strategy

**Per-phase testing**:
- [ ] Unit tests for each compilation pass
- [ ] Golden file tests comparing generated output to expected output
- [ ] Integration tests for each new feature

**Continuous validation**:
- [ ] Keep the historian example compiling at all times (starting from Phase 6)
- [ ] Run generated JS through a linter
- [ ] Validate generated markdown is well-formed

**Regression prevention**:
- [ ] Add tests for bugs as they're discovered
- [ ] Maintain a test suite that exercises all language features

## Non-Goals for MVP

These are explicitly deferred to post-MVP iterations:

- Advanced type system features (generics, type inference refinement)
- Optimization (beyond basic readability)
- Error recovery (try/catch)
- Multiple backend targets (only Claude Code for MVP)
- Package management
- Debugging support (source maps, breakpoints)
- Language server protocol (IDE integration)
- Standard library beyond runtime primitives

## Success Criteria for MVP

The MVP is complete when:

- [ ] The historian example compiles without errors
- [ ] The generated plugin loads in Claude Code
- [ ] Running the plugin successfully rewrites git commits
- [ ] The generated code is readable and maintainable
- [ ] Common errors are caught at compile time

This represents a **functionally complete but unpolished** compiler suitable for early testing and iteration.
