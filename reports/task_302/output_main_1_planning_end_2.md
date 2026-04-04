Verification is complete. Here's what I found:

**All correct:**
- Every Vec field in TOML structs is `Option<Vec<T>>` — no bare `Vec` in mergeable types
- All `*Patch` types cleanly removed, no dangling references
- Merge semantics work: `None` = inherit from base, `Some(vec![])` = explicitly empty
- Runtime semantics consistent: `None` and `Some(vec![])` both mean "no items" everywhere (`prompts.rs`, `cli.rs`)
- Build passes, all relevant tests pass

**Intentional behavior change noted:** Roles without explicit `mcp` now get **no tools** instead of **all tools**. Test fixtures updated accordingly.

**Proposed tests** to lock down the verified behavior:
1. TOML deserialization round-trip for `Option<Vec>` fields
2. Tools map merge with `Vec<ToolEntry>` wholesale replacement
3. End-to-end multi-config merge from TOML strings