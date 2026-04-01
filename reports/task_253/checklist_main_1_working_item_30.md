The function currently accepts malformed copy-pasted GitHub URLs:
- `https://github.com/owner/repo?tab=readme-ov-file` → accepted as "owner/repo?tab=readme-ov-file" (WRONG)
- `https://github.com/owner/repo#readme` → accepted as "owner/repo#readme" (WRONG)
- `http://github.com/owner/repo` → accepted (should require https://)

Fix in zbobr-repo-backend-github/src/github.rs:
1. For HTTPS path: check parts[0] == "https:" before accepting
2. After extracting full_name, validate owner and repo parts only contain valid GitHub name chars [a-zA-Z0-9_\-\.] using a helper function is_valid_github_name
3. Add test cases for these rejected patterns