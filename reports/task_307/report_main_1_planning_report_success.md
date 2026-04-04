## Plan: Add `--force` flag to `init` command

### Approach
Follow the existing `Setup` command's `--force` flag pattern. The change touches three files in a straightforward pipeline: CLI definition → dispatch → implementation.

### Key decisions
- Reuse the same `#[arg(long, short = 'f')]` pattern from `Setup`
- When `force` is true and file exists with different content, overwrite in place instead of creating `.new`
- "Unchanged" and "new file" paths are unaffected by `force`

### Analog
`Setup` command's `force` flag in `commands.rs` (lines 37-38)

### Checklist items created
1. Add `force` field to `Init` variant in `commands.rs`
2. Pass `force` through `main.rs` to `init_workspace()`
3. Update `init.rs`: accept `force` param, change `write_or_new` behavior
4. Build and test