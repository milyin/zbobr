The plan has been submitted for review. I'm waiting for your feedback before proceeding to implementation.

**Summary of proposed approach:**

1. **Redesign `Secret`** as a struct with private `source` (enum: Value/Env) and `resolved: Option<String>` cache
2. **`resolve(&mut self) -> Result<&str>`** — resolves+caches; idempotent; called at validation stage
3. **`impl AsRef<str>`** — panics if `resolve` was never called; zero-cost after resolution
4. **`Clone` resets resolved state** — safe for contexts where the config is cloned (e.g. executor building)
5. **Validate methods become `&mut self`** in 3 configs (`ZbobrDispatcherConfig`, `ZbobrTaskBackendGithubConfig`, `ZbobrRepoBackendGithubConfig`)
6. **Usage sites** (github.rs, cli.rs) switch from `resolve()` to `as_ref()`
7. **`ZbobrDispatcher::validated()`** also pre-resolves copilot token