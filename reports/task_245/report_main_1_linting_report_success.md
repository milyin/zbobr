Commands run:
1) cargo fmt --all -- --check
Output: showed diffs for 4 files (zbobr/src/commands.rs, zbobr-api/src/context/mod.rs, zbobr-api/src/task.rs, zbobr-task-backend-fs/src/fs.rs) and exited non-zero.

2) cargo clippy --all-targets --all-features
Output: built workspace, produced warnings (collapsible_if) but completed successfully.

3) cargo fmt --all
Applied formatting changes.

4) cargo fmt --all -- --check
No diffs after formatting.

5) cargo clippy --all-targets --all-features
Completed successfully (warnings remain but no failures).

6) git add -A && git commit -m "chore: fix formatting and linting" -m "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
Result: committed 4 files changed, 19 insertions(+), 9 deletions(-).

Auto-fixes applied: formatting only via rustfmt. Remaining issues: clippy warnings (collapsible_if) that are not auto-fixed.

Files changed:
- zbobr-api/src/context/mod.rs
- zbobr-api/src/task.rs
- zbobr-task-backend-fs/src/fs.rs
- zbobr/src/commands.rs

Conclusion: All auto-fixable formatting issues resolved and committed. Clippy warnings remain and require manual review if desired.