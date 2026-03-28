## Implementation Plan: Intersperse Comments with Links in User Display Context

### Problem
- `serialize_context(ctx, comments, for_prompt=true, ...)` in `prompts.rs` → includes full blockquote comments for agents ✓
- `serialize_context(ctx, &[], for_prompt=false, ...)` in `separator.rs` → empty comments, so GitHub issue body context section shows no comments ✗

### Analog
The `MdStage::from_stage_context(stage, for_prompt, report_url)` pattern already uses the `for_prompt` flag to control display. The same flag controls compact vs. full comment rendering.

---

### Proposed Changes

#### 1. Add `comment_url` to `Comment` struct (`zbobr-api/src/task.rs`)
Add `comment_url: Option<String>` field — stores direct GitHub comment URL.

#### 2. GitHub backend: populate `comment_url` (`zbobr-task-backend-github/src/github.rs`)
Add `html_url: Option<String>` to `CommentResponse`; populate `comment_url` in `Comment` from it.

#### 3. Extend `MdUserComment` for compact rendering (`zbobr-api/src/context/mod.rs`)
- Add `comment_url: Option<String>` field
- Add `compact: bool` field (set to `!for_prompt` at build time)
- Update `From<&Comment>` to carry `comment_url`; `compact` set by caller
- `Display` → when `compact=true`, render as single line:
  `- {first_line truncated to ~80 chars}... \`{date}\` <sub>[link]({url})</sub>`
  (omit `<sub>...</sub>` if no URL; no `...` if not truncated)
  Date formatted using existing `format_timestamp()`
- `FromStr` → still parses blockquote format (compact lines are not round-tripped since they're regenerated from live comments)

#### 4. Add invisible stage marker (`zbobr-api/src/context/mod.rs`)
- Add `for_user_display: bool` to `MdContext` struct
- `MdContext::fmt` → when `for_user_display=true`, emit `<!-- stage -->\n` immediately before each stage entry
- `MdContext::from_str` → skip lines that are exactly `<!-- stage -->`
- `MdContext::from_task_context` → sets `for_user_display = !for_prompt`

#### 5. Thread comments through `serialize_description_full` (`separator.rs` + `github.rs`)
- Add `comments: &[Comment]` parameter to `serialize_description_full`
- Add `comments: &[Comment]` parameter to `merge_concurrent_description_updates` (used for internal re-serialization)
- Update all callers in `github.rs`:
  - `update_task`: fetch comments once with `get_task_comments_internal(id)` before calling serialize
  - `create_task`: pass `&[]`
  - `merge_concurrent_description_updates` internal re-serialize call: pass `comments`

#### 6. Update tests
- `separator.rs` tests: pass `&[]` to updated signature
- `context/mod.rs` tests: add tests for compact comment format and stage markers
- Ensure all existing tests still pass

---

### Key Design Decisions
- Comments are always regenerated at serialization time from live data — they're never parsed back into `TaskContext`. `parse_context` already discards `MdEntry::Comment` entries.
- The `<!-- stage -->` marker allows downstream consumers to skip comment lines when iterating stage entries, without breaking anything for those who don't look for it.
- The compact text truncation threshold: ~80 characters (first line of comment text).
