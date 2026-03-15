use std::path::PathBuf;

use crate::{
    backend::TaskBackend,
    task::Role,
};

use zbobr_api::Task;
pub use zbobr_api::config::PromptsConfig;

/// Load and concatenate multiple prompt files.
/// Relative paths are resolved relative to `base_path` if provided, otherwise cwd.
/// Returns an error if any file cannot be read.
pub fn load_prompts(paths: &[PathBuf], base_path: Option<&PathBuf>) -> anyhow::Result<String> {
    let mut combined = String::new();
    for path in paths.iter() {
        let resolved_path = if let Some(base) = base_path {
            if path.is_relative() {
                base.join(path)
            } else {
                path.clone()
            }
        } else if path.is_relative() {
            std::env::current_dir()?.join(path)
        } else {
            path.clone()
        };

        let content = std::fs::read_to_string(&resolved_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read prompt file '{}': {}",
                resolved_path.display(),
                e
            )
        })?;

        let trimmed = content.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !combined.is_empty() {
            combined.push_str("\n\n");
        }
        combined.push_str(trimmed);
    }
    Ok(combined)
}

/// Build full prompt with sections in order:
/// 1. Role description (hardcoded instructions)
/// 2. MCP API docs
/// 3. Custom prompts (user context from prompt files)
/// 4. Task title
/// 5. Recent task history (latest chunk from get_history)
/// 6. Unchecked checklist items with ids
pub async fn build_full_prompt(
    user_context: &str,
    role: Role,
    task_id: u64,
    task_backend: &dyn TaskBackend,
) -> anyhow::Result<String> {
    let task = task_backend.get_task(task_id).await?.snapshot().await?;
    let history = crate::get_history(task_backend, task_id, None).await?;
    let history_json = serde_json::to_string_pretty(&history.comments).unwrap_or_default();
    Ok(assemble_prompt(user_context, role, &task, &history_json))
}

/// Pure synchronous prompt assembly (used by tests and `build_full_prompt`).
fn assemble_prompt(user_context: &str, role: Role, task: &Task, history_json: &str) -> String {
    let task_title = &task.title;
    let hardcoded = match role {
        Role::Preparator => crate::preparator_instructions(),
        Role::Planner => crate::planner_instructions(),
        Role::Worker => crate::worker_instructions(),
        Role::Reviewer => crate::reviewer_instructions(),
        Role::Tester => crate::tester_instructions(),
        Role::Merger => crate::merger_instructions(),
    };

    let api_docs = match role {
        Role::Preparator => crate::PreparatorMcp::generate_api_docs(),
        Role::Planner => crate::PlannerMcp::generate_api_docs(),
        Role::Worker => crate::WorkerMcp::generate_api_docs(),
        Role::Reviewer => crate::ReviewerMcp::generate_api_docs(),
        Role::Tester => crate::TesterMcp::generate_api_docs(),
        Role::Merger => crate::MergerMcp::generate_api_docs(),
    };

    let mut sections = vec![hardcoded];

    // MCP API docs
    sections.push(api_docs);

    // Custom prompts from prompt files
    if !user_context.is_empty() {
        sections.push(user_context.to_owned());
    }

    // Task title
    if !task_title.is_empty() {
        sections.push(format!("# Current task: {task_title}"));
    }

    // Recent task history
    if !history_json.is_empty() {
        sections.push(format!("# Recent task history\n\n{history_json}"));
    }

    // Unchecked checklist items with ids
    let unchecked: Vec<_> = task.checklist.iter().filter(|item| !item.checked).collect();
    if !unchecked.is_empty() {
        let mut checklist_text = String::from("# Unchecked checklist items\n");
        for item in &unchecked {
            checklist_text.push_str(&format!("\n- [ ] [id: {}] {}", item.id, item.text));
        }
        sections.push(checklist_text);
    }

    sections.join("\n\n---\n\n")
}

/// Validate that all specified prompt files exist.
/// Returns an error listing all missing files if any are not found.
pub fn validate_prompts(prompts: &PromptsConfig) -> anyhow::Result<()> {
    let mut missing_files = Vec::new();

    for path in &prompts.preparator {
        if !file_exists(path, prompts.path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.planner {
        if !file_exists(path, prompts.path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.worker {
        if !file_exists(path, prompts.path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.reviewer {
        if !file_exists(path, prompts.path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.tester {
        if !file_exists(path, prompts.path.as_ref()) {
            missing_files.push(path.clone());
        }
    }
    for path in &prompts.merger {
        if !file_exists(path, prompts.path.as_ref()) {
            missing_files.push(path.clone());
        }
    }

    if !missing_files.is_empty() {
        let missing_list = missing_files
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(anyhow::anyhow!(
            "The following prompt files do not exist:\n{}",
            missing_list
        ));
    }

    Ok(())
}

/// Check if a file exists, resolving relative paths with base_path if provided.
fn file_exists(path: &PathBuf, base_path: Option<&PathBuf>) -> bool {
    let resolved_path = if let Some(base) = base_path {
        if path.is_relative() {
            base.join(path)
        } else {
            path.clone()
        }
    } else if path.is_relative() {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => return false,
        }
    } else {
        path.clone()
    };

    resolved_path.exists()
}

/// Build the full prompt for the given role using the prompts config.
pub async fn build_prompt_for_role(
    prompts: &PromptsConfig,
    role: Role,
    task_id: u64,
    task_backend: &dyn TaskBackend,
) -> anyhow::Result<String> {
    let base_prompt = load_prompts(prompts.prompts_for_role(role), prompts.path.as_ref())?;
    build_full_prompt(&base_prompt, role, task_id, task_backend).await
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use tempfile::TempDir;

    use zbobr_api::Stage;

    use super::*;

    fn dummy_task(title: &str) -> Task {
        Task {
            id: 1,
            title: title.to_owned(),
            description: String::new(),
            stage: Stage::Pending,
            destination_repository: None,
            destination_branch: None,
            work_branch: None,
            pr_url: None,
            checklist: vec![],
            signal: None,
            conflict: false,
            pause: false,
            confirm: false,
            etag: None,
        }
    }

    fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    // --- load_prompts ---

    #[test]
    fn load_prompts_reads_existing_file() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "p.md", "  hello world  ");
        let result = load_prompts(&[path], None).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn load_prompts_errors_on_missing_file() {
        let result = load_prompts(&[PathBuf::from("/nonexistent/path/prompt.md")], None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Failed to read prompt file"));
    }

    #[test]
    fn load_prompts_concatenates_multiple_files() {
        let dir = TempDir::new().unwrap();
        let a = write_file(&dir, "a.md", "first");
        let b = write_file(&dir, "b.md", "second");
        let result = load_prompts(&[a, b], None).unwrap();
        assert_eq!(result, "first\n\nsecond");
    }

    #[test]
    fn load_prompts_skips_empty_files() {
        let dir = TempDir::new().unwrap();
        let empty = write_file(&dir, "empty.md", "   \n  \n");
        let real = write_file(&dir, "real.md", "content");
        let result = load_prompts(&[empty, real], None).unwrap();
        assert_eq!(result, "content");
    }

    #[test]
    fn load_prompts_resolves_relative_path_with_base_path() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "custom.md", "custom content");
        let relative = PathBuf::from("custom.md");
        let base = dir.path().to_path_buf();
        let result = load_prompts(&[relative], Some(&base)).unwrap();
        assert_eq!(result, "custom content");
    }

    #[test]
    fn load_prompts_absolute_path_ignores_base_path() {
        let dir = TempDir::new().unwrap();
        let abs = write_file(&dir, "abs.md", "absolute content");
        let fake_base = PathBuf::from("/nonexistent/base");
        let result = load_prompts(&[abs], Some(&fake_base)).unwrap();
        assert_eq!(result, "absolute content");
    }

    // --- PromptsConfig ---

    #[test]
    fn prompts_for_role_returns_correct_paths() {
        let prompts = PromptsConfig {
            path: None,
            preparator: vec![PathBuf::from("prep.md")],
            planner: vec![PathBuf::from("plan.md")],
            worker: vec![PathBuf::from("work.md")],
            reviewer: vec![PathBuf::from("review.md")],
            tester: vec![PathBuf::from("test.md")],
            merger: vec![PathBuf::from("merge.md")],
        };
        assert_eq!(prompts.prompts_for_role(Role::Preparator), &[PathBuf::from("prep.md")]);
        assert_eq!(prompts.prompts_for_role(Role::Planner), &[PathBuf::from("plan.md")]);
        assert_eq!(prompts.prompts_for_role(Role::Worker), &[PathBuf::from("work.md")]);
        assert_eq!(prompts.prompts_for_role(Role::Reviewer), &[PathBuf::from("review.md")]);
        assert_eq!(prompts.prompts_for_role(Role::Tester), &[PathBuf::from("test.md")]);
        assert_eq!(prompts.prompts_for_role(Role::Merger), &[PathBuf::from("merge.md")]);
    }

    // --- assemble_prompt ---

    #[test]
    fn assemble_prompt_includes_user_context() {
        let prompt = assemble_prompt("my custom instructions", Role::Worker, &dummy_task(""), "");
        assert!(prompt.contains("my custom instructions"));
    }

    #[test]
    fn assemble_prompt_empty_context_omits_user_section() {
        let prompt_empty = assemble_prompt("", Role::Worker, &dummy_task(""), "");
        let prompt_with = assemble_prompt("UNIQUE_MARKER", Role::Worker, &dummy_task(""), "");
        assert!(!prompt_empty.contains("UNIQUE_MARKER"));
        // With context is longer (has the extra context section)
        assert!(prompt_with.len() > prompt_empty.len());
    }

    // --- load_prompts + assemble_prompt integration ---

    #[test]
    fn load_prompts_content_appears_in_assembled_prompt() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "worker.md", "do the work carefully");
        let loaded = load_prompts(&[path], None).unwrap();
        let result = assemble_prompt(&loaded, Role::Worker, &dummy_task(""), "");
        assert!(result.contains("do the work carefully"));
    }

    #[test]
    fn no_prompt_files_gives_empty_context() {
        let loaded = load_prompts(&[] as &[PathBuf], None).unwrap();
        let result = assemble_prompt(&loaded, Role::Worker, &dummy_task(""), "");
        let expected = assemble_prompt("", Role::Worker, &dummy_task(""), "");
        assert_eq!(result, expected);
    }

    #[test]
    fn load_prompts_with_base_path_resolves_relative_files() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "reviewer.md", "review carefully");
        let loaded = load_prompts(
            &[PathBuf::from("reviewer.md")],
            Some(&dir.path().to_path_buf()),
        )
        .unwrap();
        let result = assemble_prompt(&loaded, Role::Reviewer, &dummy_task(""), "");
        assert!(result.contains("review carefully"));
    }

    // --- validate_prompts ---

    #[test]
    fn validate_prompts_succeeds_with_existing_files() {
        let dir = TempDir::new().unwrap();
        let worker_file = write_file(&dir, "worker.md", "content");
        let prompts = PromptsConfig {
            path: None,
            preparator: vec![],
            planner: vec![],
            worker: vec![worker_file],
            reviewer: vec![],
            tester: vec![],
            merger: vec![],
        };
        assert!(validate_prompts(&prompts).is_ok());
    }

    #[test]
    fn validate_prompts_succeeds_with_empty_prompts() {
        let prompts = PromptsConfig::default();
        assert!(validate_prompts(&prompts).is_ok());
    }

    #[test]
    fn validate_prompts_fails_with_missing_file() {
        let prompts = PromptsConfig {
            path: None,
            preparator: vec![],
            planner: vec![],
            worker: vec![PathBuf::from("/nonexistent/worker.md")],
            reviewer: vec![],
            tester: vec![],
            merger: vec![],
        };
        let result = validate_prompts(&prompts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("do not exist"));
        assert!(err.contains("/nonexistent/worker.md"));
    }

    #[test]
    fn validate_prompts_lists_all_missing_files() {
        let prompts = PromptsConfig {
            path: None,
            preparator: vec![PathBuf::from("/missing1.md")],
            planner: vec![PathBuf::from("/missing2.md")],
            worker: vec![],
            reviewer: vec![],
            tester: vec![],
            merger: vec![],
        };
        let result = validate_prompts(&prompts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("/missing1.md"));
        assert!(err.contains("/missing2.md"));
    }

    #[test]
    fn validate_prompts_resolves_relative_paths_with_base_path() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "worker.md", "content");
        let prompts = PromptsConfig {
            path: Some(dir.path().to_path_buf()),
            preparator: vec![],
            planner: vec![],
            worker: vec![PathBuf::from("worker.md")],
            reviewer: vec![],
            tester: vec![],
            merger: vec![],
        };
        assert!(validate_prompts(&prompts).is_ok());
    }

    #[test]
    fn validate_prompts_detects_missing_relative_paths_with_base_path() {
        let dir = TempDir::new().unwrap();
        let prompts = PromptsConfig {
            path: Some(dir.path().to_path_buf()),
            preparator: vec![],
            planner: vec![],
            worker: vec![PathBuf::from("missing.md")],
            reviewer: vec![],
            tester: vec![],
            merger: vec![],
        };
        let result = validate_prompts(&prompts);
        assert!(result.is_err());
    }
}
