## Plan: Allow Configuration Sharing via Multiple --config Parameters

### Context
Multiple zbobr instances need to share common pipeline/template logic while applying project-specific patches. Currently only a single `--config` path is supported.

### Approach

**1. Change `ConfigFileArg` to accept multiple config paths**
- File: `zbobr-dispatcher/src/cli.rs`
- Change `path: Option<PathBuf>` to `paths: Vec<PathBuf>` with `-c` short and `--config` long flags
- When `paths` is non-empty, default `zbobr.toml` is skipped; when empty, fall back to `zbobr.toml`

**2. Add Toml-to-Toml merge in `config_struct` macro**
- File: `zbobr-macros/src/lib.rs`
- Generate `merge_toml(self, other: Self) -> Self` on each `*Toml` struct
- Leaf fields: `other.field.or(self.field)` (later wins)
- Nested fields: recursive merge when both `Some`; take whichever side is `Some` otherwise
- Lists naturally fully replace (they're `Option<Vec<T>>`)

**3. Update `resolve_config_location` for multiple paths**
- File: `zbobr-dispatcher/src/cli.rs`
- Accept `&[PathBuf]` instead of `&Option<PathBuf>`
- `config_dir` derived from the last config file's parent directory
- When no paths given, use current behavior (cwd + default filename)

**4. Merge configs in main.rs**
- File: `zbobr/src/main.rs`
- Iterate config paths, parse each to `RootConfigToml`, fold-merge, then `RootConfig::build()`

**5. Update `GlobalArgs` field access** in `zbobr-dispatcher/src/cli.rs`

### Key design decisions
- Analog: follows existing `merge_with_args` pattern in the macro, just Toml×Toml instead of Toml×Args
- No config structure changes (per task requirement)
- Last file's directory determines path resolution base