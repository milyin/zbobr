# Test Plan: `init --force` flag

## Analysis

The `--force` flag adds a new branch in `write_or_new()` (`zbobr/src/init.rs:83-106`). Currently there are **zero tests** for `write_or_new` — existing tests in the module only cover `inline_dispatcher_tables`, `default_workflow` validation, stage routing, and prompt file completeness.

The function has 4 code paths that need coverage:

| Path | Condition | Behavior | New? |
|------|-----------|----------|------|
| 1 | File doesn't exist | Create file | No |
| 2 | File exists, identical content | Skip | No |
| 3 | File exists, different content, `force=false` | Write `.new` sibling | No (refactored) |
| 4 | File exists, different content, `force=true` | Overwrite in place | **Yes** |

## Tests to implement

All tests go in the existing `mod tests` block in `zbobr/src/init.rs`. Each uses `tempfile::tempdir()` for isolation.

1. **`write_or_new_force_overwrites_existing_file`** — Core new behavior: `force=true` overwrites a file with different content in place, no `.new` created.

2. **`write_or_new_no_force_creates_dot_new_file`** — Counterpart: `force=false` leaves original untouched and writes `.new` sibling.

3. **`write_or_new_skips_identical_content`** — Identical content early return works with `force=true` (no unnecessary overwrite).

4. **`write_or_new_creates_new_file`** — Base case: non-existing path creates the file regardless of force flag.

## Notes

- All tests are behavioral (file system outcomes), not static text assertions.
- `write_or_new` is a private async function, so tests must be `#[tokio::test]` within the same module.
- May need to add `tempfile` as a dev-dependency if not already present.