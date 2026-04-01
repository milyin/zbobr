The plan covers two changes:

**Part 1 — Remove `DeleteCtxRec` completely:**
- Remove the variant, param struct, impl method, and tool handler across `config_tools.rs`, `common.rs`, `mod.rs`, `unified.rs`, `traits.rs`, and `task.rs`
- Remove from all role `mcp` lists in `init.rs` (planner, worker, test_planner, test_worker)
- Remove the `{mcp_delete_ctx_rec}` reference from the planner prompt

**Part 2 — Don't show `ctx_rec_N` for records without a `report_link`:**
- In `context/mod.rs` (`MdRecord::fmt`): conditionally render `ctx_rec_N` only when `report_link` is `Some`
- In `traits.rs` (`add_checklist_item_impl`): only include `ctx_rec_{id}` in the return message when the report was stored successfully
- Update affected tests that expect `ctx_rec_N` for linkless records