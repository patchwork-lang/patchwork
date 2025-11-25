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
- [x] Plugin manifest generation (for Claude Code) - **Completed in Phase 6.5**

**Success criteria**: A complete plugin (trait + workers) compiles to a valid Claude Code plugin structure with skill/command entry points. ✅

**Completion details**: See [phase6-completion-summary.md](phase6-completion-summary.md) and [phase6.5-completion-summary.md](phase6.5-completion-summary.md)

**Example input**:
```patchwork
trait Example: Agent {
    @skill example
    fun example(input: string) {
        self.delegate([worker1(input)]).await
    }
}
```

## Phase 7: Import/Export System ✅

**Goal**: Support multi-file projects with imports and exports.

**Additions**:
- [x] Module resolution
- [x] Import statement compilation
- [x] Export declarations (workers, traits, functions)
- [x] Cross-file dependency tracking

**Success criteria**: Multi-file compilation working with proper ES6 imports/exports. ✅

**Completion details**: See [phase7-completion-summary.md](phase7-completion-summary.md)

**Deferred**: Package management, external dependencies, historian example requires embedded do blocks in prompts.

## Phase 8: Type System Foundation ✅

**Goal**: Add basic type checking without full precision.

**Additions**:
- [x] Symbol table construction
- [x] Scope analysis and variable binding validation
- [x] Basic type inference (for simple cases)
- [x] Type annotation validation (check declared types are well-formed)
- [x] Compile-time error for undefined variables

**Success criteria**: Common errors (typos, undefined variables) are caught at compile time. ✅

**Deferred**: Structural type checking, union type validation, message schema validation, import resolution.

**Completion details**: See [phase8-completion-summary.md](phase8-completion-summary.md)

## Phase 9: Error Handling ✅

**Goal**: Compile `throw` expressions and ensure proper error propagation.

**Additions**:
- [x] `throw` expression compilation (already implemented in Phase 2)
- [x] Error propagation in generated JS (Promise.all fork/join semantics)
- [x] Session cleanup on errors (cleanup() in delegate finally block)
- [x] Error context in session state (filesystem-based .failed file)
- [x] Fork/join delegation with failure propagation
- [x] Cross-process failure detection via fs.watch
- [x] Mailbox operations abort on session failure

**Success criteria**: A worker that throws an error properly terminates and cleans up its session. ✅

**Completion details**: See [phase9-completion-summary.md](phase9-completion-summary.md)

## Phase 10: Shell Command Safety ✅

**Goal**: Add runtime safety mechanisms for shell commands.

**Additions**:
- [x] Variable substitution that prevents injection (via JS template literals)
- [x] Exit code handling (already implemented, verified)
- [x] Error reporting for failed commands (already implemented, verified)
- [x] Stream redirection support ($shellPipe, $shellAnd, $shellOr, $shellRedirect)

**Success criteria**: Shell commands with interpolated variables execute safely without injection vulnerabilities. ✅

**Completion details**: See [phase10-completion-summary.md](phase10-completion-summary.md)

## Phase 11: End-to-End Integration Testing

**Goal**: Validate the entire pipeline with real Claude Code plugin execution.

**Design document**: See [runtime-design.md](runtime-design.md) for complete architecture.

**Additions**:
- [x] **Filesystem-based mailboxes** - Cross-process message passing
  - [x] Directory-per-mailbox structure (`session.dir/mailboxes/{name}/`)
  - [x] Timestamp-PID filenames for atomic writes and FIFO ordering
  - [x] Message envelope with metadata (sender, recipient, timestamp, payload)
  - [x] Filesystem watching with periodic polling fallback
  - [x] Unit tests for mailbox functionality (4 new tests, 251 total passing)
- [x] **Prompt block compilation** - Think/ask blocks to skill documents
  - [x] Detect think/ask blocks during codegen
  - [x] Generate skill documents for each block (skills/{module}_{worker}_{kind}_{n}/SKILL.md)
  - [x] Replace blocks with executePrompt() calls with skill names
  - [x] Capture and pass variable bindings from lexical scope
  - [x] Skill documents include frontmatter, variable bindings section, and task content
- [x] **IPC infrastructure** - Code ↔ Prompt communication
  - [x] Implement code-process-init.js helper script
  - [x] Update executePrompt() with stdio IPC (replace mock)
  - [x] Update delegate() with Task spawning via IPC
  - [x] Add stdin reading helpers for response handling
  - [x] StdinReader class with newline-delimited JSON parsing
  - [x] sendIpcMessage() function for writing to stdout
- [x] **Manifest updates** - Plugin entry points with code process spawning
  - [x] Update SKILL.md generation for @skill entry points with code process setup
  - [x] Add IPC message handling loops to @skill entry points
  - [x] Update prompt block skill documents with IPC protocol documentation
- [x] **Integration testing**
  - [x] Compile simple test plugin (`examples/simple-test.pw`)
  - [x] Verify output directory structure (all files generated correctly)
  - [x] Compiler CLI file writing (with `-o` flag)
  - [x] Code generation bug fixes (async, shell, delegate session passing)
  - [x] std.log implementation for standard library support
  - [ ] Invoke via `claude` CLI (deferred to Phase 12)
  - [ ] Test mailbox communication across processes (deferred to Phase 12)
  - [ ] Compile full historian plugin (blocked by additional std utilities)

**Success criteria**: Compilation pipeline validated end-to-end with test plugin. ✅

**See [phase11-status.md](phase11-status.md) for detailed completion summary.**

## Phase 12: Runtime Testing and Validation

**Goal**: Validate compiled plugins execute correctly in Claude Code runtime environment. Focus on testing existing implementation rather than adding new language features.

**Strategy**: Use the integration testing framework (created in Phase 11) to validate runtime behavior with automated tests.

**Additions**:
- [x] **Minimal standard library additions**
  - [x] Implement `cat()` function for JSON serialization
  - [x] Add type checking support for `cat()`

- [x] **Mailbox communication testing**
  - [x] Create integration test with two workers communicating via mailboxes
  - [x] Verify FIFO message ordering
  - [x] Test send/receive across worker processes
  - [x] Validate filesystem-based mailbox implementation

- [x] **Think block variable interpolation testing**
  - [x] Create test with think block using variable references
  - [x] Verify IPC protocol passes variable bindings correctly
  - [x] Validate skill document receives interpolated values

- [x] **Multi-worker delegation testing**
  - [x] Test delegate() with multiple workers in fork-join pattern
  - [x] Verify session management across workers
  - [x] Test session cleanup on completion

**Success criteria**: Automated integration tests validate that compiled plugins execute correctly, with working IPC, mailboxes, and multi-worker coordination. ✅

**Completion details**: See [phase12-status.md](phase12-status.md)

**Deferred (language still exploratory)**:
- Embedded do blocks in prompts (major language feature)
- Array `.length` property (requires member access on dynamic types)
- Full historian plugin compilation (blocked by embedded do blocks)
- Shell command edge cases (negation, complex redirects, brace expansion)
- Polish and refinement (better errors, optimization, diagnostics)
- Additional language features and standard library expansion

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

## Future Work

The language is still in exploratory phase. The following are deferred until the language design stabilizes:

**Advanced Language Features**:
- Embedded do blocks in prompts (code blocks within think/ask blocks)
- Dynamic member access (e.g., array `.length` property)
- Error recovery (try/catch)
- Advanced type system features (generics, type inference refinement)
- Shell command operators (`<=`, `>=`, `!$`, complex redirects)

**Tooling & Polish**:
- Better error messages with source locations
- Optimization passes (dead code elimination, constant folding)
- Compiler diagnostics and warnings
- Debugging support (source maps, breakpoints)
- Language server protocol (IDE integration)

**Ecosystem**:
- Rich standard library (beyond minimal runtime primitives)
- Package management
- Multiple backend targets (only Claude Code currently)
- Documentation generation from annotations

**Example Programs**:
- Full historian plugin compilation (blocked by embedded do blocks and `.length`)

## Current Status

**Exploratory Phase Complete** ✅ (2025-11-24)

The Patchwork compiler has reached a functionally complete state for early experimentation:

**Compilation Pipeline**:
- ✅ Phases 1-12 complete (all planned phases)
- ✅ 251 unit tests passing
- ✅ 4 integration tests passing (greeter, mailbox, interpolation, delegation)
- ✅ End-to-end: source → compiled plugin → execution in Claude Code

**Validated Features**:
- ✅ Worker definitions with code mode (variables, control flow, shell commands)
- ✅ Prompt blocks (think/ask) with variable interpolation via IPC
- ✅ Multi-worker fork-join delegation
- ✅ Filesystem-based mailbox communication
- ✅ Session management and cleanup
- ✅ Trait-based plugin definitions with @skill/@command annotations
- ✅ Import/export system for multi-file projects
- ✅ Type checking with basic type inference
- ✅ Error handling and propagation

**What Works**:
- Test plugins compile and execute successfully
- Generated code is readable and maintainable
- Common errors caught at compile time
- Runtime behavior validated end-to-end

**What's Next**:
The language is still exploratory. Future work depends on:
1. Real-world usage and feedback
2. Language design evolution
3. Identification of critical missing features

This represents a **working prototype compiler** suitable for experimentation and language design exploration.
