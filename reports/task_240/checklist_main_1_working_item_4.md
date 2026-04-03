In zbobr-dispatcher/src/cli.rs run_manager_loop():
- Phase 1: iterate all_tasks, apply transitions (pause, ready-from-state), handle Done actions eagerly, handle call_pipeline stages eagerly, collect RunStage candidate tasks into a Vec
- Phase 2: call select_ready_task(&runstage_candidates) to pick the highest-priority ready task, then run its stage
This shares the ready-task selection logic between --select CLI flag and the manager loop.