# Patchwork Lexer Design

## Overview

The patchwork lexer performs context-aware tokenization, switching between code and prompt contexts based on `think`/`ask`/`do` operators and brace matching.

## Lexical States

### Code State (default)
- Tokenizes JavaScript/bash-like syntax
- Keywords, operators, identifiers, literals
- Transitions to Prompt state on `think {` or `ask {`

### Prompt State
- Activated inside `think { ... }` and `ask { ... }` blocks
- Most content becomes a single `PromptText` token
- Transitions back to Code state on `do {`
- Tracks brace depth to handle nesting

## Token Types

### Code Context Tokens

**Keywords:**
```
import, var, if, else, for, while, await, task, skill, fun, type,
return, succeed, fail, break, self, in
```

**Operators & Punctuation:**
```
=, ==, !=, !, ||, &&, +, -, *, /, ., ->, |, &, <, >, ...,
{, }, (, ), [, ], ,, ;, :, @
```

**Context Operators:**
```
think, ask, do
```

**Literals:**
- `String`: `"..."` (with `${...}` interpolation)
- `BashSubst`: `$(...)`
- `Number`: integers and floats
- `Identifier`: variable/function names

**Other:**
- `Comment`: `# ...` to end of line
- `Whitespace`: spaces, tabs, newlines (typically ignored)

### Prompt Context Tokens

- `PromptText`: All content between `{` and matching `}`
  - Excludes `do {` which triggers state transition
  - Single token containing raw text (including markdown, whitespace, etc.)

## State Transitions

### Entering Prompt State

**From Code state:**
```
think { ...  →  emit Think, emit LBrace, switch to Prompt state (depth=1)
ask { ...    →  emit Ask, emit LBrace, switch to Prompt state (depth=1)
```

### Exiting Prompt State

**From Prompt state:**
```
do {         →  emit Do, emit LBrace, switch to Code state (depth=1)
}            →  if depth == 1: emit RBrace, switch to Code state
                else: decrement depth, include } in PromptText
```

**Critical detail:** `do` only acts as a context operator when followed by `{` (with optional whitespace between). Otherwise it's part of the prompt text.

### Nesting

States can nest arbitrarily:
```
Code → think { Prompt → do { Code → think { Prompt } Code } Prompt } Code
```

The lexer maintains a state stack and brace depth counter to track nesting levels.

## String and Bash Substitution

### String Interpolation (`${...}`)

Inside string literals in Code state:
- `${` begins an embedded expression (mini-Code context)
- `}` ends the expression and returns to string parsing
- Parser handles validation of expression content

### Bash Command Substitution (`$(...)`)

In Code state:
- `$(` begins bash command
- `)` ends bash command
- Contents treated as single token for parser to handle

## Lexer Architecture

### ALEX Specification

The lexer will be defined using parlex-gen's ALEX format:

**States:**
- `<Code>`: Default code tokenization
- `<Prompt>`: Prompt text accumulation

**Key Rules:**
```
# Code state
Think: <Code> think / \{         # lookahead for {
Ask: <Code> ask / \{             # lookahead for {
Do: <Prompt> do / \{             # only in Prompt, with lookahead

# Delimiters trigger state tracking
LBrace: <Code,Prompt> \{
RBrace: <Code,Prompt> \}

# Prompt content
PromptText: <Prompt> [^{}]+      # accumulate non-brace content
```

### State Stack Management

The lexer maintains:
- **State stack**: `Vec<State>` tracking nested contexts
- **Brace depth**: Counter for matching `{` and `}`
- **Current state**: Top of state stack

On `think {` or `ask {`:
1. Push Prompt state onto stack
2. Initialize depth = 1

On `do {` (in Prompt state):
1. Push Code state onto stack
2. Initialize depth = 1

On `}`:
1. Decrement depth
2. If depth == 0: pop state from stack

## Future Considerations

### Prompt Text Refinement

Currently `PromptText` is a single token. Future versions might need:
- Markdown structure awareness
- Inline code block detection
- Better handling of special characters

### String State

May need dedicated states for:
- String literal parsing (to handle escape sequences)
- Nested interpolation `"... ${a + "${b}"} ..."`

### Error Recovery

Initial version does minimal error handling. Future enhancements:
- Unclosed brace detection
- Invalid token recovery
- Better error messages with position tracking
