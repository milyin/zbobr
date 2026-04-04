Commands run:
1) cargo fmt --all -- --check
2) cargo clippy --workspace --all-targets -- -D warnings

Output (cargo fmt --check produced diffs and non-zero exit):
== cargo fmt --all -- --check ==
Diff in /data/home/skynet/zdam/zbobr-dev/workspaces/skynet/task-307/zbobr/zbobr-api/src/config.rs:1:
 use std::{collections::HashMap, path::PathBuf};
 
 use indexmap::IndexMap;
-use zbobr_utility::config_struct;
 use zbobr_utility::MergeToml;
+use zbobr_utility::config_struct;
 
 use crate{
     config_tools::McpTool,
Diff in /data/home/skynet/zdam/zbobr-dev/workspaces/skynet/task-307/zbobr/zbobr-api/src/config.rs:508:
         let resolved_prompts_dir = self
             .prompts_dir
             .map(|p| zbobr_utility::resolve_path(p, config_dir));
-        let prompt_base: &std::path::Path = resolved_prompts_dir
-            .as_deref()
-            .unwrap_or(config_dir);
+        let prompt_base: &std::path::Path = resolved_prompts_dir.as_deref().unwrap_or(config_dir);
         let roles = self.roles.map(|roles| {
             roles
                 .into_iter()
[...truncated: many rustfmt diffs in zbobr-api/src/config.rs and a few in zbobr-dispatcher/src/cli.rs ...]
FMT_EXIT=1

Output (cargo clippy):
== cargo clippy --workspace --all-targets -- -D warnings ==
Compiling and checking workspace crates... (completed)
CLIPPY_EXIT=0

Summary:
- cargo fmt reported formatting diffs (exit code 1). Files with diffs include zbobr-api/src/config.rs (multiple locations) and zbobr-dispatcher/src/cli.rs (at least one location). rustfmt would change code layout.
- cargo clippy completed with exit code 0 (no warnings treated as errors under -D warnings). A preexisting clippy_output.txt in repo lists prior warnings, but current run exited successfully.

Recommendation:
- Run `cargo fmt --all` to apply formatting changes, then re-run the checks.

Full raw output (truncated to key sections above). If full untruncated logs are required, request and they will be provided.