Change the type of every token field in the config structs from `String` to `Secret`:

1. `ZbobrDispatcherConfig.agent_github_token` — `zbobr-api/src/config.rs`
2. `ZbobrRepoBackendGithubConfig.github_token` — `zbobr-repo-backend-github/src/config.rs`
3. `ZbobrTaskBackendGithubConfig.github_token` — `zbobr-task-backend-github/src/config.rs`
4. `ZbobrExecutorCopilotConfig.copilot_github_token` — `zbobr-executor-copilot/src/config.rs`

For each field:
- Change the field type from `String` to `Secret`
- If the field has `#[arg(long, env = "...")]`, remove the clap arg attribute entirely or add `#[config(skip_args)]` so the `#[config_struct]` macro skips generating a CLI flag for it. Secrets should not be passed on the command line.
- Update the `Default` impl: replace `String::new()` / `"not-configured".to_string()` with `Secret::Value(String::new())` / `Secret::Value("not-configured".to_string())` as appropriate.
- Update `build()` methods: the `copilot` executor currently uses manual `std::env::var()` fallback logic — remove this ad-hoc logic. Users who want env-var-sourced secrets must now configure `{ env = "VAR_NAME" }` in TOML. The build method should simply propagate the `Secret` value from Toml without special env-var overriding.

Follow the same field + Default update pattern used throughout the existing config structs.