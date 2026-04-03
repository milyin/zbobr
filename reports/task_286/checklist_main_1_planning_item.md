## Add `priority` field to `ToolEntry`

**File:** `zbobr-api/src/config.rs` — `ToolEntry` struct

Add an optional `priority: Option<i32>` field to `ToolEntry`, following the same serde conventions as `ProviderDefinition.priority`:
- `#[serde(default, skip_serializing_if = "Option::is_none")]`

`ToolEntry` uses `#[serde(deny_unknown_fields)]`, so the new field must be added inside the struct body.

**Why:** Allows individual tool entries to override the priority tier inherited from their provider, enabling "fallback" patterns where a tool entry with lower priority is only selected if higher-priority entries are excluded.
