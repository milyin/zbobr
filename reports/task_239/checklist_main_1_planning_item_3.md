## GitHub backend list_tasks: filter issues by `zbobr:<instance>` label

Modify `TaskBackendGithub::list_tasks()` in `zbobr-task-backend-github/src/github.rs`.

**What to change:**
- Add a `labels` query parameter to every GitHub issues API call: `("labels", format!("zbobr:{}", self.inner.backend_config.instance))`
- The GitHub REST API supports multiple filter params simultaneously, so `labels` can be combined with `creator` (used by `allowed_usernames`)
- Apply the label filter in BOTH code paths:
  1. When `allowed_usernames` is set (loop over users): add `("labels", instance_label)` to each per-user request
  2. When `allowed_usernames` is not set: add `("labels", instance_label)` to the single request

**Why:** Using the GitHub label filter server-side is efficient — it avoids fetching all issues and filtering in memory. The `zbobr:<instance>` label on an issue means "this task is assigned to this instance".

**Pattern to follow:** The `("creator", username)` filter parameter already in the `allowed_usernames` code path is the direct analog. Add `("labels", instance_label)` alongside it in the params vec.

**Note:** If `instance` is empty (e.g. unset), skip the label filter to preserve backwards compatibility. However, since validation ensures `instance` is non-empty, this is just a defensive check.