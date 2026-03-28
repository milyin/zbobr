Plan finalized and sent to implementation. Here's a summary of the 6 checklist items:

1. **Add `instance: String` to `ZbobrDispatcherConfig`** — required field, default `"default"`, validated non-empty/no-colon; also update `init.rs` default template.

2. **Thread instance to GitHub backend** — add `instance: String` to `ZbobrTaskBackendGithubConfig`; inject it from dispatcher config in `commands.rs` before backend construction (follows `allowed_usernames` pattern).

3. **GitHub setup: create `zbobr:<instance>` label** — add `INSTANCE_LABEL_PREFIX = "zbobr:"` constant; create/update the label in `setup()`; on `--force`, delete orphaned `zbobr:*` labels for other instances.

4. **GitHub list_tasks: filter by label** — add `("labels", "zbobr:<instance>")` parameter to the GitHub API call (works alongside the existing `creator` filter for `allowed_usernames`).

5. **Stage title format update** — add `instance: String` to `StageInfo` and `MdStageTitle`; update display to `instance:pipeline:run_id:**stage**`; update parser with backwards-compat for old format (empty instance).

6. **Populate `instance` in dispatcher** — set `instance: self.zbobr.config().instance.clone()` when constructing `StageInfo` in `cli.rs`; placeholder for test code in `task.rs`.