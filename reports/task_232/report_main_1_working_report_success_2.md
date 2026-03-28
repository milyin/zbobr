# Fix: Parser threshold consistency

## Problem
The reviewer (ctx_rec_8) identified that the parser threshold for detecting child records was `>= 6` instead of `>= 8`, which is inconsistent with the actual indentation scheme:
- Top-level records: 4 spaces
- Child records: 8 spaces

## Fix
Changed `zbobr-api/src/context/mod.rs:429` threshold from `>= 6` to `>= 8` to correctly detect child records at 8-space indentation.

## Verification
- All 120 tests pass
- `cargo fmt --check` passes
- Logic is now consistent: child threshold (>= 8) matches child indentation (8 spaces)