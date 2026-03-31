Remove the "preparing" stage (preparator role) from the default pipeline definition in `zbobr/src/init.rs`. This includes:
- Deleting the "preparing" stage entry from `default_workflow()`
- Removing the `preparator` role from default roles
- Removing any preparator-specific prompt files or references

The planner stage should now be the first stage in the pipeline. The `StageDefinition` pattern for remaining stages serves as the analog — no new patterns are needed, just removal.

Why: The preparator is no longer needed because the branch name will be derived automatically from the task name in code (see next item), which was the preparator's primary job.