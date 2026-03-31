The new `sanitize_branch_postfix()` function (zbobr-dispatcher/src/cli.rs) has non-trivial logic with no test coverage. Add a `#[cfg(test)]` module with tests covering:

- Basic sanitization: "Fix login bug" → "fix-login-bug"
- Non-alphanumeric replacement: "feat: add retry/backoff" → "feat-add-retry-backoff"
- Consecutive dash collapsing: "fix---multiple---dashes" → "fix-multiple-dashes"
- Leading/trailing dash trimming: "---trimmed---" → "trimmed"
- Truncation to 50 characters (and trailing dash trim after truncation)
- Empty input → empty string
- All-special-characters input → empty string
- Unicode/non-ASCII characters replaced with dashes

The function is currently private (`fn sanitize_branch_postfix`), so tests should be in the same module or the function should be made `pub(crate)` for testing.