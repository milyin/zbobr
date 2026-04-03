In zbobr-dispatcher/src/cli.rs:
1. Add !t.state.is_running() filter to select_ready_task
2. Extract fn task_priority(task: &Task) -> u64 { task.stage_count } as the single source of truth for priority
3. Use task_priority in select_ready_task's max_by_key
4. Use task_priority in run_manager_loop's sort_by