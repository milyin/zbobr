The README.md still references --repo-github-repository in two places, but the actual CLI flag generated from ZbobrRepoBackendGithubConfig is --repo-repository (verified by running zbobr --help). Fix both occurrences:
1. "Target Repository" concept description: (`--repo-github-repository`) → (`--repo-repository`)  
2. Bottom sentence about eliminating manual flag specification