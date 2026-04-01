README.md has two doc inconsistencies with the current config/task model:

1. Lines ~279-290: "Owner Token" section says it's used for "managing issues, labels, milestones" and refers to `github_token` in `[repo]`. But `[repo]` token manages code repo (branches, PRs), while issues/milestones are managed by `[tasks]` token. The description should accurately reflect that `[repo]` token is for branch/PR operations.

2. Line ~112: "Stage-specific settings (tool, model, prompts) can be placed in nested tables under `[dispatcher]`. If a stage table is omitted the global defaults are used." — This is outdated. Stage-specific settings are now in `[workflow.pipelines.*.stages.*]`, not under `[dispatcher]`. The workflow config from init.rs confirms this. This note should be removed or corrected.