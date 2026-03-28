
## What
Extend `MdContext` in `zbobr-api/src/context/mod.rs` to support two comment display modes: the existing full blockquote format (for agent prompts) and a new compact single-line title format (for user-display / GitHub issue body).

## Why
Currently all comment rendering uses the blockquote `MdUserComment` format regardless of context. The task requires a compact format when rendering for users.

## Changes to `MdContext`
- Add a `compact_comments: bool` field to the `MdContext` struct (default `false` in `FromStr` / parsing paths).
- Update `MdContext::from_task_context` to set `compact_comments = !for_prompt` (i.e., compact when rendering for user display, full blockquote when rendering for agent prompts).

## Changes to `MdUserComment`
- Add a `report_link: Option<String>` field (the already-resolved URL, not the raw filename) so the compact display can emit a link without needing access to the `report_url` closure at render time.
- Populate `report_link` in the `From<&Comment>` impl **can't** do this directly because resolving the URL requires `report_url` which is only known inside `from_task_context`. Instead, handle this inside `MdContext::from_task_context`: after calling `MdUserComment::from(comment)`, resolve `comment.report_name` via `report_url` and store it in `report_link`.
- The `FromStr` impl for `MdUserComment` should set `report_link = None` (compact comments are never parsed back into domain objects; they are rendered-only annotations).

## Changes to `MdContext::fmt` (Display)
When `compact_comments = true`:
- Before emitting each `MdEntry::Stage`, write `<!-- stage -->\n` on its own line.
- For `MdEntry::Comment`, emit a compact single-line entry:
  `- {preview} \`{date}\`[ <sub>[{report_filename}]({url})</sub>]`
  where:
  - `preview` = first line of comment text, stripped of CRLF, trimmed, then truncated to 80 chars with trailing `...` if longer.
  - `date` = timestamp formatted with `format_timestamp()` (existing helper from `stage_title.rs`), same format as stage titles.
  - The `<sub>[…](…)</sub>` part is only emitted when `report_link` is `Some(url)`.
  - Use the `report_name` field as the link label (the filename, without path components); use the resolved URL as the href.

When `compact_comments = false` (current behavior):
- No `<!-- stage -->` markers.
- Comments rendered as blockquotes (unchanged).

## Changes to `MdContext::from_str` (parser)
Two updates needed so existing `parse_context` callers don't break when the issue body contains the new markers:
1. Skip lines starting with `<!--` (HTML comment lines) — these are the `<!-- stage -->` markers.
2. When a line starts with `- ` and **fails** to parse as `MdStageTitle`, skip it silently (instead of returning an error) — these are compact comment title lines. The existing bail! on parse failure should become a `continue`.

## Add helper function
Private `fn compact_preview(text: &str, max_len: usize) -> String`:
- Extracts the first line, strips leading/trailing whitespace, removes internal CRLF artifacts.
- If longer than `max_len`, truncates at a word boundary (or hard-truncates) and appends `...`.
- Recommended `max_len`: 80.

## Analog
Follow `MdRecord::from_context_record` as the analog for resolving file-based links to URLs inside `from_task_context`. Follow `MdStageTitle::fmt` as the analog for the compact comment display format (backtick-wrapped timestamp).
