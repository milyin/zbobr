# Tests for `resolve_providers()` — zbobr-api/src/config.rs

Add a `#[cfg(test)] mod tests` block to `zbobr-api/src/config.rs`. These tests validate the provider inheritance resolution logic.

## Test cases

### 1. `resolve_providers_basic` — provider with executor and no parent
Create a `ZbobrDispatcherConfig` with one provider `{ executor: "claude", priority: 10 }`. Call `resolve_providers()`. Assert the resolved provider has executor "claude", priority 10, plan_mode false, access_key None.

### 2. `resolve_providers_single_level_inheritance` — child inherits from parent
Define parent `{ executor: "claude", priority: 10 }` and child `{ parent: "claude_base" }`. Assert child resolves with executor "claude" inherited from parent.

### 3. `resolve_providers_multi_level_chain` — grandchild inherits through chain
Define grandparent `{ executor: "claude" }`, parent `{ parent: "grandparent", plan_mode: true }`, child `{ parent: "parent" }`. Assert child resolves with executor "claude" and plan_mode true.

### 4. `resolve_providers_circular_reference` — error on cycle
Define A `{ parent: "B" }`, B `{ parent: "A" }`. Assert `resolve_providers()` returns an error containing "Circular".

### 5. `resolve_providers_child_overrides_parent` — child fields win
Define parent `{ executor: "claude", priority: 10, plan_mode: false }` and child `{ parent: "parent_name", plan_mode: true, priority: 5 }`. Assert child resolves with plan_mode true and priority 5, executor "claude" inherited.

## Location

`zbobr-api/src/config.rs` — new `#[cfg(test)] mod tests { ... }` at the end of file.

## Dependencies

Only `indexmap::IndexMap` and existing types from the same crate. No external mocking needed — `ZbobrDispatcherConfig` fields are directly constructable.
