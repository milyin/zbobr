Both `ZbobrRepoBackendFsConfig::repo_short_name()` and `ZbobrRepoBackendGithubConfig::repo_short_name()` are new functions with no tests.

For FS config (zbobr-repo-backend-fs/src/config.rs), test:
- Local path: "/home/user/my-project" → "my-project"
- Path with trailing slash: "/home/user/my-project/" → "my-project"
- Path with .git suffix: "/home/user/my-project.git" → "my-project"
- Remote URL: "https://github.com/owner/repo.git" → "repo"
- Simple name: "repo" → "repo"

For GitHub config (zbobr-repo-backend-github/src/config.rs), test:
- Standard format: "owner/repo" → "repo"
- Just name (no slash): "repo" → "repo"
- Nested: "org/sub/repo" → "repo"

Add `#[cfg(test)]` modules in the respective config files.