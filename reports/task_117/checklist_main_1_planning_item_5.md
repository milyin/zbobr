## Verification steps

1. **Compile check**: Run `cargo build` to ensure all changes compile without errors. Pay attention to:
   - New `git_env` and `git_check_env` functions are correctly exported from `zbobr-utility`
   - Import in `github.rs` includes the new functions
   - All call sites pass the correct types (particularly `&[(&str, &str)]`)
   - The `commands.rs` change doesn't break any imports

2. **Run existing tests**: `cargo test` to check for regressions.

3. **Grep for remaining token-in-URL patterns**: Search for `x-access-token` in the codebase to verify no instances remain outside of the env var construction (which is expected — the token is in the env var value, not in command args or URLs).

4. **Verify error messages don't leak tokens**: Check that the `git_env` and `git_check_env` error messages only include `args.join(" ")` and not env var values.
