## Add `instance: String` to `ZbobrDispatcherConfig`

Add `instance: String` as a new field to the `ZbobrDispatcherConfig` struct in `zbobr-api/src/config.rs`.

**What to change:**
- Add the field with a sensible default (e.g. `"default"`) in the `Default` impl
- The `#[config_struct]` macro auto-generates `ZbobrDispatcherToml` and `ZbobrDispatcherArgs` — the field will appear there automatically
- Add validation in `ZbobrDispatcherConfig::validate()`: the instance name must be non-empty and must not contain a `:` character (because it appears verbatim in stage titles like `instance:pipeline:run_id:**stage**`)

**Why:** The instance name identifies which zbobr dispatcher process is running. It is used both for stage title formatting and for task filtering.

**Pattern to follow:** The `work_branch_prefix: String` field is the closest analog — also a required string with a default, also validated, also generated into TOML/args by the macro.

**Also update `init.rs`:** The `default_config_toml()` function builds a `ZbobrDispatcherToml`. Add `instance: Some("default".into())` there so that `zbobr init` produces a config file with an example instance value.