# Fix: Non-prompt multiline comment formatting regression

## Problem
`MdCompactComment::from_comment` was using `c.body.lines().next()` in the non-prompt path, dropping all comment content after the first newline. This regressed from the expected behavior where multiline bodies are joined with spaces.

## Fix (commit 9c5182f)
- Short comments: `c.body.lines().collect::<Vec<_>>().join(" ")` 
- Long comments: truncate to COMPACT_COMMENT_MAX_LEN chars, then join resulting lines with spaces + `...`
- Prompt mode unchanged: full body preserved verbatim

## Tests updated
- Renamed `compact_comment_uses_first_line_only` → `compact_comment_joins_multiline_with_spaces` with corrected assertion
- Updated `for_prompt_preserves_multiline_comment_body` non-prompt assertions to expect space-joined lines

## Verification
All 126 tests pass (54 lib + 57 dispatcher + 13 integration + others).