Remove all references to destination_repository and destination_branch from:
- zbobr-dispatcher/src/cli.rs: ensure_work_branch no longer sets them; print_task no longer shows them
- zbobr-dispatcher/src/task.rs: remove getter/setter methods; fix finish() to use repo_backend.repo_name()
- zbobr-dispatcher/src/prompts.rs: remove adding them from task fields in build_template_variables
- zbobr-dispatcher/src/workflow.rs: remove from test task construction
- zbobr/src/commands.rs: remove from dummy task creation