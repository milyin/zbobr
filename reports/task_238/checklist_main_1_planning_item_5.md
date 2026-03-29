Remove the old `Secret` enum implementation from `zbobr-api` and replace it with a re-export from `zbobr-utility`.

**What to change:**
- Delete `zbobr-api/src/secret.rs`.
- In `zbobr-api/src/lib.rs`: remove `mod secret;` and `pub use secret::Secret;`, replace with `pub use zbobr_utility::Secret;`. Existing crates that import `zbobr_api::Secret` will continue to work without changes to their import paths.
- In `zbobr-api/Cargo.toml`: remove the `anyhow` dependency if it was added solely for `secret.rs`. Check whether other files in zbobr-api still use `anyhow`; if not, remove it. The `toml` dev-dependency should also be removed since tests for Secret now live in zbobr-utility.
- In `zbobr-api/src/config.rs`: the `use crate::secret::Secret` import (line 8) should change to `use zbobr_utility::Secret` or simply rely on the re-export `use crate::Secret`. Update the two construction sites in config.rs from `Secret::Value(...)` to `Secret::value(...)`.