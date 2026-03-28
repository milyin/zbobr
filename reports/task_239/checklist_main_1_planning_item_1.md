## Add `instance` to GitHub backend config; inject from dispatcher in commands.rs

**Part A — Add `instance: String` to `ZbobrTaskBackendGithubConfig`** (`zbobr-task-backend-github/src/config.rs`):
- Add `instance: String` with default `"default"`
- Do NOT add custom validation here (the dispatcher already validates its own `instance` field)
- The `#[config_struct]` macro will auto-generate TOML/args support, but in practice this field is set programmatically

**Part B — Inject instance from dispatcher config** (`zbobr/src/commands.rs`):
- In the `run()` function, before calling `TaskBackendGithub::new(tasks_config)`, add:  
  `tasks_config.instance = dispatcher_config.instance.clone();`
- This keeps instance configured in one place (the dispatcher config) while making it available to the backend

**Why:** The backends are constructed independently from the dispatcher config. Threading the instance name through the backend config struct is the least invasive approach — it follows the existing `allowed_usernames` pattern (config field that controls filtering behavior).

**Pattern to follow:** `allowed_usernames: Option<Vec<String>>` in the same config struct. The difference is `instance` is injected from the dispatcher rather than set by the user directly.

**Also update `init.rs`:** Do NOT include `instance` in the default `ZbobrTaskBackendGithubToml` — it is always injected from the dispatcher. Leave it at its default.