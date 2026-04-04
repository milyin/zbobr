## Review fix: update `init_workspace` rustdoc

The review (ctx_rec_8) identified that the `init_workspace()` function's rustdoc still described only the `.new` file behavior, not accounting for the new `force` parameter.

### Change made

**File:** `zbobr/src/init.rs:32-39`

Updated the doc comment to describe both behaviors:
- When `force` is `true`: existing files are overwritten in place.
- When `force` is `false` (default): new version is written as `{filename}.new`.

### Verification
- `cargo check` passes successfully.