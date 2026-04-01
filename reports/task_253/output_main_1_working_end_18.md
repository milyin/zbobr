Fixed both issues from the review:

1. **URL validation** (`github.rs`): Added `is_valid_github_name()` to reject URLs with query strings (`?tab=readme`), fragments (`#readme`), and `http://` scheme. 3 new tests added, all 31 GitHub backend tests pass.

2. **README docs**: Corrected the "Owner Token" section (split into "Repo Token" for branches/PRs and "Task Token" for issues/milestones) and fixed the wrong claim that stage-specific settings go under `[dispatcher]` (they go in `[workflow.pipelines.*.stages.*]`).