# Tests for Model::try_new() — zbobr-api/src/task.rs

The latest commit added `Model::try_new(s: &str) -> Result<Self, String>` which rejects strings containing whitespace. `FromStr` and `Deserialize` both delegate to it. There are currently **zero tests** for this validation.

## Tests to add (in existing `#[cfg(test)] mod tests` in task.rs)

### 1. `model_try_new_valid`
- Input: `"claude-opus-4.6"`
- Assert: `Model::try_new("claude-opus-4.6").is_ok()` and inner string matches

### 2. `model_try_new_rejects_space`
- Input: `"claude opus"`
- Assert: `Model::try_new("claude opus").is_err()` and error message contains "whitespace"

### 3. `model_try_new_rejects_tab`
- Input: `"model\there"`
- Assert: `Model::try_new(...)` returns `Err`

### 4. `model_from_str_rejects_whitespace`
- Input: `"bad model".parse::<Model>()`
- Assert: result is `Err`
- Verifies `FromStr` properly delegates to `try_new`

### 5. `model_deserialize_rejects_whitespace`
- Input: deserialize JSON `"bad model"` into `Model`
- Assert: deserialization error
- Verifies `Deserialize` properly delegates to `try_new`

## Implementation notes
- These are pure unit tests requiring no mocks or external dependencies.
- Add inside the existing `mod tests` block at the bottom of `zbobr-api/src/task.rs` (after line 1155).
- `serde_json` is likely already a dev-dependency; if not, use `toml` which is certainly available.
