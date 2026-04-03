## Summary

All checklist items completed in a single commit (`f059b315`).

### Changes made

**`zbobr-dispatcher/src/prompts.rs`**
- Added public `sample_task_and_comments()` function with non-trivial field values:
  - `pr_url`: `Some("https://github.com/example/repo/pull/42")`
  - `context`: `TaskContext` with a `StageContext` containing a `ContextRecord` of type `Success`
  - `signal`: `Some(Signal::Go(Stage::new("working")))`
  - `stack`: one `StackEntry` (pipeline "parent", signal Go("done"))
  - comment `url` fields populated with GitHub comment URLs
- Updated `validate_all_prompts` to call `sample_task_and_comments()` instead of its own inline Task construction with empty values

**`zbobr-dispatcher/src/lib.rs`**
- Exported `sample_task_and_comments` in the `pub use prompts::{...}` block

**`zbobr/src/commands.rs`**
- Removed the old private `dummy_task_and_comments` function
- Replaced all 3 call sites with `sample_task_and_comments()` from `zbobr_dispatcher`
- Cleaned up now-unused imports

Build: clean (`cargo build` and `cargo test` both pass with no warnings).