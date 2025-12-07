# Patchwork: Behind the Stitches - MVP Implementation Plan

Implementation plan for the architecture book. Goal: explain the current architecture with minimal extraneous content.

## Scope

**In scope**: Core concepts needed to understand how the interpreter executes Patchwork code and communicates with LLMs.

**Out of scope**:
- Future features not yet implemented
- Language syntax reference (this is an architecture book, not a user guide)
- The compiler (archived)
- Detailed coverage of every builtin function

## Tasks

### Phase 1: Setup

- [x] Run `mdbook init docs/arch` to create the book structure
- [x] Configure `book.toml` with title "Patchwork: Behind the Stitches"
- [x] Enable mermaid preprocessor for diagrams
- [x] Create `SUMMARY.md` with chapter structure

### Phase 2: Foundation Chapters

- [x] **Introduction** - Core idea, why the architecture matters, book overview
- [x] **The Value System** - `Value` enum, JSON interop, type coercion
- [x] **The Runtime** - Scopes, working directory, PrintSink

### Phase 3: Execution Chapters

- [ ] **An Example Program** - Concrete Patchwork code before diving into mechanics
- [ ] **The Evaluator** - `eval_expr`, `eval_statement`, `eval_block`, exceptions

### Phase 4: LLM Integration Chapters

- [ ] **Think Blocks** - `eval_think_block`, channel architecture, blocking semantics
- [ ] **The Agent** - `AgentHandle`, redirect actor, `think_message` flow

### Phase 5: Integration Chapter

- [ ] **The ACP Proxy** - Prompt detection, evaluation spawning, print forwarding

### Phase 6: Advanced Chapter (Optional)

- [ ] **Nested Think Blocks** - Stack-based routing, recursive interplay
  - Only include if time permits and the earlier chapters are solid

### Phase 7: Review

- [ ] Read through for consistency and flow
- [ ] Verify all mermaid diagrams render correctly
- [ ] Check that code snippets match current implementation

## Chapter Priorities

If we need to cut scope, prioritize in this order:

1. **Must have**: Introduction, Example, Think Blocks, Agent
2. **Should have**: Values, Runtime, Evaluator, ACP Proxy
3. **Nice to have**: Nested Think Blocks

The "must have" chapters form a coherent story about the core innovation (deterministic code + LLM thinking). The "should have" chapters fill in important context. The "nice to have" chapter is a deep dive for advanced readers.

## Writing Guidelines

- Keep chapters concise - this is architecture documentation, not a textbook
- Use mermaid diagrams liberally to show component relationships and message flow
- Reference actual code paths (e.g., `crates/patchwork-eval/src/eval.rs`) so readers can explore
- Avoid documenting features that don't exist yet
- When showing code, use simplified/annotated versions rather than raw copies
