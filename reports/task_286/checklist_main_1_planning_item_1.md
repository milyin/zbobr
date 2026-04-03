## Update `select_provider_excluding` dispatch logic

**File:** `zbobr-dispatcher/src/lib.rs` — `select_provider_excluding` method

When grouping tool entries by priority (the "Group by provider priority" section), use the entry's own `priority` field if set, falling back to the resolved provider's priority:

```
entry.priority.unwrap_or(rp.priority)
```

instead of just `rp.priority`.

**Tests:**
- The `tool_entry()` helper function at the bottom needs `priority: None` added so it compiles with the new field.
- Add a new test `select_provider_entry_priority_overrides_provider` that verifies: when a `ToolEntry` has `priority: Some(0)` but its provider has priority 10, and the other entry has a provider with priority 10 and no per-entry override, the entry with `priority: Some(0)` is treated as lower priority and only selected when the higher-priority entry's provider is excluded.

**Why:** This is the key semantic change — entries can opt into a lower priority tier even if their provider normally has a high priority, enabling "fallback" tool routing without requiring a separate low-priority provider definition.
