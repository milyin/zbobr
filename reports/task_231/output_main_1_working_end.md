The implementation is complete. Here's what was done:

**Feature: Compact comment titles in user-display context**

In user-display mode (task description body), comments are now shown as compact single-line entries interspersed with stage entries by timestamp:

```
<!-- stage -->
- main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-28 14:22:54 +0100`
    - ✅ Plan ready...
- confirmed, go on `2026-03-28 17:21:27 +0000` <sub>[link](https://github.com/.../comment)</sub>
<!-- stage -->
- main:1:**working** ...
```

**Key changes across 7 files:**
- `Comment` struct: added `url: Option<String>` for GitHub HTML URL
- `context/mod.rs`: added `MdCompactComment`, `MdEntry::CompactComment`, compact rendering when `for_prompt=false`, `<!-- stage -->` markers before stages when compact comments present, lenient parsing that skips unknown `- ` lines
- `separator.rs`: `serialize_description_full` now accepts and threads `comments: &[Comment]`
- `github.rs`: captures `html_url` from GitHub API, fetches comments in `modify_task_internal` before serializing
- 8 new tests added; all 128 tests pass