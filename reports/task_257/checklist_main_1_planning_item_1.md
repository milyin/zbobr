## What to change

Add a new `get_ctx_rec` MCP tool that returns the content referenced by a context record (its full report file, or the brief if no file is attached). This is the complement to the simplified prompt rendering: agents see plain `[ctx_rec_N]` references and can call this tool to fetch the full content.

### Files to change (in order)

**`zbobr-api/src/config_tools.rs`**:
- Add `GetCtxRec` variant to `McpTool` enum.
- Add its `as_str()` arm returning `"get_ctx_rec"`.
- Add its `FromStr` arm matching `"get_ctx_rec"`.
- Add to `ALL_TOOLS` and `ALL_TOOL_NAMES` arrays.
- Pattern: identical to `DeleteCtxRec`.

**`zbobr-dispatcher/src/mcp/common.rs`**:
- Add `GetCtxRecParam` struct with `id: String` field and schemars description `"Context record ID — either a numeric id or a string like 'ctx_rec_5'"`.
- Pattern: identical in shape to `DeleteCtxRecParam`.

**`zbobr-dispatcher/src/task.rs`** (on `RoleSession`):
- Add method `get_context_record_content(record_id: u64) -> anyhow::Result<Option<String>>`:
  - Call `self.get_task().await?` to get the task.
  - Use `task.context.find_record(record_id)` to find the record.
  - If found with a `report_link`, call `self.read_report(link).await` and return `Ok(Some(content))`.
  - If found without a `report_link`, return `Ok(Some(record.brief.clone()))`.
  - If not found, return `Ok(None)`.

**`zbobr-dispatcher/src/mcp/traits.rs`**:
- Add `get_ctx_rec_impl(&self, id_str: &str) -> String`:
  - Parse id with `parse_ctx_rec_id`.
  - Call `self.session().get_context_record_content(record_id).await`.
  - On `Ok(Some(content))`: log and return `content`.
  - On `Ok(None)`: return `"Error: record ctx_rec_{id} not found"`.
  - On `Err(e)`: return `"Error: {e}"`.
  - Follow the logging pattern from `delete_ctx_rec_impl` (log tool name, role, task id, and response).

**`zbobr-dispatcher/src/mcp/unified.rs`**:
- Import `GetCtxRecParam` in the `use` block.
- Add tool method:
  ```
  #[tool(description = "Get the content of a context record by its id. Returns the full report content if available, or the brief summary. Accepts either a numeric id or a string like 'ctx_rec_5'.")]
  async fn get_ctx_rec(&self, Parameters(params): Parameters<GetCtxRecParam>) -> String {
      self.get_ctx_rec_impl(&params.id).await
  }
  ```

### Why
Context records with report links (e.g. planning summaries, success reports) contain detailed information that is too long to include inline in every prompt. The simplified prompt rendering shows only `[ctx_rec_N]` references; this tool lets agents retrieve the full content when needed.

### Note on tool availability
`GetCtxRec` needs to be added to the `allowed_tools` set for roles that should have access. Check where `allowed_tools` is configured for each role (likely in the pipeline/role configuration) and add it appropriately.
