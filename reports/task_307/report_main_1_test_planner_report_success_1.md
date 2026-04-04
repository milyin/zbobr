# Test Plan Analysis: `init --force` flag

## Summary
No additional tests are required. The existing test suite adequately covers all behavioral changes introduced by the `--force` flag.

## Existing Coverage

The previous test_worker stage (ctx_rec_16) added 4 tests in `zbobr/src/init.rs` covering every branch of `write_or_new()`:

| Test | Branch | Verified |
|------|--------|----------|
| `write_or_new_force_overwrites_existing_file` | force=true, different content | File overwritten in place, no `.new` created |
| `write_or_new_no_force_creates_dot_new_file` | force=false, different content | Original untouched, `.new` sibling created |
| `write_or_new_skips_identical_content` | Identical content, force=true | File unchanged, no `.new` created |
| `write_or_new_creates_new_file` | File doesn't exist | File created with correct content |

All 4 tests pass (verified by running `cargo test -p zbobr write_or_new`).

## Why No Additional Tests

1. **`write_or_new` is the sole behavioral change** — all force-flag logic is centralized here, and all 4 code paths are covered.
2. **CLI argument parsing** (`commands.rs`) uses clap's derive macro — testing `#[arg(long, short = 'f')]` would just be testing clap itself.
3. **`main.rs` plumbing** is a trivial pattern-match destructure passing the flag through — no conditional logic to test.
4. **`init_workspace`** calls `write_or_new` in a loop over prompt files and config — the interesting behavior is in `write_or_new`, which is already tested. An integration test of `init_workspace` would primarily test file iteration, not force-flag logic.