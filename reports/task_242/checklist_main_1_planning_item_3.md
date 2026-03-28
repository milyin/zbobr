In `zbobr/src/init.rs`, add `allowed_usernames: None` to the `ZbobrTaskBackendGithubToml` struct literal in `default_config_toml()`.

This keeps the default config template in sync with the new field so it compiles and gives users a commented-out example of the option.