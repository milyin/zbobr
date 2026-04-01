# Test: SSH URL rejection in `parse_github_repo()`

## Motivation
Commit 81fb5d0 tightened `parse_github_repo()` to reject SSH URLs where the part after `:` is not exactly `owner/repo`. The HTTPS rejection path already has a test (`parse_rejects_url_with_extra_path`), but the SSH rejection path has **no tests**.

## File
`zbobr-repo-backend-github/src/github.rs` — add a new test function in the existing `mod tests` block, near `parse_rejects_url_with_extra_path`.

## Test cases (3 assertions in 1 test function)

### `parse_rejects_ssh_url_with_extra_path`
```rust
#[test]
fn parse_rejects_ssh_url_with_extra_path() {
    // SSH URLs with extra path components must be rejected
    let result = parse_github_repo("git@github.com:owner/repo/extra");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid GitHub SSH URL"));

    // Empty owner segment
    let result2 = parse_github_repo("git@github.com:/repo");
    assert!(result2.is_err());

    // Empty repo segment (owner only)
    let result3 = parse_github_repo("git@github.com:owner/");
    assert!(result3.is_err());
}
```

## Location
Place immediately after the existing `parse_rejects_url_with_extra_path` test (around line 971).

## Verification
```bash
cargo test -p zbobr-repo-backend-github parse_rejects_ssh
```
