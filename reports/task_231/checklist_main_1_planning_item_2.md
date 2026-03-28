
## What
Update `modify_task_internal` in `zbobr-task-backend-github/src/github.rs` to fetch the task's comments and pass them to `serialize_description_full`, so the GitHub issue body includes compact comment titles in the CONTEXT section.

## Why
`serialize_description_full` now accepts comments, but the call sites in `github.rs` all pass `&[]`. The main read-modify-write loop (`modify_task_internal`) needs real comments to render the compact titles.

## Changes to `github.rs`

### `modify_task_internal`
- After fetching the task (`self.fetch_task(id).await?`), fetch comments: `let comments = self.get_task_comments_internal(id).await.unwrap_or_default();`
- Pass `&comments` to all three `serialize_description_full` calls inside this function (lines ~816, ~828, ~851).
- Use `unwrap_or_default()` (or log and default to `&[]`) so a comment-fetch failure does not abort the task update — comments are display-only.

### `create_task` (line ~1328)
- New tasks have no comments. Keep passing `&[]` here. No change needed.

### Other call sites of `serialize_description_full`
- Audit all remaining calls in `github.rs` and pass `&[]` for any context where comments are unavailable or irrelevant (e.g., conflict-merge paths that have no comment context available at that point).

## Trade-off
This adds one extra GitHub API call per `modify_task_internal` invocation. This is acceptable because: (a) the call is lightweight (listing comments), (b) comments are needed for accurate display, and (c) the function already does multiple API calls (fetch task, update description, apply state).

## Analog
Follow the existing `get_task_comments_internal` + `unwrap_or_default` pattern used elsewhere in the file for non-critical auxiliary data.
