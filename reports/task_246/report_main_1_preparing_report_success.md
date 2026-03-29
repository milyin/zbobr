Findings:
- Task title: 'disallow comments from non-authorized users'
- Description summary: There is existing GitHub user-filtering for task selection; apply same list to comments so only authorized users' comments are read.
- No explicit destination repository or branch in the task description.

Derived values:
- work_branch_postfix: 'disallow-non-authorized-comments'

Configured values returned by configure_worktree:
- destination_repository: milyin/zbobr
- destination_branch: main
- work_branch: zbobr_fix-246-disallow-non-authorized-comments

Next steps (for implementer):
- Update comment-reading logic to filter comments to only those by authorized users, reusing existing user-filtering list.
- Add tests validating comments from unauthorized users are ignored and authorized users' comments are preserved.
- Run existing test suite and add tests as needed.
