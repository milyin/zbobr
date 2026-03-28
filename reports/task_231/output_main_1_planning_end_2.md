The plan is finalized with 3 checklist items:

1. **`context/mod.rs`** — Add `compact_comments` mode to `MdContext`: compact single-line comment titles with `<!-- stage -->` markers before stages; update parser to skip both.

2. **`separator.rs`** — Add `comments: &[Comment]` parameter to `serialize_description_full` and forward to `serialize_context`.

3. **`github.rs`** — Fetch comments inside `modify_task_internal` and pass them to `serialize_description_full`.

The core insight: `for_prompt=false` (user/issue display) switches to compact format, `for_prompt=true` (agent prompts) keeps the existing full blockquote format.