All 3 review issues resolved:

1. **GitHub `repo_short_name()` normalization** — now strips trailing `/` and `.git` suffix to match FS backend, handles URL-formatted inputs correctly
2. **Branch validation** — both GitHub and FS backends now validate `branch` is non-empty at startup
3. **Outdated docs** — README.md and `docs/github-token-permissions.md` updated to remove `fork_owner`, `[dispatcher.preparator]`, and fork-based workflow references; added `[repo]` table config example