In `zbobr-task-backend-github/src/github.rs`, modify `get_task_comments_internal()` to filter out comments from non-authorized users.

**What to change:** After the existing `comments.into_iter().map(...)` chain that builds `Vec<Comment>`, add a `.filter()` step before `.collect()` that checks whether `self.backend_config.allowed_usernames` is set. If it is, only keep comments whose `username` is in the allowed list. If it is `None`, keep all comments (current behavior).

**Why:** The `list_tasks` method already uses the same `allowed_usernames` field to restrict which issues are fetched. Comments should follow the same authorization logic so that only comments from authorized users are included in the task context presented to agents.

**Analog:** The filtering pattern in `list_tasks` (lines ~1156–1173) — it checks `self.inner.backend_config.allowed_usernames.as_deref()` and branches accordingly. Apply the same guard in `get_task_comments_internal`, but as a `.filter()` on the iterator rather than separate API calls (since the GitHub Issues Comments API doesn't support filtering by creator).