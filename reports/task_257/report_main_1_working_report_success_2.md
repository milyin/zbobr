# Fix: Review Blocking Issues

## Changes Made

### 1. `<!-- stage -->` markers suppressed in prompt mode
- Added `for_prompt: bool` field to `MdContext` struct
- In `MdContext::fmt`, the `has_compact` flag (which triggers stage marker emission) is now only `true` when `for_prompt == false`
- Prompt-mode rendering no longer includes `<!-- stage -->` markers, reducing noise

### 2. Non-prompt comment format restored
- `MdCompactComment::from_comment` now uses conditional logic:
  - `for_prompt=true`: `user {name}: {body}` (plain format, full body)
  - `for_prompt=false` short: `user:**{name}** {first_line}` (bold markdown, first line)
  - `for_prompt=false` long: `user:**{name}** {truncated}...` (bold markdown, first line truncated)
- The original `user:**name**` bold markdown format is preserved for normal (non-prompt) context rendering
- Test assertions updated from `user unknown:` to `user:**unknown**` for non-prompt tests

### 3. New test added
- `stage_marker_not_added_in_prompt_mode`: verifies `<!-- stage -->` markers are absent even when compact comments are present, when in prompt mode

## Test Results
All 52+ tests pass across all packages.

## Commit
`e1bb556` - fix: scope comment format changes to for_prompt mode, suppress stage markers in prompt output
