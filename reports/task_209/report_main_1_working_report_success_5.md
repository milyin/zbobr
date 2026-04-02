# Fix: Propagate stage-title parse errors in MdContext::from_str

## Problem

`MdContext::from_str` was silently skipping malformed `- ...` lines even when preceded by a `<!-- stage -->` marker. The lower-level `MdStageTitle::from_str` correctly rejected invalid model tokens, but the higher-level context parser discarded those errors.

## Fix

In `zbobr-api/src/context/mod.rs`, added `after_stage_marker: bool` tracking:

- Set `true` when `<!-- stage -->` is encountered
- Cleared on every other non-empty line
- When `was_after_marker` is true and a `- ...` line fails `MdStageTitle` parsing, the error is propagated (not silently ignored)
- Without the marker, existing behavior is preserved: valid stage titles are parsed, invalid ones silently fall through to compact-comment/unknown-line handling

## Files Changed

- `zbobr-api/src/context/mod.rs` — tracked `after_stage_marker`, added mandatory error propagation for post-marker lines

## Tests

All 252 existing tests pass (0 failures). No new tests were added — this is a fix-only session per the review's guidance.