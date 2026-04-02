# Test: resolve_providers_inherits_priority_from_parent

**File:** `zbobr-api/src/config.rs` (in existing `#[cfg(test)] mod tests`)

**Why:** The review fix (ae85fd88) changed `priority: i32` → `priority: Option<i32>` so children inherit parent's priority when unset. The existing tests either have both parent and child as `None` (both default to 10), or the child explicitly overrides with `Some(5)`. No test verifies that a child with `priority: None` actually inherits from a parent with an explicit non-default priority.

**Test case:**
- Parent provider: `executor = "claude"`, `priority = Some(3)`
- Child provider: `parent = "base"`, `priority = None`
- Assert child resolves to `priority == 3` (not default 10)

This directly tests the bug that was fixed: without the `Option<i32>` change, the child would silently get priority 10 instead of inheriting 3.