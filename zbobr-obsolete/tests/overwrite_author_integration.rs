/*
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Test that author rewriting works with a real git repository.
///
/// This integration test:
/// 1. Creates a temporary git repository
/// 2. Creates commits with a different author
/// 3. Simulates what the dispatcher does when overwrite_author is enabled
/// 4. Verifies the commits now have the new author
#[test]
fn test_author_rewriting_with_git_rebase() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize a git repository
    init_git_repo(repo_path);

    // Create initial commit with one author
    create_commit(
        repo_path,
        "Initial commit",
        "original-user",
        "original@example.com",
    );

    // Get the default branch name after first commit
    let default_branch_cmd = "git rev-parse --abbrev-ref HEAD";
    let branch_output = std::process::Command::new("sh")
        .arg("-c")
        .arg(default_branch_cmd)
        .current_dir(repo_path)
        .output()
        .expect("Failed to get branch");
    let default_branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Create a work branch
    run_git_command(repo_path, &["checkout", "-b", "work-branch"]);

    // Create commits with different author
    create_commit(repo_path, "Work commit 1", "work-user", "work@example.com");
    create_commit(repo_path, "Work commit 2", "work-user", "work@example.com");

    // Switch back to default branch and verify it still has original author
    run_git_command(repo_path, &["checkout", &default_branch]);
    let main_author = get_last_commit_author(repo_path);
    assert_eq!(
        main_author, "original-user",
        "Default branch author should be original"
    );

    // Switch back to work branch
    run_git_command(repo_path, &["checkout", "work-branch"]);

    // Verify work commits have different author before rewriting
    let before_author = get_last_commit_author(repo_path);
    assert_eq!(
        before_author, "work-user",
        "Work branch author should be different"
    );

    // Now simulate the dispatcher's author rewriting logic
    let new_author = "dispatcher-user";
    let new_email = "dispatcher@example.com";

    // Set up git config as the dispatcher does
    run_git_command(repo_path, &["config", "--local", "user.name", new_author]);
    run_git_command(repo_path, &["config", "--local", "user.email", new_email]);

    // Use git rebase to rewrite authors on commits since default branch (as dispatcher does)
    let rebase_cmd = format!(
        "git rebase --exec 'git commit --amend --no-edit --reset-author' '{}'",
        default_branch
    );
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&rebase_cmd)
        .env("GIT_AUTHOR_NAME", new_author)
        .env("GIT_AUTHOR_EMAIL", new_email)
        .env("GIT_COMMITTER_NAME", new_author)
        .env("GIT_COMMITTER_EMAIL", new_email)
        .current_dir(repo_path)
        .output()
        .expect("Failed to run rebase command");

    assert!(
        output.status.success(),
        "Rebase failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify that commits after rebase have the new author
    let after_author = get_last_commit_author(repo_path);
    assert_eq!(after_author, new_author, "Author was not rewritten");

    // Verify that default branch still has original author (not affected by rebase)
    run_git_command(repo_path, &["checkout", &default_branch]);
    let main_author_check = get_last_commit_author(repo_path);
    assert_eq!(
        main_author_check, "original-user",
        "Default branch author was affected by rebase"
    );
}

/// Test that author rewriting handles multiple commits correctly.
#[test]
fn test_author_rewriting_multiple_commits() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    init_git_repo(repo_path);
    create_commit(repo_path, "Initial", "original", "original@example.com");

    // Get the default branch name after first commit
    let default_branch_cmd = "git rev-parse --abbrev-ref HEAD";
    let branch_output = std::process::Command::new("sh")
        .arg("-c")
        .arg(default_branch_cmd)
        .current_dir(repo_path)
        .output()
        .expect("Failed to get branch");
    let default_branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    run_git_command(repo_path, &["checkout", "-b", "work-branch"]);

    // Create multiple commits with different authors
    create_commit(repo_path, "Commit 1", "user1", "user1@example.com");
    create_commit(repo_path, "Commit 2", "user2", "user2@example.com");
    create_commit(repo_path, "Commit 3", "user3", "user3@example.com");

    // Get commits before rewriting
    let authors_before = get_all_commit_authors(repo_path);
    assert_eq!(authors_before.len(), 4, "Should have 4 commits total");

    // Perform author rewriting
    let new_author = "dispatcher";
    let new_email = "dispatcher@example.com";

    run_git_command(repo_path, &["config", "--local", "user.name", new_author]);
    run_git_command(repo_path, &["config", "--local", "user.email", new_email]);

    let rebase_cmd = format!(
        "git rebase --exec 'git commit --amend --no-edit --reset-author' '{}'",
        default_branch
    );
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&rebase_cmd)
        .env("GIT_AUTHOR_NAME", new_author)
        .env("GIT_AUTHOR_EMAIL", new_email)
        .env("GIT_COMMITTER_NAME", new_author)
        .env("GIT_COMMITTER_EMAIL", new_email)
        .current_dir(repo_path)
        .output()
        .expect("Failed to run rebase");

    assert!(output.status.success(), "Rebase failed");

    // Get all commits after rewriting
    let authors_after = get_all_commit_authors(repo_path);
    assert_eq!(authors_after.len(), 4, "Commit count should not change");

    // Verify the first 3 commits (newest) on work branch have the new author after rebase
    // The last one is the original commit which was not part of the rebase
    for (i, author) in authors_after.iter().take(3).enumerate() {
        assert_eq!(
            author, new_author,
            "Commit {} author not rewritten to {}",
            i, new_author
        );
    }
    // The original commit (last in the log, first chronologically) should still have its original author
    assert_eq!(
        authors_after[3], "original",
        "Original commit author should not change"
    );
}

/// Test that author rewriting fails gracefully when destination branch doesn't exist.
#[test]
fn test_author_rewriting_nonexistent_destination() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    init_git_repo(repo_path);
    create_commit(repo_path, "Initial", "original", "original@example.com");

    run_git_command(repo_path, &["checkout", "-b", "work-branch"]);
    create_commit(repo_path, "Work commit", "work-user", "work@example.com");

    // Try to rebase onto nonexistent branch
    let rebase_cmd =
        "git rebase --exec 'git commit --amend --no-edit --reset-author' 'nonexistent'";
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(rebase_cmd)
        .env("GIT_AUTHOR_NAME", "dispatcher")
        .env("GIT_AUTHOR_EMAIL", "dispatcher@example.com")
        .current_dir(repo_path)
        .output()
        .expect("Failed to run command");

    // Command should fail gracefully
    assert!(
        !output.status.success(),
        "Rebase should have failed for nonexistent destination"
    );
}

// Helper functions

fn init_git_repo(path: &Path) {
    run_git_command(path, &["init"]);
    run_git_command(path, &["config", "user.name", "Test User"]);
    run_git_command(path, &["config", "user.email", "test@example.com"]);
}

fn create_commit(path: &Path, message: &str, author_name: &str, author_email: &str) {
    // Create a unique file for each commit to avoid conflicts
    let file_name = message.replace(" ", "-").to_lowercase();
    let file_path = path.join(format!("{}.txt", file_name));
    let content = format!("{}\n", message);
    std::fs::write(&file_path, &content).expect("Failed to write file");

    // Stage all changes
    run_git_command(path, &["add", "-A"]);

    // Commit with specific author using -c which is safer
    let commit_output = std::process::Command::new("git")
        .args(&[
            "commit",
            "-m",
            message,
            "--author",
            &format!("{} <{}>", author_name, author_email),
        ])
        .current_dir(path)
        .output()
        .expect("Failed to run git commit");

    if !commit_output.status.success() {
        panic!(
            "Git commit failed:\n{}",
            String::from_utf8_lossy(&commit_output.stderr)
        );
    }
}

fn run_git_command(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .expect("Failed to run git command");

    if !output.status.success() {
        panic!(
            "Git command failed: {:?}\nStderr: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    String::from_utf8(output.stdout).expect("Invalid UTF-8 in git output")
}

fn get_last_commit_author(path: &Path) -> String {
    let output = run_git_command(path, &["log", "-1", "--format=%an"]);
    output.trim().to_string()
}

fn get_all_commit_authors(path: &Path) -> Vec<String> {
    let output = run_git_command(path, &["log", "--format=%an"]);
    output.lines().map(|line| line.to_string()).collect()
}

*/
