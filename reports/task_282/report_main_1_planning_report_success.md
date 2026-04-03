## Plan Summary

The task unifies duplicate sample-data construction across two crates.

### Key decisions
- **Move to zbobr-dispatcher**: The function is placed in `zbobr-dispatcher/src/prompts.rs` (not `zbobr`) because dependency direction only allows `zbobr` → `zbobr-dispatcher`, not the reverse.
- **Enrich with non-trivial values**: `pr_url`, `context` (with a StageContext + ContextRecord), `signal` (a real Signal variant), `stack` (one StackEntry), and comment `url` fields are all populated to make validation in #266 more meaningful.
- **Analog**: Follows the existing `dummy_task` helper in the same file — same style, same scope.

### Checklist items created
1. Move/rename `dummy_task_and_comments` → `sample_task_and_comments` in `zbobr-dispatcher`, enrich fields, export from lib.rs, update `zbobr/src/commands.rs` call sites.
2. Update `validate_all_prompts` to call `sample_task_and_comments()` instead of its own inline construction.