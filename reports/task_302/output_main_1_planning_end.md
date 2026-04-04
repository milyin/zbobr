Plan is ready for review. The key design decisions:

- **Analog**: follows the existing `merge_with_args` pattern in the proc macro, adding a symmetric `merge_toml` for Toml-to-Toml merging
- **Semantics**: later config files override earlier ones; lists fully replace (natural from `Option<Vec<T>>` + `or`)
- **No config structure changes** per the task requirement
- **`config_dir`** resolved from the last config file's parent directory