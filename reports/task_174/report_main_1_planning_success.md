# Plan: Avoid GitHub issue auto-linking in technical commits

## Context
When zbobr stores reports, it creates commits with messages like `zbobr: store report ... for task #174`. GitHub auto-links `#174` to the issue, polluting the issue timeline with noise from internal/technical commits.

## Analysis
Only one location in the codebase creates git commits with GitHub-linkable task references:
- `zbobr-task-backend-github/src/github.rs:1129` — `format!("zbobr: store report {filename} for task #{task_id}")`

All other `#{task_id}` occurrences are in tracing/logging or test file content — not in git commit messages.

## Fix
Change `#{task_id}` to `# {task_id}` (add space) in the commit message format string. This preserves human readability while preventing GitHub's auto-linking.

## Verification
`cargo build` + `cargo test` — no test relies on this exact commit message format.