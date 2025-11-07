# Test Case: parlex UTF-8 Bug Reproduction

## Hypothesis
The parlex UTF-8 column tracking bug occurs when:
1. Lexer uses mode transitions
2. Input contains multi-byte UTF-8 characters

## parlex-calc Analysis

### What They Do
- **Mode transitions**: YES (Expr ⇄ Comment)
- **Multi-byte UTF-8 in tests**: NO (all ASCII)

### Test String from nested_block_comments_are_skipped
```
"a /* outer /* inner\n */ still\n comment */ + b;"
```

All characters are single-byte ASCII. Expected spans work correctly:
- `a`: span!(0, 0, 0, 1) ✅
- comment: span!(0, 2, 2, 11) ✅
- `+`: span!(2, 12, 2, 13) ✅
- `b`: span!(2, 14, 2, 15) ✅

### Modified Test with UTF-8
If we replace the test with multi-byte UTF-8 characters:
```
"a /* outer → inner\n */ still → comment */ + b;"
```

Where `→` (U+2192) is 3 bytes but 1 column, we would expect:
- After "outer " (6 bytes, 6 columns)
- → (3 bytes, 1 column) - **parlex bug**: column += 3 instead of += 1
- " inner" (6 bytes, 6 columns)
- Total: 15 bytes but only 13 columns
- **Bug accumulates**: reported column = 15, actual = 13

This would cause span misalignment after mode transitions, similar to our analyst.pw issue.

## Conclusion

parlex-calc **does not demonstrate any workaround** for the UTF-8 bug because:
1. They use mode transitions (like us)
2. But all tests use ASCII-only content
3. They haven't encountered the bug yet

**Our diagnosis stands**: The bug is in parlex's `LexerCursor` UTF-8 column tracking, and there's **no workaround available** in the public parlex API.

## Recommendations

1. **Report bug to parlex maintainers** with reproduction case
2. **Keep our workaround** (skip invalid spans) until parlex is fixed
3. **Document limitation** in parser-workarounds.md
4. **Move forward** with semantic analysis/interpreter work
