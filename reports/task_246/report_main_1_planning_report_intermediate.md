## Plan: Disallow Comments from Non-Authorized Users

### Context
- `allowed_usernames: Option<Vec<String>>` already exists in `ZbobrTaskBackendGithubConfig`
- Task selection (`list_tasks`) already filters by this list using the GitHub API `creator` param
- Comments are fetched via `get_task_comments_internal()` in `zbobr-task-backend-github/src/github.rs` — currently returns all comments with no user filtering

### Approach
After fetching and mapping `CommentResponse` -> `Comment` in `get_task_comments_internal()`, apply a post-fetch filter: if `allowed_usernames` is `Some`, retain only comments whose `username` is in the allowed list.

GitHub's issue comments API does not support `creator` filtering, so in-memory filtering (analogous to how task title/label filtering works elsewhere) is the right approach.

### Change Required
One targeted change in `get_task_comments_internal()`: add a filter step on the resulting `Vec<Comment>` that checks `comment.username` against `self.inner.backend_config.allowed_usernames`.

No config changes needed — the field already exists and is populated correctly.