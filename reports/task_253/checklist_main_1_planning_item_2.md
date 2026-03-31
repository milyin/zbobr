Move the per-task default repository/branch config from the dispatcher to the repo backend:

1. In `zbobr-api/src/config.rs`, remove `default_destination_repository` and `default_destination_branch` from `ZbobrDispatcherConfig`.

2. Add `repository: String` and `branch: String` as required fields to both `ZbobrRepoBackendGithubConfig` (in `zbobr-repo-backend-github/src/config.rs`) and the filesystem backend config (`zbobr-repo-backend-fs/src/config.rs`). These replace the optional dispatcher defaults and are now mandatory.

3. In `configure_worktree_impl` (dispatcher's `mcp/traits.rs`), instead of using `destination_repository`/`destination_branch` from the MCP call or dispatcher defaults, always use the values from the repo backend config. The repo backend exposes its configured `repository` and `branch` via its config or a method.

4. Update TOML structure: what was `[dispatcher] default_destination_repository` becomes `[repo] repository`. Update README/docs examples accordingly.

5. Update any CLI args that mapped to the old dispatcher fields.

The analog for the repo backend config fields is the existing `fork_owner`, `github_token`, `repos_dir` pattern in `zbobr-repo-backend-github/src/config.rs`.

Why: With single-repo operation, the repo backend is the natural owner of "which repo to use". The dispatcher should not need to know about repos — it only manages workflow stages.