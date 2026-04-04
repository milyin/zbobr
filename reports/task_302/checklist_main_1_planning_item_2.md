**What:** Modify `resolve_config_location` in `zbobr-dispatcher/src/cli.rs` to handle multiple config paths instead of a single optional path.

**Changes:**
- Change the function signature to accept `&[PathBuf]` instead of `&Option<PathBuf>`
- When the slice is non-empty: return all resolved config paths, with `config_dir` derived from the **last** config file's parent directory
- When the slice is empty: fall back to current behavior (use cwd + default filename)
- Update the `ConfigLocation` struct to hold multiple paths if needed, or return a list of paths alongside the config_dir
- The function should still canonicalize paths and produce clear error messages

**Why:** Multiple config files need their paths resolved, and path-relative settings in the final merged config should resolve relative to the last (most specific) config file's directory. This preserves the principle that the most project-specific config determines the base directory.

**Analog:** Follow the existing `resolve_config_location` logic for path canonicalization and error handling.