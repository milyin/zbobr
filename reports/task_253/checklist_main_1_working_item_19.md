Update documentation to reflect single-repo simplification:

1. `docs/transitions.dot`: remove PREPARING node, GO_PREPARE node, and all PREPARING-related edges
2. `docs/transitions.md`: remove PREPARING from state list in Task fields section
3. `zbobr-task-backend-fs/README.md`: remove `destination_repository, destination_branch` from parameters description
4. `docs/github-token-permissions.md`: update reference to `--tasks-github-task-repo` CLI flag if it was removed