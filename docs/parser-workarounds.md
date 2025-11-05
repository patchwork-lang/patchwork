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

**Technical Details:** The `→` character (U+2192) is 3 bytes in UTF-8 (bytes: `e2 86 92`). Line 79 has 4 of these, adding 12 bytes but only 4 to the column count.

**Root Cause Confirmed:** Parlex framework UTF-8 bug verified via API investigation:
- Parlex `Position.column` documented as "character position in the line"
- Parlex `LexerCursor` documented as advancing "one byte at a time"
- **Bug**: `LexerCursor` increments column by bytes instead of UTF-8 characters
- Multi-byte character `→` (3 bytes) incorrectly increments column by 3 instead of 1
- After many mode transitions, accumulated error causes `column > actual line length`
- Example: reports column 96 on 80-character line
- Our `position_to_offset()` correctly uses `char_indices()` for UTF-8 conversion
- Trying to find character 96 on 80-character line → walks past line end → backwards span

**Our Usage Verified Correct:** Investigation of adapter code (lines 17-37) confirms proper UTF-8 handling. The conversion from (line, column) to byte offsets uses Rust's `char_indices()` iterator which correctly counts UTF-8 characters. The bug is entirely within parlex's `LexerCursor` column tracking.

**Verification via parlex-calc Example:** Examined the official parlex-calc example from the parlex repository:
- parlex-calc DOES use mode transitions (Expr ⇄ Comment)
- parlex-calc tests ALL use ASCII-only content (no multi-byte UTF-8)
- Their nested comment test with newlines works correctly (ASCII only)
- **No workarounds found** - they haven't encountered the bug yet
- Confirms bug only triggers with: mode transitions + multi-byte UTF-8 characters

**Temporary Workaround (Applied):** Replaced Unicode arrows (`→` U+2192) with ASCII arrows (`->`) in example files (analyst.pw:79) to avoid triggering the bug. This stopgap works for our controlled examples but doesn't solve the underlying issue for user-provided content with multi-byte UTF-8 characters.

**Proper Fix:** Fix parlex's `LexerCursor` to count UTF-8 characters instead of bytes for column positions. Options:
1. Patch parlex to fix UTF-8 byte offset tracking across mode changes
2. Switch to a different lexer that handles UTF-8 correctly
3. Work around by normalizing UTF-8 characters in input (unacceptable - loses user content)
4. Accept the workaround (skip invalid spans) as good enough

**Impact:**
- ~~Affects analyst.pw (1 Newline token skipped at line ~80)~~ **RESOLVED** by ASCII arrow workaround
- Still a potential issue for user-provided content with multi-byte UTF-8
- Silently skips malformed tokens (logs warning to stderr)
- May cause parser to see incomplete token stream

**Status:** Temporarily resolved for example files; underlying parlex bug remains

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

### 3. Comment Style Changes
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

### Keyword Ambiguity (IdentifierCall)
**Fixed in:** Commit "Eliminate IdentifierCall token"

**Issue:** `while(` tokenized as `IdentifierCall` instead of `while` + `(` due to longest-match rule.

**Solution:** Eliminated `IdentifierCall` token and bare command syntax. All shell commands now require explicit `$` prefix. Function calls use standard `identifier "(" args ")"` parsing.

**Status:** ✅ Properly fixed, not a workaround.

---

## Summary Statistics

- **Active workarounds:** 3
  1. Invalid span skipping (#1) - temporarily resolved for examples via ASCII arrows
  2. Defensive span validation (#2)
  3. Comment style (#3)
- **Known issues without workarounds:** 2
  5. Code fences in prompts - **currently blocking analyst.pw**
  6. Multi-line ask blocks - blocking scribe.pw
- **Files passing:** 2/4 historian files (main.pw, narrator.pw)
- **Files failing:** 2/4 (analyst.pw: code fence issue, scribe.pw: multi-line ask)
- **Test status:** 93 passing, 2 failing, 1 ignored

---

## Recommendations

**Priority 1 - Blocking 2 Files:**
1. ~~Fix lexer span tracking in Prompt mode (#1, #2)~~ **Temporarily resolved** with ASCII arrows
2. Fix code fences in prompts (#5) - **currently blocking analyst.pw**
3. Fix lexer Prompt mode state across newlines (#6) - blocking scribe.pw

**Priority 2 - Quality of Life:**
4. Decide on `//` comment support (#3) - affects syntax consistency

**Long-term:**
- Report UTF-8 column tracking bug to parlex maintainers or consider alternative lexer
- Add comprehensive lexer tests for mode transitions
- Document lexer mode state machine formally
