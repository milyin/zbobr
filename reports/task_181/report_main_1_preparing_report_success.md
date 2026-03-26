Findings:
- Task title: use system prompt parameter for claude
- Requirements: pass role prompts to claude with --system-prompt; keep existing approach for copilot; avoid phrasing like "in the section below" or "in this prompt" in prompt files.
- No explicit destination repo/branch in description.

Worktree configuration set by configure_worktree:
- destination_repository: milyin/zbobr
- destination_branch: main
- work_branch_postfix: claude-system-prompt
- Resulting work_branch: zbobr_fix-181-claude-system-prompt

Next: Implement changes on the configured branch.