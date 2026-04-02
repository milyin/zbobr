## Unit test for stage-title parsing rejecting malformed model tokens

### Location
Add to the existing `#[cfg(test)] mod tests` block in `zbobr-api/src/context/stage_title.rs`.

### Tests to add (2 tests)

1. **`parse_rejects_malformed_model_token`** — Construct a stage title string with a valid tool backtick but an invalid model backtick (e.g. containing whitespace like `` `bad model` ``). Assert that `s.parse::<MdStageTitle>()` returns `Err` and the error message contains "Invalid model token".

2. **`parse_accepts_valid_model_token`** — Construct a stage title string with a valid tool and valid model (no whitespace). Assert that `s.parse::<MdStageTitle>()` succeeds and `model` field contains the expected `Model` value. (This may overlap with existing roundtrip test, but specifically targets the parsing branch that was changed.)

### Rationale
The previous implementation silently dropped invalid model tokens (`.ok()`), and the fix changed this to propagate errors. Without a specific test, a future refactor could reintroduce the silent-drop behavior. The test directly exercises the error path in the `FromStr` impl at line 168-171.