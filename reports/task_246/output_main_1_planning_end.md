The plan is straightforward:

**One change** in `zbobr-task-backend-github/src/github.rs`, in `get_task_comments_internal()`: after mapping `CommentResponse` → `Comment`, filter the resulting vec to only retain comments whose `username` is in `allowed_usernames` (when that config field is `Some`).

No config changes needed — `allowed_usernames` already exists and is wired up. The GitHub comments API doesn't support server-side creator filtering, so this is a post-fetch in-memory filter, analogous to how other local filters work in the codebase.

Waiting for your approval to proceed to implementation.