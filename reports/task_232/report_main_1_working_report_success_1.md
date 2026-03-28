# Fix: Correct rustfmt Formatting in Test Assertion

## Problem
The previous working session had introduced a test assertion with improper line wrapping that failed `cargo fmt --check`. The formatter expected:

```rust
assert!(
    output.contains("    - ❌ Build failed <sub>[ctx_rec_4](reports/build_fail.md)</sub>")
);
```

But the code had:
```rust
assert!(
    output
        .contains("    - ❌ Build failed <sub>[ctx_rec_4](reports/build_fail.md)</sub>")
);
```

## Fix
Updated `zbobr-api/src/context/mod.rs:819-822` to use the rustfmt-compliant form.

## Verification
- `cargo fmt --check` passes with no diffs
- All 102+ tests pass (42 in zbobr-api, 41 in other modules, 15 in fs backend, 1 in utility, 3 in other crates)
