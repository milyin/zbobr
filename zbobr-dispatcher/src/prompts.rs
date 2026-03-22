use std::{borrow::Cow, collections::HashMap, path::PathBuf, sync::Arc};

use simpleinterpolation::Interpolation;
use zbobr_api::{
    Comment, HistoryRecordType, Task, classify_comment,
    config::{StageDefinition, WorkflowConfig},
    config_tools::McpTool,
};

use crate::{backend::TaskBackend, workflow::Workflow};

// Template placeholder names used in prompt .md files.
pub const VAR_TITLE: &str = "title";
pub const VAR_DESCRIPTION: &str = "description";
pub const VAR_DESTINATION_REPOSITORY: &str = "destination_repository";
pub const VAR_DESTINATION_BRANCH: &str = "destination_branch";
pub const VAR_WORK_BRANCH: &str = "work_branch";
pub const VAR_CHECKLIST: &str = "checklist";
pub const VAR_LAST_REPORT: &str = "last_report";
pub const VAR_LAST_REQUEST: &str = "last_request";

#[derive(Clone)]
pub struct ConfiguredPromptBuilder {
    base_path: Option<PathBuf>,
    workflow: Arc<Workflow>,
}

impl ConfiguredPromptBuilder {
    pub fn new(base_path: Option<PathBuf>, workflow: Arc<Workflow>) -> Self {
        Self {
            base_path,
            workflow,
        }
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
        let prompt_files = prompt_files_for_stage(stage_def, self.workflow.config());
        let base_prompt = load_prompts(&prompt_files, self.base_path.as_ref())?;
        build_full_prompt(
            &base_prompt,
            stage_def.role_name().unwrap_or(""),
            task_id,
            task_backend,
            self.workflow.config(),
        )
        .await
    }
}

/// Collect prompt file paths from a StageDefinition.
/// If no main_prompt is specified, tries the role's prompt from the workflow config.
/// Relative paths are prefixed with `workflow.prompts_dir` when set.
pub fn prompt_files_for_stage(
    stage_def: &StageDefinition,
    workflow: &WorkflowConfig,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Some(ref main) = stage_def.main_prompt {
        files.push(main.clone());
    } else if let Some(role_def) = stage_def
        .role_name()
        .and_then(|r| workflow.role_definition(r))
    {
        if let Some(ref prompt_path) = role_def.prompt {
            files.push(prompt_path.clone());
        }
    }
    files.extend(stage_def.additional_prompts.iter().cloned());
    if let Some(ref prompts_dir) = workflow.prompts_dir {
        files = files
            .into_iter()
            .map(|p| {
                if p.is_relative() {
                    prompts_dir.join(&p)
                } else {
                    p
                }
            })
            .collect();
    }
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

/// Strip `[tool_name]\n` prefix line from comment text if present.
pub fn strip_tool_prefix(text: &str) -> &str {
    let first_line = text.lines().next().unwrap_or("");
    if first_line.starts_with('[') && first_line.ends_with(']') {
        // Skip the prefix line and the following newline
        let rest = &text[first_line.len()..];
        rest.strip_prefix('\n').unwrap_or(rest)
    } else {
        text
    }
}

/// Build template variables from task and comments.
pub fn build_template_variables<'a>(
    task: &'a Task,
    comments: &'a [Comment],
) -> HashMap<Cow<'static, str>, Cow<'a, str>> {
    let mut vars: HashMap<Cow<'static, str>, Cow<'a, str>> = HashMap::new();

    // Task fields
    vars.insert(Cow::Borrowed(VAR_TITLE), Cow::Borrowed(&task.title));
    vars.insert(
        Cow::Borrowed(VAR_DESCRIPTION),
        Cow::Borrowed(&task.description),
    );
    if let Some(ref v) = task.destination_repository {
        vars.insert(Cow::Borrowed(VAR_DESTINATION_REPOSITORY), Cow::Borrowed(v));
    }
    if let Some(ref v) = task.destination_branch {
        vars.insert(Cow::Borrowed(VAR_DESTINATION_BRANCH), Cow::Borrowed(v));
    }
    if let Some(ref v) = task.work_branch {
        vars.insert(Cow::Borrowed(VAR_WORK_BRANCH), Cow::Borrowed(v));
    }

    // Checklist: unchecked items
    let unchecked: Vec<_> = task.checklist.iter().filter(|item| !item.checked).collect();
    if unchecked.is_empty() {
        vars.insert(Cow::Borrowed(VAR_CHECKLIST), Cow::Borrowed(""));
    } else {
        let mut checklist_text = String::new();
        for item in &unchecked {
            if !checklist_text.is_empty() {
                checklist_text.push('\n');
            }
            checklist_text.push_str(&format!("- [ ] [id: {}] {}", item.id, item.text));
        }
        vars.insert(Cow::Borrowed(VAR_CHECKLIST), Cow::Owned(checklist_text));
    }

    // last_report: last Success or Failure comment (stripped tool prefix)
    let last_report = comments
        .iter()
        .rev()
        .find(|c| {
            let t = classify_comment(&c.text);
            t == HistoryRecordType::Success || t == HistoryRecordType::Failure
        })
        .map(|c| strip_tool_prefix(&c.text))
        .unwrap_or("");
    vars.insert(Cow::Borrowed(VAR_LAST_REPORT), Cow::Borrowed(last_report));

    // last_request: last user comment (Other type), fallback to task.description
    let last_request = comments
        .iter()
        .rev()
        .find(|c| classify_comment(&c.text) == HistoryRecordType::Other)
        .map(|c| c.text.as_str())
        .unwrap_or(&task.description);
    vars.insert(Cow::Borrowed(VAR_LAST_REQUEST), Cow::Borrowed(last_request));

    vars
}

/// Insert `mcp_{name}` → `name` for each tool in the allowed set.
pub fn add_mcp_tool_variables<'a>(
    vars: &mut HashMap<Cow<'static, str>, Cow<'a, str>>,
    allowed_tools: &'a [McpTool],
) {
    for tool_name in allowed_tools {
        let tool_name_str = tool_name.as_str();
        let key = format!("mcp_{tool_name_str}");
        vars.insert(Cow::Owned(key), Cow::Borrowed(tool_name_str));
    }
}

/// Build full prompt by loading task data, building template variables,
/// and rendering the template from prompt files.
pub async fn build_full_prompt(
    user_context: &str,
    role_name: &str,
    task_id: u64,
    task_backend: &dyn TaskBackend,
    workflow: &WorkflowConfig,
) -> anyhow::Result<String> {
    let weak = task_backend.get_task(task_id).await?;
    let task = weak.snapshot(false).await?;
    let comments = weak.get_comments().await?;
    let mut vars = build_template_variables(&task, &comments);

    // Look up allowed tools for this role; fall back to all static tools.
    let allowed_tools: Vec<McpTool> = workflow
        .role_definition(role_name)
        .map(|d| d.mcp.clone())
        .unwrap_or_else(|| McpTool::all().to_vec());
    add_mcp_tool_variables(&mut vars, &allowed_tools);

    // Convert to owned HashMap for Interpolation
    let owned_vars: HashMap<Cow<str>, Cow<str>> = vars
        .into_iter()
        .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
        .collect();
    let template = Interpolation::new(user_context)
        .map_err(|e| anyhow::anyhow!("Failed to parse prompt template: {e}"))?;
    template
        .try_render(&owned_vars)
        .map_err(|e| anyhow::anyhow!("Failed to render prompt template: {e}"))
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
    use zbobr_api::ChecklistItem;

    use super::*;

    fn dummy_task(title: &str) -> Task {
        Task {
            id: 1,
            title: title.to_owned(),
            description: String::new(),
            state: "READY".into(),
            destination_repository: None,
            destination_branch: None,
            work_branch: None,
            pr_url: None,
            checklist: vec![],
            signal: None,
            stack: vec![],
            pause: false,
            confirm: false,
            pipeline_run_id: 0,
            etag: None,
        }
    }

    fn dummy_comment(text: &str) -> Comment {
        Comment {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            stage: "working".to_string(),
            hostname: "test".to_string(),
            tool: None,
            model: None,
            text: text.to_string(),
            pipeline: String::new(),
            pipeline_run_id: 0,
            caller_pipeline: None,
            caller_pipeline_run_id: None,
            report_name: None,
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

    // --- strip_tool_prefix ---

    #[test]
    fn strip_tool_prefix_removes_prefix() {
        assert_eq!(strip_tool_prefix("[report_success]\nAll good"), "All good");
    }

    #[test]
    fn strip_tool_prefix_no_prefix() {
        assert_eq!(strip_tool_prefix("Just text"), "Just text");
    }

    #[test]
    fn strip_tool_prefix_empty() {
        assert_eq!(strip_tool_prefix(""), "");
    }

    #[test]
    fn strip_tool_prefix_prefix_only() {
        assert_eq!(strip_tool_prefix("[report_success]"), "");
    }

    // --- build_template_variables ---

    #[test]
    fn build_template_variables_has_all_keys() {
        let task = dummy_task("Test");
        let vars = build_template_variables(&task, &[]);
        let keys: Vec<&str> = vars.keys().map(|k| k.as_ref()).collect();
        // Always-present keys
        for expected in &[
            VAR_TITLE,
            VAR_DESCRIPTION,
            VAR_CHECKLIST,
            VAR_LAST_REPORT,
            VAR_LAST_REQUEST,
        ] {
            assert!(keys.contains(expected), "missing key: {expected}");
        }
        // Optional keys absent when task fields are None
        for absent in &[
            VAR_DESTINATION_REPOSITORY,
            VAR_DESTINATION_BRANCH,
            VAR_WORK_BRANCH,
        ] {
            assert!(!keys.contains(absent), "key should be absent: {absent}");
        }
    }

    #[test]
    fn build_template_variables_task_fields() {
        let mut task = dummy_task("My Task");
        task.description = "Task desc".to_string();
        task.destination_repository = Some("owner/repo".to_string());
        task.destination_branch = Some("main".to_string());
        task.work_branch = Some("feature-x".to_string());

        let vars = build_template_variables(&task, &[]);
        assert_eq!(
            vars[&Cow::Borrowed(VAR_TITLE) as &Cow<str>].as_ref(),
            "My Task"
        );
        assert_eq!(
            vars[&Cow::Borrowed(VAR_DESCRIPTION) as &Cow<str>].as_ref(),
            "Task desc"
        );
        assert_eq!(
            vars[&Cow::Borrowed(VAR_DESTINATION_REPOSITORY) as &Cow<str>].as_ref(),
            "owner/repo"
        );
        assert_eq!(
            vars[&Cow::Borrowed(VAR_DESTINATION_BRANCH) as &Cow<str>].as_ref(),
            "main"
        );
        assert_eq!(
            vars[&Cow::Borrowed(VAR_WORK_BRANCH) as &Cow<str>].as_ref(),
            "feature-x"
        );
    }

    #[test]
    fn build_template_variables_checklist() {
        let mut task = dummy_task("T");
        task.checklist = vec![
            ChecklistItem {
                id: "1".to_string(),
                checked: true,
                text: "Done item".to_string(),
            },
            ChecklistItem {
                id: "2".to_string(),
                checked: false,
                text: "Todo item".to_string(),
            },
            ChecklistItem {
                id: "3".to_string(),
                checked: false,
                text: "Another".to_string(),
            },
        ];
        let vars = build_template_variables(&task, &[]);
        let checklist = vars[&Cow::Borrowed(VAR_CHECKLIST) as &Cow<str>].as_ref();
        assert!(checklist.contains("- [ ] [id: 2] Todo item"));
        assert!(checklist.contains("- [ ] [id: 3] Another"));
        assert!(!checklist.contains("Done item"));
    }

    #[test]
    fn build_template_variables_last_report() {
        let task = dummy_task("T");
        let comments = vec![
            dummy_comment("user request"),
            dummy_comment("[report_success]\nAll tests passed"),
            dummy_comment("another user msg"),
        ];
        let vars = build_template_variables(&task, &comments);
        assert_eq!(
            vars[&Cow::Borrowed(VAR_LAST_REPORT) as &Cow<str>].as_ref(),
            "All tests passed"
        );
    }

    #[test]
    fn build_template_variables_last_report_failure() {
        let task = dummy_task("T");
        let comments = vec![dummy_comment("[report_failure]\nBuild failed")];
        let vars = build_template_variables(&task, &comments);
        assert_eq!(
            vars[&Cow::Borrowed(VAR_LAST_REPORT) as &Cow<str>].as_ref(),
            "Build failed"
        );
    }

    #[test]
    fn build_template_variables_last_request_from_comments() {
        let task = dummy_task("T");
        let comments = vec![
            dummy_comment("Please fix the bug"),
            dummy_comment("[report_success]\nDone"),
        ];
        let vars = build_template_variables(&task, &comments);
        assert_eq!(
            vars[&Cow::Borrowed(VAR_LAST_REQUEST) as &Cow<str>].as_ref(),
            "Please fix the bug"
        );
    }

    #[test]
    fn build_template_variables_last_request_fallback_to_description() {
        let mut task = dummy_task("T");
        task.description = "Implement feature X".to_string();
        let comments = vec![dummy_comment("[report_success]\nDone")];
        let vars = build_template_variables(&task, &comments);
        assert_eq!(
            vars[&Cow::Borrowed(VAR_LAST_REQUEST) as &Cow<str>].as_ref(),
            "Implement feature X"
        );
    }

    // --- template rendering ---

    #[test]
    fn template_with_placeholder_renders() {
        let template_str = "Task: {title}\nDesc: {description}";
        let task = dummy_task("My Task");
        let vars = build_template_variables(&task, &[]);
        let owned_vars: HashMap<Cow<str>, Cow<str>> = vars
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        let template = Interpolation::new(template_str).unwrap();
        let result = template.render(&owned_vars);
        assert_eq!(result, "Task: My Task\nDesc: ");
    }

    #[test]
    fn template_no_placeholders_passthrough() {
        let template_str = "No placeholders here";
        let task = dummy_task("T");
        let vars = build_template_variables(&task, &[]);
        let owned_vars: HashMap<Cow<str>, Cow<str>> = vars
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        let template = Interpolation::new(template_str).unwrap();
        let result = template.render(&owned_vars);
        assert_eq!(result, "No placeholders here");
    }

    // --- load_prompts + template integration ---

    #[test]
    fn load_prompts_content_renders_with_template() {
        let dir = TempDir::new().unwrap();
        let path = write_file(&dir, "worker.md", "do the work on {title}");
        let loaded = load_prompts(&[path], None).unwrap();
        let mut task = dummy_task("Feature X");
        task.description = "Build it".to_string();
        let vars = build_template_variables(&task, &[]);
        let owned_vars: HashMap<Cow<str>, Cow<str>> = vars
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        let template = Interpolation::new(&loaded).unwrap();
        let result = template.render(&owned_vars);
        assert!(result.contains("do the work on Feature X"));
    }

    #[test]
    fn no_prompt_files_gives_empty_template() {
        let loaded = load_prompts(&[] as &[PathBuf], None).unwrap();
        let task = dummy_task("T");
        let vars = build_template_variables(&task, &[]);
        let owned_vars: HashMap<Cow<str>, Cow<str>> = vars
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        let template = Interpolation::new(&loaded).unwrap();
        let result = template.render(&owned_vars);
        assert_eq!(result, "");
    }

    #[test]
    fn load_prompts_with_base_path_resolves_relative_files() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "reviewer.md", "review {title} carefully");
        let loaded = load_prompts(
            &[PathBuf::from("reviewer.md")],
            Some(&dir.path().to_path_buf()),
        )
        .unwrap();
        let template = Interpolation::new(&loaded).unwrap();
        let task = dummy_task("PR-42");
        let vars = build_template_variables(&task, &[]);
        let owned_vars: HashMap<Cow<str>, Cow<str>> = vars
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        let result = template.render(&owned_vars);
        assert!(result.contains("review PR-42 carefully"));
    }

    // --- MCP tool variables ---

    #[test]
    fn mcp_tool_variables_added() {
        let allowed = vec![McpTool::ReportSuccess, McpTool::StopWithError];
        let mut vars: HashMap<Cow<'static, str>, Cow<str>> = HashMap::new();
        add_mcp_tool_variables(&mut vars, &allowed);
        assert_eq!(
            vars[&Cow::Borrowed("mcp_report_success") as &Cow<str>].as_ref(),
            "report_success"
        );
        assert_eq!(
            vars[&Cow::Borrowed("mcp_stop_with_error") as &Cow<str>].as_ref(),
            "stop_with_error"
        );
        assert!(!vars.contains_key(&Cow::Borrowed("mcp_configure_worktree") as &Cow<str>));
    }

    #[test]
    fn undefined_mcp_tool_errors() {
        let template_str = "Use {mcp_nonexistent} tool";
        let template = Interpolation::new(template_str).unwrap();
        let vars: HashMap<Cow<str>, Cow<str>> = HashMap::new();
        let result = template.try_render(&vars);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("mcp_nonexistent"),
            "error should name the undefined variable: {err}"
        );
    }

    #[test]
    fn mcp_tool_renders_correctly() {
        let template_str = "Call `{mcp_report_success}` when done";
        let allowed = vec![McpTool::ReportSuccess];
        let mut vars: HashMap<Cow<'static, str>, Cow<str>> = HashMap::new();
        add_mcp_tool_variables(&mut vars, &allowed);
        let owned_vars: HashMap<Cow<str>, Cow<str>> = vars
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        let template = Interpolation::new(template_str).unwrap();
        let result = template.try_render(&owned_vars).unwrap();
        assert_eq!(result, "Call `report_success` when done");
    }

    #[test]
    fn unavailable_tool_for_role_errors() {
        // A role that only has report_success should fail if prompt uses configure_worktree
        let template_str = "Use {mcp_configure_worktree} to set up";
        let allowed = vec![McpTool::ReportSuccess];
        let mut vars: HashMap<Cow<'static, str>, Cow<str>> = HashMap::new();
        add_mcp_tool_variables(&mut vars, &allowed);
        let owned_vars: HashMap<Cow<str>, Cow<str>> = vars
            .into_iter()
            .map(|(k, v)| (k, Cow::Owned(v.into_owned())))
            .collect();
        let template = Interpolation::new(template_str).unwrap();
        let result = template.try_render(&owned_vars);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("mcp_configure_worktree"),
            "error should name the unavailable tool: {err}"
        );
    }
}
