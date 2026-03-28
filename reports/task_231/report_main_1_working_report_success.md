# Implementation Complete

## Summary

Implemented interspersed compact comment titles in the user-display context (task description body). Comments are now shown as compact single-line entries alongside stage entries, ordered by timestamp.

## Changes Made

### 1. `zbobr-api/src/task.rs`
- Added `url: Option<String>` field to `Comment` struct to store the GitHub HTML URL of the comment

### 2. `zbobr-api/src/context/mod.rs`
- Added `MdCompactComment` struct: single-line comment representation with truncated text (max 80 chars), timestamp, and optional URL
- Added `MdEntry::CompactComment(MdCompactComment)` variant
- `MdContext::from_task_context`: when `for_prompt=false`, creates `CompactComment` entries; when `for_prompt=true`, keeps full blockquote entries (no change for agent prompts)
- `MdContext::fmt`: renders compact comments as `- text \`date\` <sub>[link](url)</sub>`, and adds `<!-- stage -->` markers before stage titles when compact comments are present
- `MdContext::from_str`: skips `<!-- stage -->` markers; gracefully handles unknown `- ` lines (compact comments) by attempting stage title parse and skipping on failure
- `MdContext::into_task_context`: discards both `Comment` and `CompactComment` entries
- Added 8 new tests covering compact rendering, truncation, roundtrip, and URL handling

### 3. `zbobr-task-backend-github/src/separator.rs`
- Added `comments: &[Comment]` parameter to `serialize_description_full`
- Passes comments to `serialize_context` (instead of empty slice)
- `merge_concurrent_description_updates` passes `&[]` internally (no compact comments during merge — re-added by caller on next write)
- Updated all tests to pass `&[]`

### 4. `zbobr-task-backend-github/src/github.rs`
- Added `html_url: Option<String>` to `CommentResponse`
- Maps `html_url` to `Comment.url` in `get_task_comments_internal`
- `modify_task_internal` now fetches comments once at the start and passes them to all `serialize_description_full` calls

### 5. Other files (`zbobr-task-backend-fs/src/fs.rs`, `zbobr/src/commands.rs`, `zbobr-dispatcher/src/task.rs`)
- Added `url: None` to `Comment` struct literals

## Test Results
All 128 tests pass.

## Format Example
With a comment "confirmed, go on" posted at 2026-03-28 17:21:27 UTC with GitHub URL, the context section would contain:
```
<!-- stage -->
- main:1:**planning** `claude` `claude-sonnet-4.6` `2026-03-28 14:22:54 +0100`
    - ✅ Plan ready ...
- confirmed, go on `2026-03-28 17:21:27 +0000` <sub>[link](https://github.com/milyin/zbobr/issues/231#issuecomment-...)</sub>
<!-- stage -->
- main:1:**working** ...
```
