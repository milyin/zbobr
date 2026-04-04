**What:** Modify `ConfigFileArg` in `zbobr-dispatcher/src/cli.rs` to accept multiple config file paths instead of a single optional path.

**Changes:**
- Change `path: Option<PathBuf>` to `paths: Vec<PathBuf>` (or similar)
- Add `-c` as a short alias and keep `--config` as long flag
- The field should use clap's repeating argument pattern so users can pass `-c base.toml -c overlay.toml`

**Why:** This is the entry point for multi-config support. Users need to specify multiple config files that get merged in order, with later files overriding earlier ones.

**Analog:** Follow existing clap `Args` derive patterns already used in the crate. Look at how other Vec fields are handled in the arg structs.