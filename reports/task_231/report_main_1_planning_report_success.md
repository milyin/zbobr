
## Approach

The task requires compact comment summaries (similar to stage title lines) to appear in the GitHub issue body's CONTEXT section, interspersed with stage entries by timestamp. Currently the GitHub backend passes `&[]` for comments when rendering the issue body, so nothing appears.

## Key Design Decisions

**Compact vs. blockquote rendering controlled by `for_prompt`**
- `for_prompt=true` (agent prompts): keep existing full blockquote format — agents need the full text.
- `for_prompt=false` (user/issue display): new compact single-line format — keeps the issue body readable.

**`<!-- stage -->` HTML comment before each stage in user-display mode**
Compact comment lines (`- text preview \`date\``) start with `- ` and would be misinterpreted as stage titles by `parse_context`. Adding `<!-- stage -->` before real stages lets the parser distinguish them. The parser is updated to: (1) skip `<!--...-->` lines, (2) skip `- ` lines that fail stage-title parse (compact comment remnants) instead of erroring.

**Report link stored as resolved URL in `MdUserComment`**
The `report_url` closure is only available at construction time (`from_task_context`), not at render time (`fmt`). Resolved URL is stored directly in `MdUserComment.report_link`, mirroring how `MdRecord.report_link` is already handled.

**Comments fetched inside `modify_task_internal`**
One extra GitHub API call per task update. Acceptable trade-off: comments are lightweight to list and necessary for correct display. Failure is non-fatal (`unwrap_or_default`).

## Checklist items
1. Add compact rendering to `MdContext` in `zbobr-api/src/context/mod.rs`
2. Thread comments through `separator.rs::serialize_description_full`
3. Fetch and pass comments in `github.rs::modify_task_internal`
