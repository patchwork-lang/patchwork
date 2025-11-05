# Parser & Lexer Workarounds

This document tracks all workarounds implemented to handle edge cases and bugs. Each workaround masks an underlying issue that should eventually be fixed properly.

## Active Workarounds

### 1. Invalid Span Skipping in Adapter
**File:** `crates/patchwork-parser/src/adapter.rs:208-213`

**Issue:** Lexer produces tokens with invalid spans (start > end) in Prompt mode with string interpolation.

**Workaround:**
```rust
// Workaround for lexer span tracking bug in prompt mode with interpolation
// If we get an invalid span, skip this token and try the next one
if start > end {
    eprintln!("Warning: Skipping token {:?} with invalid span {}..{}", token.rule, start, end);
    continue;
}
```

**Root Cause:** Lexer span tracking breaks when transitioning between modes (Code → Prompt → InString) with interpolation like `${work_dir}` inside think/ask blocks, **combined with multi-byte UTF-8 characters**. Specifically triggered by:
- Multiple mode transitions in sequence (think → ask → think)
- Backtick strings with interpolation in Prompt mode
- Multi-byte UTF-8 characters (e.g., `→` U+2192, 3 bytes) in the text
- Complex: `think { } || ask { }` followed by `think { backtick-string-with-${interpolation} }` with UTF-8 chars

**Reproduction:** analyst.pw lines 60-79 shows the pattern:
```
} || ask {
    ...
}

var commit_plan = think {
    Read `${work_dir}/master.diff` ...
    - Follows a progression (infrastructure → core → features → polish → tests/docs)
```

After multiple Prompt mode transitions, line 79 (containing 4 × `→` characters, 3 bytes each = 12 bytes but 4 columns) causes Newline token to have backwards span (start=2781, end=2774).

**Technical Details:** The `→` character (U+2192) is 3 bytes in UTF-8 (bytes: `e2 86 92`). Line 79 has 4 of these, adding 12 bytes but only 4 to the column count. The lexer's position tracking uses column-based positions but span start/end are byte offsets. After mode transitions, the conversion between column positions and byte offsets gets corrupted when multi-byte characters are present. The position accumulator goes negative, producing backwards spans.

**Proper Fix:** Fix parlex's UTF-8 position tracking during mode transitions. The issue is in how `lexer.span()` converts between line/column positions and byte offsets when mode transitions occur. This is likely a parlex framework bug rather than our lexer code. Options:
1. Patch parlex to fix UTF-8 byte offset tracking across mode changes
2. Switch to a different lexer that handles UTF-8 correctly
3. Work around by normalizing UTF-8 characters in input (unacceptable - loses user content)
4. Accept the workaround (skip invalid spans) as good enough

**Impact:**
- Silently skips malformed tokens (logs warning to stderr)
- May cause parser to see incomplete token stream
- Affects analyst.pw (1 Newline token skipped at line ~80)
- Only triggers with specific complex patterns, not all interpolation

**Test:** analyst.pw exhibits the bug, simplified tests don't reproduce it

---

### 2. Defensive Span Validation
**File:** `crates/patchwork-parser/src/adapter.rs:86-89`

**Issue:** Panic on invalid spans with confusing error message.

**Workaround:**
```rust
// Defensive check for invalid spans
if start > end {
    panic!("Invalid token span for {:?}: start={} > end={}", rule, start, end);
}
```

**Root Cause:** Same as #1 - lexer produces invalid spans.

**Proper Fix:** Fix lexer or remove after #1 is properly fixed.

**Impact:** Better error messages for debugging, but still panics.

---

### 3. Example File Modifications
**Files:** `examples/historian/scribe.pw`

**Issue:** Lexer keyword ambiguity - `while(` tokenizes as `IdentifierCall` instead of `while` + `(`.

**Workaround:** Added space in source file: `while(true)` → `while (true)`

**Root Cause:** Alex lexer uses longest-match rule. `{{ID}}\(` matches `while(` (6 chars) better than just `while` (5 chars). Keywords need to be checked before IdentifierCall pattern.

**Proper Fix:**
- Option A: Lexer keyword disambiguation using reserved word list
- Option B: Make IdentifierCall pattern explicitly exclude keywords
- Option C: Grammar accepts both patterns and handles in semantic analysis

**Impact:**
- Requires specific formatting in source files
- `if(`, `while(`, `for(` won't parse without spaces
- Inconsistent with common programming style

**Files affected:** scribe.pw (changed line 20)

---

### 4. Comment Style Changes
**Files:** `examples/historian/scribe.pw`

**Issue:** Language only supports `#` comments, not `//` comments.

**Workaround:** Changed `//` to `#` in example files.

**Root Cause:** Lexer only has `#[^\n]*` pattern, no `//` pattern.

**Proper Fix:** Either:
- Add `//` comment support to lexer (if desired)
- Or document that only `#` style is supported

**Impact:**
- Minor - just consistency
- Developers need to use `#` not `//`

**Files affected:** scribe.pw lines 16, 21

---

## Known Issues Without Workarounds

### 5. Code Fences in Prompts
**Test:** `test_parse_historian_analyst` (ignored)

**Issue:** Code fences like ` ```javascript ` in prompts contain `{` `}` that lexer treats as patchwork tokens instead of text.

**Root Cause:** PromptText pattern `[^{}\s\$]+` explicitly excludes braces. In prose or code examples, braces should be allowed.

**No Workaround:** Cannot fix without lexer changes.

**Proper Fix:**
- Option A: Change PromptText to allow braces: `[^\s\$]+` (but then how to detect `do {`?)
- Option B: Implement fence-aware lexing (complex state machine)
- Option C: Use different syntax for code examples in prompts

**Impact:** analyst.pw cannot parse due to JavaScript code example at line 91.

---

### 6. Multi-line Ask Blocks
**Test:** `test_parse_historian_scribe` (fails)

**Issue:** Multi-line `ask` blocks fail - lexer doesn't stay in Prompt mode correctly across newlines.

**Root Cause:** Lexer mode state management issue with newlines in Prompt mode.

**No Workaround:** Cannot fix without lexer changes.

**Proper Fix:** Fix lexer state machine to properly maintain Prompt mode across newlines.

**Impact:** scribe.pw cannot parse due to multi-line ask block at lines 61-73.

---

## Historical Workarounds (Resolved)

### Removed { } from Shell Mode
**Fixed in:** Commit "Complete M10 Task 7"

**Issue:** `HEAD^{tree}` in shell commands was tokenizing `{tree}` as separate tokens.

**Solution:** Removed `{` and `}` from Shell mode special tokens, made them part of `ShellArg`.

**Status:** ✅ Properly fixed, not a workaround.

---

## Summary Statistics

- **Active workarounds:** 4
- **Known issues without workarounds:** 2
- **Files passing with workarounds:** 2/4 historian files (main.pw, narrator.pw)
- **Files failing:** 2/4 (analyst.pw, scribe.pw)
- **Test status:** 95 passing, 1 failing, 1 ignored

---

## Recommendations

**Priority 1 - Blocking 2 Files:**
1. Fix lexer span tracking in Prompt mode (#1, #2)
2. Fix lexer Prompt mode state across newlines (#6)

**Priority 2 - Quality of Life:**
3. Fix keyword ambiguity (#3) - affects code style
4. Decide on `//` comment support (#4) - affects syntax consistency

**Priority 3 - Advanced Features:**
5. Code fences in prompts (#5) - affects documentation in prompts

**Long-term:**
- Consider lexer rewrite with better mode handling
- Add comprehensive lexer tests for mode transitions
- Document lexer mode state machine formally
