In prompt mode, only show `[ctx_rec_N]` for records an agent can actually interact with.

**What to change:**
- `zbobr-api/src/context/mod.rs`, `MdRecord::fmt()` (around lines 150-161):

Current behavior when `for_prompt == true`: always writes ` [ctx_rec_N]`.

New behavior when `for_prompt == true`: only write ` [ctx_rec_N]` when the record is interactive — i.e., when `self.record_type` is `CheckboxUnchecked` or `CheckboxChecked` (agent can call `check_checklist_item`), OR when `self.report_link` is `Some(_)` (agent can call `get_ctx_rec`). For all other records in prompt mode, omit the ID entirely.

**Why:** Agents only need `ctx_rec_N` IDs to call tools that act on specific records. Without `DeleteCtxRec`, the only tools that need an ID are `check_checklist_item` (checkboxes only) and `get_ctx_rec` (records with a report link). Printing IDs for Success/Failure/Comment/Question records that have no link wastes context and confuses agents.

**Parsing is unaffected:** `FromStr` / `Deserialize` for `MdRecord` parses the storage format (`<sub>ctx_rec_N</sub>` or `<sub>[ctx_rec_N](url)</sub>`), not the prompt format. Storage format is unchanged by this task.