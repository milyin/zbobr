The plan is ready for review. Here's the summary:

**Two parts:**

**Part 1 — Simplify prompt context rendering** (`zbobr-api/src/context/mod.rs`):
- Stage header: `- planning` instead of `- skynet:main:1:**planning** \`claude\` \`claude-sonnet-4.6\` \`2026-04-01...\``
- Records: `[ctx_rec_N]` instead of `<sub>[ctx_rec_N](url)</sub>`
- Comments: `- user milyin: text` instead of `- user:**milyin** text \`timestamp\``

All via adding `for_prompt: bool` to `MdRecord`, `MdStage`, `MdCompactComment` and branching in their `Display` implementations. The `for_prompt` flag already flows through the chain — this just extends it to the leaf rendering.

**Part 2 — Add `get_ctx_rec` MCP tool** (5 files):
- `config_tools.rs`: new `GetCtxRec` variant
- `mcp/common.rs`: `GetCtxRecParam`
- `mcp/traits.rs`: `get_ctx_rec_impl` — reads task context to find the record, returns `report_link` file content (or brief if no file attached)
- `mcp/unified.rs`: tool definition
- `init.rs`: add to all roles