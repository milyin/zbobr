# Test: entry priority can also increase effective priority

**File:** `zbobr-dispatcher/src/lib.rs` (in the `mod tests` block)

## Test to add

### `select_provider_entry_priority_elevates_above_provider`
Set up two providers, both with priority 5. Give one tool entry an explicit `priority: Some(20)` override (higher than the provider's default of 5). Assert:
- Without any exclusion, the entry with the elevated priority is selected first.
- After excluding that provider, the other entry (effective priority 5) is used.

This is the mirror case of the existing `select_provider_entry_priority_overrides_provider` test, which only tests *lowering* priority. Testing elevation confirms the feature is symmetric and the override applies correctly in both directions.

## Rationale
The existing new test (`select_provider_entry_priority_overrides_provider`) only exercises the use case described in the task description (marking an entry as lower-priority fallback). The general feature allows any integer override. A test for the elevation case ensures the implementation is not accidentally special-casing downward overrides.
