Remove the MCP tool handler for `delete_ctx_rec` and its implementation chain.

**What to change:**
- `zbobr-dispatcher/src/mcp/unified.rs`: Remove the `#[tool]`-annotated `delete_ctx_rec` async fn and its import of `DeleteCtxRecParam`.
- `zbobr-dispatcher/src/mcp/mod.rs`: Remove `DeleteCtxRecParam` from the `pub use common::{ ... }` re-export line.
- `zbobr-dispatcher/src/mcp/common.rs`: Remove the `DeleteCtxRecParam` struct definition and its associated unit tests (`parse_ctx_rec_id_*` tests are for a shared helper — keep those; only remove tests that directly test deletion param parsing if any are specific to delete). Keep `parse_ctx_rec_id` — it is still used by `get_ctx_rec_impl` (line ~224) and `check_checklist_item_impl` (line ~373) in `traits.rs`.
- `zbobr-dispatcher/src/mcp/traits.rs`: Remove the `delete_ctx_rec_impl` method (lines ~305-361). Keep the `parse_ctx_rec_id` import since it is used by other impls in the same file.
- `zbobr-dispatcher/src/task.rs`: Remove the `delete_context_record` method from `RoleSession` (lines ~281-292).

**Why:** With `DeleteCtxRec` removed from the tool registry, the handler and its plumbing are dead code. `TaskContext::delete_record` in `zbobr-api/src/task.rs` is a domain-layer method with its own test; leave it in place.