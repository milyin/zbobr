Test plan complete. Two checklist items cover the untested repository normalization logic:

1. **`parse_github_repo()` unit tests** (9 cases) — covers HTTPS URLs, SSH URLs, bare `owner/repo`, `.git` suffix stripping, trailing slashes, and error cases
2. **`from_config()` normalization tests** (2 cases) — verifies that constructing the backend normalizes the stored repository field

Everything else (sanitize_branch_postfix, repo_short_name, validate, TaskIdentity, preparator removal) already has adequate coverage from prior test rounds.