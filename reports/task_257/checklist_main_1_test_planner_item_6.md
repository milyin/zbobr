# Multi-line comment body preserved in for_prompt mode

**Priority: Medium**
**File:** `zbobr-api/src/context/mod.rs` (in `mod tests`)

## Rationale

The `for_prompt=true` path in `MdCompactComment::from_comment` preserves the full comment body via `format!("user {}: {}", username, c.body)`. However, existing tests only use single-line comment bodies:
- `for_prompt_true_uses_compact_comment_format`: uses "a user comment" (single line)
- `for_prompt_true_does_not_truncate_long_comment_text`: uses "a".repeat(100) (single long line)
- `md_compact_comment_display_for_prompt`: uses "please proceed" (single line)

No test verifies behavior with a multi-line body like `"first line\nsecond line\nthird line"` in prompt mode.

## What to test

Create a test `for_prompt_preserves_multiline_comment_body` that:

1. Creates a multi-line comment (e.g., "proceed with plan\nalso fix the bug\nand update docs")
2. Serializes with `for_prompt=true`
3. Asserts the full body appears in the output (all lines present)
4. Contrasts with the non-prompt behavior which only shows the first line

## Why this matters

The non-prompt path was specifically changed to use first-line-only extraction. If someone accidentally applies the same logic to the for_prompt path, this test would catch it. Additionally, it documents the intentional design difference between prompt (full body) and non-prompt (first line) rendering of multi-line comments.
