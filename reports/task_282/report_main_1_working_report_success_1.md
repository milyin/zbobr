## Fix: Canonical strings in sample_task_and_comments

### Issue
The review (ctx_rec_6) found that `sample_task_and_comments()` in `zbobr-dispatcher/src/prompts.rs` used:
- `"claude".to_string()` instead of the canonical `Tool::CLAUDE` constant
- Repeated `"https://github.com/example/repo/issues/1"` string literal across 3 fields

### Changes made

**`zbobr-dispatcher/src/prompts.rs`**
- Added `Tool` to imports from `zbobr_api::task`
- Replaced `tool: Some("claude".to_string())` with `tool: Some(Tool::CLAUDE.to_string())`
- Added two local `const` values inside the function:
  - `SAMPLE_REPO_URL: &str = "https://github.com/example/repo"`
  - `SAMPLE_ISSUE_URL: &str = "https://github.com/example/repo/issues/1"`
- Used `format!("{SAMPLE_REPO_URL}/pull/42")` for `pr_url`
- Used `format!("{SAMPLE_ISSUE_URL}#issuecomment-NNN")` for all issue comment URLs

### Verification
Build passes cleanly with no warnings (`cargo build` exit 0).

Commit: `5937732a`