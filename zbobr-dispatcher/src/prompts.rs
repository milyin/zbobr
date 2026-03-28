use std::{borrow::Cow, collections::HashMap, path::PathBuf, sync::Arc};

use simpleinterpolation::Interpolation;
use zbobr_api::{
    Comment, PARAM_DESTINATION_BRANCH, PARAM_DESTINATION_REPOSITORY, PARAM_WORK_BRANCH, Task,
    config::{StageDefinition, WorkflowConfig},
    config_tools::McpTool,
    context::serialize_context,
};

use crate::{backend::TaskBackend, workflow::Workflow};

// Template placeholder names used in prompt .md files.
pub const VAR_TITLE: &str = "title";
pub const VAR_DESCRIPTION: &str = "description";
pub const VAR_DESTINATION_REPOSITORY: &str = PARAM_DESTINATION_REPOSITORY;
pub const VAR_DESTINATION_BRANCH: &str = PARAM_DESTINATION_BRANCH;
pub const VAR_WORK_BRANCH: &str = PARAM_WORK_BRANCH;
pub const VAR_CONTEXT: &str = "context";

#[derive(Clone)]
pub struct ConfiguredPromptBuilder {
    base_path: Option<PathBuf>,
    workflow: Arc<Workflow>,
    extra_vars: HashMap<String, String>,
}

impl ConfiguredPromptBuilder {
    pub fn new(base_path: Option<PathBuf>, workflow: Arc<Workflow>) -> Self {
        Self {
            base_path,
            workflow,
            extra_vars: HashMap::new(),
        }
    }

    /// Add an extra template variable that will be available in all prompts.
    pub fn with_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_vars.insert(key.into(), value.into());
        self
    }

    pub fn base_path(&self) -> Option<&PathBuf> {
        self.base_path.as_ref()
    }

    pub fn workflow_config(&self) -> &WorkflowConfig {
        self.workflow.config()
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
            &self.extra_vars,
        )
        .await
    }

    /// Build prompt for a stage using the given task and comments
    /// instead of fetching from the backend.
    pub fn build_for_stage_with_task(
        &self,
        stage_def: &StageDefinition,
        task: &Task,
        comments: &[Comment],
    ) -> anyhow::Result<String> {
        let prompt_files = prompt_files_for_stage(stage_def, self.workflow.config());
        let base_prompt = load_prompts(&prompt_files, self.base_path.as_ref())?;
        let role_name = stage_def.role_name().unwrap_or("");
        build_prompt_with_task(
            &base_prompt,
            role_name,
            task,
            comments,
            self.workflow.config(),
            &self.extra_vars,
        )
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
    if let Some(ref main) = stage_def.role_prompt {
        files.push(main.clone());
    } else if let Some(role_def) = stage_def
        .role_name()
        .and_then(|r| workflow.role_definition(r))
    {
        if let Some(ref prompt_path) = role_def.prompt {
            files.push(prompt_path.clone());
        }
    }
    files.extend(stage_def.prompts.iter().cloned());
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

    // context: serialized TaskContext for prompt (with for_prompt=true)
    let context_md = serialize_context(&task.context, comments, true, None);
    vars.insert(Cow::Borrowed(VAR_CONTEXT), Cow::Owned(context_md));

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
    extra_vars: &HashMap<String, String>,
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

    // Inject extra variables from config.
    for (k, v) in extra_vars {
        vars.insert(Cow::Owned(k.clone()), Cow::Owned(v.clone()));
    }

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

/// Build prompt using a provided task and comments (no backend needed).
pub fn build_prompt_with_task(
    user_context: &str,
    role_name: &str,
    task: &Task,
    comments: &[Comment],
    workflow: &WorkflowConfig,
    extra_vars: &HashMap<String, String>,
) -> anyhow::Result<String> {
    let mut vars = build_template_variables(task, comments);

    let allowed_tools: Vec<McpTool> = workflow
        .role_definition(role_name)
        .map(|d| d.mcp.clone())
        .unwrap_or_else(|| McpTool::all().to_vec());
    add_mcp_tool_variables(&mut vars, &allowed_tools);

    for (k, v) in extra_vars {
        vars.insert(Cow::Owned(k.clone()), Cow::Owned(v.clone()));
    }

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
    use zbobr_api::task::TaskContext;

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
            context: TaskContext::default(),
            signal: None,
            stack: vec![],
            error: None,
            pause: false,
            confirm: false,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: 0,
            closed: false,
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

    // --- build_template_variables ---

    #[test]
    fn build_template_variables_has_all_keys() {
        let task = dummy_task("Test");
        let vars = build_template_variables(&task, &[]);
        let keys: Vec<&str> = vars.keys().map(|k| k.as_ref()).collect();
        // Always-present keys
        for expected in &[VAR_TITLE, VAR_DESCRIPTION, VAR_CONTEXT] {
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
            McpTool::ReportSuccess.as_str()
        );
        assert_eq!(
            vars[&Cow::Borrowed("mcp_stop_with_error") as &Cow<str>].as_ref(),
            McpTool::StopWithError.as_str()
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
