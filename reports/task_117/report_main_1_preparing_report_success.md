Findings from task description:
- Task: "do not pass githhub token in url"
- Problem: GitHub token is currently included directly in URLs; should be passed via environment variable to avoid leakage in logs.
- No repository or branch specified in task text.

Derived work_branch_postfix: pass-token-via-env

configure_worktree result:
- destination_repository: milyin/zbobr
- destination_branch: main
- work_branch: zbobr_fix-117-pass-token-via-env

Notes:
- Implementation should replace URL-embedded token usage with environment variable access (e.g., process.env.GITHUB_TOKEN) and ensure all scripts, CI configs, or helpers are updated. Also update documentation and secrets handling in CI where applicable.