## Summary

Added filtering of GitHub issue comments by `allowed_usernames` in `get_task_comments_internal()` in `zbobr-task-backend-github/src/github.rs`.

## Changes

- Added a `.filter()` step before `.map()` in `get_task_comments_internal()` that checks the comment author's login against `self.backend_config.allowed_usernames`
- If `allowed_usernames` is `None`, all comments pass through (unchanged behavior)
- If `allowed_usernames` is set, only comments from listed users are returned
- Comments with no user info are excluded when filtering is active

## Analog

Followed the same pattern as `list_tasks()` which already filters issues by `allowed_usernames`.