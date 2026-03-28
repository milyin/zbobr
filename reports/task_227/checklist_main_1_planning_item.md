## What

Add an optional `allowed_users: Vec<String>` field to `ZbobrDispatcherConfig` in `zbobr-api/src/config.rs`.

The field should be declared similar to other optional/defaultable fields in the same struct (e.g., `timezone`, `overwrite_author`). It is optional in the TOML config.

## Default behavior

After the config is fully built (in the dispatcher initialization code, likely in `zbobr-dispatcher/src/lib.rs` or `zbobr/src/main.rs`), apply this rule: if `allowed_users` is empty/not set, initialize it to `vec![config.git_user_email.clone()]`. This ensures there is always at least one allowed user unless explicitly configured otherwise.

## Why

This gives users a way to restrict task processing to specific GitHub accounts. The default of using `git_user_email` preserves backwards-compatible behavior — by default, the dispatcher only processes tasks created by the account associated with the configured git email.

## Pattern to follow

Follow the same pattern as other `Vec<String>` or optional fields in `ZbobrDispatcherConfig`. The `#[config_struct]` macro handles TOML deserialization automatically for standard types.