Plan is ready. I've created 5 checklist items covering the full implementation:

1. **ConfigFileArg** - change to `Vec<PathBuf>` with `-c` shortcut
2. **config_struct macro** - add `merge_toml` method generation (analog: `merge_with_args`)
3. **resolve_config_location** - update for multiple paths, `config_dir` from last file
4. **main.rs** - fold-merge multiple configs before `build()`
5. **Callers** - update all references to old interfaces