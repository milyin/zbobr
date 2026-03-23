Preparator Agent findings:

- Task summary: Replace milestone-based task state storage with labels; represent state using labels: state:{done|pause|ready|pending|running}, pipeline:{name}, stage:{name}. Map labels to State enum as specified; apply color conventions for labels.
- No explicit destination repository or branch provided in the task description.

Values passed to configure_worktree:
- destination_repository: null
- destination_branch: null
- work_branch_postfix: replace-milestones-to-labels

configure_worktree response:
- destination_repository: milyin/zbobr
- destination_branch: main
- work_branch: zbobr_fix-158-replace-milestones-to-labels

Next steps (for implementer):
1. Create new branch 'zbobr_fix-158-replace-milestones-to-labels' from main.
2. Update backend code to use labels instead of milestones, implement parsing and enum mapping per task description.
3. Add label color settings and migration if necessary.
4. Run tests and update documentation.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>