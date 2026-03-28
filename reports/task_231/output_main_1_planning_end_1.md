The plan has been submitted for review. Here's a summary of the proposed approach:

**Problem:** The GitHub issue body's CONTEXT section is built with `serialize_context(ctx, &[], for_prompt=false, ...)` — empty comments slice, so no comments appear there. Agent prompts do get comments (full blockquotes). 

**Solution — 5 coordinated changes:**

1. **Add `comment_url: Option<String>` to `Comment`** — stores the GitHub comment HTML URL

2. **GitHub backend: populate `comment_url`** — add `html_url` to `CommentResponse`, propagate to `Comment`

3. **Compact `MdUserComment` display** — add `compact: bool` field; when true (user display mode), render as a single line: `- truncated text... \`date\` <sub>[link](url)</sub>` instead of full blockquote

4. **Invisible stage marker** — when `for_user_display=true`, prefix each stage with `<!-- stage -->` so parsers can skip over compact comment lines when they only care about stages; `from_str` ignores these marker lines

5. **Thread comments through `serialize_description_full`** — add `comments: &[Comment]` parameter, fetch them in `update_task` in the GitHub backend, pass `&[]` elsewhere