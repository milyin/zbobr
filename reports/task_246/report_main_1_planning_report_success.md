## Plan: Disallow Comments from Non-Authorized Users

### Approach
Modify `get_task_comments_internal()` in `zbobr-task-backend-github/src/github.rs` to filter out comments from users not in `allowed_usernames`.

### Key decision
Apply filtering as an iterator `.filter()` step after fetching all comments from GitHub — since the GitHub Issues Comments API does not support filtering by creator, unlike the Issues list API. This is the minimal, correct approach.

### Analog
The `list_tasks` method already uses `backend_config.allowed_usernames` to restrict which issues are fetched. The same field and same branching pattern (Some → filter, None → allow all) applies here.

### Checklist
- Filter comments by allowed_usernames in `get_task_comments_internal()` — add a `.filter()` on the iterator that keeps only comments whose `username` is in the allowed list when `allowed_usernames` is set.