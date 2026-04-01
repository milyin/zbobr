# Plan: Remove DeleteCtxRec Action and Hide ctx_rec_ for Linkless Records

## Summary

Two changes are proposed:

1. **Remove `DeleteCtxRec` tool** from the entire MCP stack — enum, parameter struct, implementation, role tool lists, and prompts.

2. **Hide `ctx_rec_{}` in serialized (storage) format for records without links** — their only remaining purpose is to let agents call `get_ctx_rec` to fetch linked report content. Linkless records have nothing to fetch.

**Key constraint**: In `for_prompt` mode, IDs are still shown for ALL records (including linkless checklist items) so agents can still call `check_checklist_item` with `ctx_rec_N` IDs.

---

## Part 1: Remove `DeleteCtxRec` Operation

- `zbobr-api/src/config_tools.rs`: Remove enum variant, string mapping, `ALL_TOOLS`/`ALL_TOOL_NAMES` entries
- `zbobr-dispatcher/src/mcp/common.rs`: Remove `DeleteCtxRecParam` struct (keep `parse_ctx_rec_id` — still used by `check_checklist_item`)
- `zbobr-dispatcher/src/mcp/unified.rs`: Remove `delete_ctx_rec` tool method
- `zbobr-dispatcher/src/mcp/traits.rs`: Remove `delete_ctx_rec_impl()`
- `zbobr-dispatcher/src/task.rs`: Remove `delete_context_record()` method
- `zbobr/src/init.rs`: Remove from all 4 role tool lists; remove prompt text referencing `{mcp_delete_ctx_rec}`
- `zbobr-dispatcher/tests/mcp_integration/abstract_scenarios.rs`: Remove `delete_ctx_rec` test scenario

## Part 2: Make `ctx_rec_` Optional in Storage Format

**Approach**: Make `MdRecord::id` optional (`Option<u64>`).

Changes to `zbobr-api/src/context/mod.rs`:

1. `MdRecord` struct: `id: u64` → `id: Option<u64>`

2. `MdRecord::fmt()`:
   - `for_prompt` mode: emit `[ctx_rec_N]` only when `id.is_some()`
   - storage + link: emit `<sub>[ctx_rec_N](url)</sub>` only when `id.is_some()`
   - storage + no link: emit nothing (no `<sub>` tag)

3. `MdRecord::from_str()`: make `<sub>` optional — if absent, treat full rest as brief, `id = None`, `report_link = None`

4. `MdRecord::from_context_record()`:
   - `for_prompt = true`: always `id = Some(r.id)` (agents need IDs for `check_checklist_item`)
   - `for_prompt = false` + has link: `id = Some(r.id)`
   - `for_prompt = false` + no link: `id = None`

5. ID assignment on parse: two-pass in `into_task_context()` — collect all `Some(id)` values to find `max_known_id`, then assign sequential IDs to `None`-id records starting from `max_known_id + 1` to avoid collisions.

---

## Verification
1. `cargo build` and `cargo test` pass
2. `delete_ctx_rec` absent from all role tool lists
3. Prompt context shows `[ctx_rec_N]` for all records (linked + unchecked checklist)
4. Storage context omits `<sub>ctx_rec_N</sub>` for linkless records
