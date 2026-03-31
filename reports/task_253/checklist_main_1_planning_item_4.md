After the above changes, `TaskIdentity` and the repo backend trait may carry fields that are no longer meaningful:

1. In `zbobr-api/src/config.rs`, review `destination_repository` and `destination_branch` fields on the task/identity structs. Since these are now always determined by the repo backend config, consider whether to remove them from `TaskIdentity` entirely, or derive them from the backend config at call time.

2. If `destination_repository`/`destination_branch` are removed from `TaskIdentity`, update all callers of the repo backend trait methods to not pass them.

3. The repo backend trait (in `zbobr-api`) may have method signatures taking `destination_repository`/`destination_branch` parameters — simplify these to just `work_branch`.

Use the filesystem backend implementation as the analog for the simplified interface shape.

Why: With single-repo operation, the backend already knows the repo and branch from its config. Passing them again via TaskIdentity is redundant and creates a mismatch opportunity.