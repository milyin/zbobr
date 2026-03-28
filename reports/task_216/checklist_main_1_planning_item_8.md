In `zbobr-task-backend-fs/src/fs.rs`, replace the `"pr_url"` string literals with `PARAM_PR_URL` from `zbobr-api`.

Import the constant from `zbobr_api` at the top of the file.

Why: Same consistency requirement — all parameter keys should reference named constants rather than inline strings.