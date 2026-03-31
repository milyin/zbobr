Update all tests and configuration examples to reflect the simplified schema:

1. In `zbobr-dispatcher/src/config.rs` tests, remove references to `preparator` config entries and `default_destination_repository`/`default_destination_branch` fields.

2. In `zbobr-repo-backend-github` tests, remove `fork_owner` setup and fork-related test assertions.

3. Update any integration tests in `tests/` that create full pipeline configs.

4. Update `README.md` and `.zbobr/` example configs: replace `[dispatcher] default_destination_repository`/`default_destination_branch` with `[repo] repository`/`branch`. Remove `fork_owner` entries. Remove preparator tool/model/prompts config sections.

5. Remove `preparator_task.md` prompt file if it exists as a standalone asset.

Why: Keeping tests and docs in sync prevents confusion and ensures CI passes after the structural changes.