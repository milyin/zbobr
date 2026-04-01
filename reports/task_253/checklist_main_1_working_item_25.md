The parse_github_repo() function in zbobr-repo-backend-github/src/github.rs is too permissive:
1. HTTPS URL path does not validate that the hostname is "github.com" — e.g., https://notgithub.com/owner/repo passes because it has exactly 5 path components
2. Plain "owner/repo" format does not check that both parts are non-empty — e.g., "/repo" → ["", "repo"] → len==2 → passes

Fix: add `parts[2] == "github.com"` check for HTTPS URLs, and add non-empty check for both parts in the final validation.