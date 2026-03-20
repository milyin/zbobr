use std::path::PathBuf;
use std::sync::Arc;

use crate::backend::TaskBackend;

use zbobr_api::Task;
use zbobr_api::config::{StageDefinition, WorkflowConfig};

#[derive(Clone)]
pub struct ConfiguredPromptBuilder {
    base_path: Option<PathBuf>,
    workflow: Arc<WorkflowConfig>,
}

impl ConfiguredPromptBuilder {
    pub fn new(base_path: Option<PathBuf>, workflow: Arc<WorkflowConfig>) -> Self {
        Self { base_path, workflow }
    }

    pub fn base_path(&self) -> Option<&PathBuf> {
        self.base_path.as_ref()
    }

    /// Build full prompt for a stage definition.
    pub async fn build_for_stage(
        &self,
        stage_def: &StageDefinition,
        task_id: u64,
        task_backend: &dyn TaskBackend,
    ) -> anyhow::Result<String> {
        let prompt_files = prompt_files_for_stage(stage_def, &self.workflow);
        let base_prompt = load_prompts(&prompt_files, self.base_path.as_ref())?;
        build_full_prompt(
            &base_prompt,
            &stage_def.role,
            task_id,
            task_backend,
        )
        .await
    }
}

/// Collect prompt file paths from a StageDefinition.
/// If no main_prompt is specified, tries the role's prompt from the workflow config.
pub fn prompt_files_for_stage(stage_def: &StageDefinition, workflow: &WorkflowConfig) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Some(ref main) = stage_def.main_prompt {
        files.push(main.clone());
    } else if let Some(role_def) = workflow.role_definition(&stage_def.role) {
        if let Some(ref prompt_path) = role_def.prompt {
            files.push(prompt_path.clone());
        }
    }
    files.extend(stage_def.additional_prompts.iter().cloned());
    files
}

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
/// 1. Role description (from built-in default or prompt file)
/// 2. Custom prompts (user context from prompt files)
/// 3. Task title
/// 4. Recent task history (non-hidden comments)
/// 5. Unchecked checklist items with ids
pub async fn build_full_prompt(
    user_context: &str,
    role_name: &str,
    task_id: u64,
    task_backend: &dyn TaskBackend,
) -> anyhow::Result<String> {
    let weak = task_backend.get_task(task_id).await?;
    let task = weak.snapshot().await?;
    let comments = weak.get_comments().await?;
    let visible_comments: Vec<_> = comments.iter().filter(|c| !c.hidden).collect();
    let history_json = serde_json::to_string_pretty(&visible_comments).unwrap_or_default();
    Ok(assemble_prompt(
        user_context,
        role_name,
        &task,
        &history_json,
    ))
}

/// Pure synchronous prompt assembly (used by tests and `build_full_prompt`).
fn assemble_prompt(
    user_context: &str,
    _role_name: &str,
    task: &Task,
    history_json: &str,
) -> String {
    let task_title = &task.title;

    let mut sections = Vec::new();

    // Prompts from configured prompt files
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

/// Validate that all prompt files referenced by stage definitions exist.
pub fn validate_stage_prompts(
    workflow: &WorkflowConfig,
    base_path: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let mut missing_files = Vec::new();

    for (_, _, stage) in workflow.all_stages() {
        for path in prompt_files_for_stage(stage, workflow) {
            if !file_exists(&path, base_path) {
                missing_files.push(path);
            }
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

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use tempfile::TempDir;

    use super::*;

    fn dummy_task(title: &str) -> Task {
        Task {
            id: 1,
            title: title.to_owned(),
            description: String::new(),
            state: "READY".to_string(),
            destination_repository: None,
            destination_branch: None,
            work_branch: None,
            pr_url: None,
            checklist: vec![],
            signal: None,
            stack: vec![],
            pause: false,
            confirm: false,
            worktree_retries: 0,
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

    // --- assemble_prompt ---

    #[test]
    fn assemble_prompt_includes_user_context() {
        let prompt = assemble_prompt(
            "my custom instructions",
            "worker",
            &dummy_task(""),
            "",
        );
        assert!(prompt.contains("my custom instructions"));
    }

    #[test]
    fn assemble_prompt_empty_context_omits_user_section() {
        let prompt_empty = assemble_prompt("", "worker", &dummy_task(""), "");
        let prompt_with = assemble_prompt("UNIQUE_MARKER", "worker", &dummy_task(""), "");
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
        let result = assemble_prompt(&loaded, "worker", &dummy_task(""), "");
        assert!(result.contains("do the work carefully"));
    }

    #[test]
    fn no_prompt_files_gives_empty_context() {
        let loaded = load_prompts(&[] as &[PathBuf], None).unwrap();
        let result = assemble_prompt(&loaded, "worker", &dummy_task(""), "");
        let expected = assemble_prompt("", "worker", &dummy_task(""), "");
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
        let result = assemble_prompt(&loaded, "reviewer", &dummy_task(""), "");
        assert!(result.contains("review carefully"));
    }
}
