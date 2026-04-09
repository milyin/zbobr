use std::{borrow::Cow, collections::HashMap, path::PathBuf, sync::Arc};

use simpleinterpolation::Interpolation;
use zbobr_api::{
    Comment, ContextRecord, ContextRecordType, Signal, StackEntry, StageContext, StageInfo, Task,
    TaskContext,
    config::{Role, StageDefinition, WorkflowConfig},
    config_tools::McpTool,
    context::serialize_context,
    task::{DEFAULT_MAX_STAGE_COUNT, Executor, Pipeline, Stage},
};
use zbobr_utility::TomlOption;

use crate::{backend::TaskBackend, workflow::Workflow};

// Template placeholder names used in prompt .md files.
pub const VAR_TITLE: &str = "title";
pub const VAR_DESCRIPTION: &str = "description";
pub const VAR_DESTINATION_REPOSITORY: &str = "destination_repository";
pub const VAR_DESTINATION_BRANCH: &str = "destination_branch";
pub const VAR_WORK_BRANCH: &str = "work_branch";
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
            stage_def.role().map_or("", |r| r.as_str()),
            task_id,
            task_backend,
            self.workflow.config(),
            &self.extra_vars,
        )
        .await
    }

    /// Validate all prompts by rendering every stage's prompt with sample data.
    /// Catches template parse errors and undefined variables at startup.
    pub fn validate_all_prompts(&self) -> anyhow::Result<()> {
        let (task, comments) = sample_task_and_comments();
        let mut errors: Vec<String> = Vec::new();

        for (pipeline, stage_name, stage_def) in self.workflow.config().all_stages() {
            if stage_def.is_call() {
                continue;
            }
            if let Err(e) = self.build_for_stage_with_task(stage_def, &task, &comments) {
                errors.push(format!(
                    "  pipeline '{}', stage '{}': {}",
                    pipeline, stage_name, e
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Prompt validation failed:\n{}",
                errors.join("\n")
            ))
        }
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
        let default_role = Role::new("");
        let role = stage_def.role().unwrap_or(&default_role);
        build_prompt_with_task(
            &base_prompt,
            role,
            task,
            comments,
            self.workflow.config(),
            &self.extra_vars,
        )
    }
}

/// Returns a representative task and comments for prompt previewing and validation.
///
/// The values are non-trivial so that template variables such as `context`,
/// `signal`, `stack`, `pr_url`, and comment `url` fields are exercised.
pub fn sample_task_and_comments() -> (Task, Vec<Comment>) {
    const SAMPLE_REPO_URL: &str = "https://github.com/example/repo";
    const SAMPLE_ISSUE_URL: &str = "https://github.com/example/repo/issues/1";
    let stage_info = StageInfo {
        instance: "zbobr".to_string(),
        pipeline: Pipeline::from("main"),
        run_id: 1,
        stage: Stage::new("planning"),
        tool: Some(Executor::CLAUDE.to_string()).into(),
        model: None,
        prompt_link: None,
        output_link: None,
        timestamp: "2025-01-01T00:00:00+00:00".parse().unwrap(),
    };
    let context = TaskContext {
        stages: vec![StageContext {
            info: stage_info,
            records: vec![ContextRecord {
                id: 1,
                record_type: ContextRecordType::Success,
                brief: "Plan approved and ready for implementation".to_string(),
                report_link: Some(format!("{SAMPLE_ISSUE_URL}#issuecomment-100")),
            }],
        }],
    };
    let task = Task {
        id: 1,
        title: "TITLE".to_string(),
        description: "DESCRIPTION".to_string(),
        state: zbobr_api::State::Ready,
        work_branch: Some("zbobr_fix-1-sample-task".to_string()),
        pr_url: Some(format!("{SAMPLE_REPO_URL}/pull/42")),
        context,
        signal: Some(Signal::Go(Stage::new("working"))),
        stack: vec![StackEntry {
            pipeline: Pipeline::from("parent"),
            pipeline_run_id: 1,
            signal: Signal::Go(Stage::new("done")),
        }],
        status: None,
        go_pause: false,
        confirm: false,
        pipeline_run_id: 1,
        stage_count: 2,
        max_stage_count: DEFAULT_MAX_STAGE_COUNT,
        closed: false,
        etag: None,
    };
    let comments = vec![
        Comment {
            timestamp: "2025-01-01T00:00:00Z".parse().unwrap(),
            username: "user".to_string(),
            body: "USER_REQUEST".to_string(),
            url: Some(format!("{SAMPLE_ISSUE_URL}#issuecomment-200")),
        },
        Comment {
            timestamp: "2025-01-01T01:00:00Z".parse().unwrap(),
            username: "zbobr[bot]".to_string(),
            body: "[report_success]\nREPORT".to_string(),
            url: Some(format!("{SAMPLE_ISSUE_URL}#issuecomment-201")),
        },
    ];
    (task, comments)
}

/// Collect prompt file paths from a StageDefinition using three-level map merge.
///
/// Merges workflow-level, role-level, and stage-level `prompts` maps in order.
/// Slots set to `nan` (ExplicitNone) are cleared. Remaining `Value` paths are
/// collected in insertion order.
pub fn prompt_files_for_stage(
    stage_def: &StageDefinition,
    workflow: &WorkflowConfig,
) -> Vec<PathBuf> {
    // Start with workflow-level prompts.
    let mut merged: indexmap::IndexMap<String, TomlOption<std::path::PathBuf>> =
        workflow.prompts.clone().unwrap_or_default();

    // Merge role-level prompts per key (preserve insertion order).
    if let Some(role_def) = stage_def
        .role()
        .map(|r| r.as_str())
        .and_then(|r| workflow.role_definition(r))
        && let Some(ref role_prompts) = role_def.prompts
    {
        for (k, v) in role_prompts {
            if let Some(base_v) = merged.get_mut(k) {
                *base_v = base_v.clone().merge(v.clone());
            } else {
                merged.insert(k.clone(), v.clone());
            }
        }
    }

    // Merge stage-level prompts per key (preserve insertion order).
    if let Some(ref stage_prompts) = stage_def.prompts {
        for (k, v) in stage_prompts {
            if let Some(base_v) = merged.get_mut(k) {
                *base_v = base_v.clone().merge(v.clone());
            } else {
                merged.insert(k.clone(), v.clone());
            }
        }
    }

    // Collect only Value paths (filter out ExplicitNone and Absent).
    merged
        .into_values()
        .filter_map(|v| v.into_option())
        .collect()
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

    let allowed_tools: &[McpTool] = workflow
        .role_definition(role_name)
        .and_then(|d| d.mcp.as_deref())
        .unwrap_or(&[]);
    add_mcp_tool_variables(&mut vars, allowed_tools);

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
    role: &Role,
    task: &Task,
    comments: &[Comment],
    workflow: &WorkflowConfig,
    extra_vars: &HashMap<String, String>,
) -> anyhow::Result<String> {
    let mut vars = build_template_variables(task, comments);

    let allowed_tools: &[McpTool] = workflow
        .role_definition(role)
        .and_then(|d| d.mcp.as_deref())
        .unwrap_or(&[]);
    add_mcp_tool_variables(&mut vars, allowed_tools);

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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, io::Write, sync::Arc};

    use indexmap::IndexMap;
    use tempfile::TempDir;
    use zbobr_api::{
        config::{PipelineConfig, StageDefinition},
        task::{Pipeline, Stage, TaskContext},
    };

    use super::*;
    use crate::workflow::Workflow;

    fn dummy_task(title: &str) -> Task {
        Task {
            id: 1,
            title: title.to_owned(),
            description: String::new(),
            state: "READY".into(),
            work_branch: None,
            pr_url: None,
            context: TaskContext::default(),
            signal: None,
            stack: vec![],
            status: None,
            go_pause: false,
            confirm: false,
            pipeline_run_id: 0,
            stage_count: 0,
            max_stage_count: DEFAULT_MAX_STAGE_COUNT,
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
        {
            let absent = &VAR_WORK_BRANCH;
            assert!(!keys.contains(absent), "key should be absent: {absent}");
        }
    }

    #[test]
    fn build_template_variables_task_fields() {
        let mut task = dummy_task("My Task");
        task.description = "Task desc".to_string();
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

    // --- validate_all_prompts ---

    /// Helper: build a `ConfiguredPromptBuilder` from a single-pipeline workflow config.
    fn make_prompt_builder(
        base_path: Option<PathBuf>,
        stages: IndexMap<Stage, StageDefinition>,
    ) -> ConfiguredPromptBuilder {
        let mut pipelines = IndexMap::new();
        pipelines.insert(
            Pipeline::from("main"),
            zbobr_utility::TomlOption::Value(PipelineConfig {
                stages: Some(stages.into_iter().map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v))).collect()),
            }),
        );
        let config = WorkflowConfig {
            prompts: None,
            pipelines: Some(pipelines),
            roles: Some(indexmap::IndexMap::new()),
        };
        let workflow = Arc::new(Workflow::from_config(config));
        ConfiguredPromptBuilder::new(base_path, workflow)
    }

    fn stage_prompts_map(path: PathBuf) -> Option<indexmap::IndexMap<String, TomlOption<PathBuf>>> {
        Some(indexmap::IndexMap::from([(
            "main".to_string(),
            TomlOption::Value(path),
        )]))
    }

    #[test]
    fn validate_all_prompts_valid_templates_pass() {
        let dir = TempDir::new().unwrap();
        let prompt_path = write_file(&dir, "worker.md", "Do the work on {title}");
        let mut stages = IndexMap::new();
        stages.insert(
            Stage::from("work"),
            StageDefinition {
                role: Some("default".to_string().into()).into(),
                prompts: stage_prompts_map(prompt_path),
                ..Default::default()
            },
        );
        let builder = make_prompt_builder(None, stages);
        assert!(builder.validate_all_prompts().is_ok());
    }

    #[test]
    fn validate_all_prompts_undefined_variable_fails() {
        let dir = TempDir::new().unwrap();
        let prompt_path = write_file(&dir, "bad.md", "Use {mcp_nonexistent} tool");
        let mut stages = IndexMap::new();
        stages.insert(
            Stage::from("work"),
            StageDefinition {
                role: Some("default".to_string().into()).into(),
                prompts: stage_prompts_map(prompt_path),
                ..Default::default()
            },
        );
        let builder = make_prompt_builder(None, stages);
        let result = builder.validate_all_prompts();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("mcp_nonexistent"),
            "error should mention the undefined variable: {err}"
        );
    }

    #[test]
    fn validate_all_prompts_missing_file_fails() {
        let mut stages = IndexMap::new();
        stages.insert(
            Stage::from("work"),
            StageDefinition {
                role: Some("default".to_string().into()).into(),
                prompts: stage_prompts_map(PathBuf::from("/nonexistent/prompt.md")),
                ..Default::default()
            },
        );
        let builder = make_prompt_builder(None, stages);
        let result = builder.validate_all_prompts();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Failed to read prompt file"),
            "error should mention missing file: {err}"
        );
    }

    #[test]
    fn validate_all_prompts_aggregates_multiple_errors() {
        let dir = TempDir::new().unwrap();
        // Stage 1: missing file error
        // Stage 2: undefined variable error
        let bad_var_path = write_file(&dir, "bad_var.md", "Use {mcp_nonexistent} tool");
        let mut stages = IndexMap::new();
        stages.insert(
            Stage::from("stage_a"),
            StageDefinition {
                role: Some("default".to_string().into()).into(),
                prompts: stage_prompts_map(PathBuf::from("/nonexistent/prompt.md")),
                ..Default::default()
            },
        );
        stages.insert(
            Stage::from("stage_b"),
            StageDefinition {
                role: Some("default".to_string().into()).into(),
                prompts: stage_prompts_map(bad_var_path),
                ..Default::default()
            },
        );
        let builder = make_prompt_builder(None, stages);
        let result = builder.validate_all_prompts();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("stage_a"),
            "error should mention first failing stage: {err}"
        );
        assert!(
            err.contains("stage_b"),
            "error should mention second failing stage: {err}"
        );
    }

    /// Helper: build a `ConfiguredPromptBuilder` from a multi-pipeline workflow config.
    fn make_prompt_builder_multi(
        base_path: Option<PathBuf>,
        pipelines: IndexMap<Pipeline, PipelineConfig>,
    ) -> ConfiguredPromptBuilder {
        let config = WorkflowConfig {
            prompts: None,
            pipelines: Some(pipelines.into_iter().map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v))).collect()),
            roles: Some(indexmap::IndexMap::new()),
        };
        let workflow = Arc::new(Workflow::from_config(config));
        ConfiguredPromptBuilder::new(base_path, workflow)
    }

    #[test]
    fn validate_all_prompts_multi_pipeline() {
        let dir = TempDir::new().unwrap();
        let valid_path = write_file(&dir, "ok.md", "Do {title}");

        let mut main_stages = IndexMap::new();
        main_stages.insert(
            Stage::from("work"),
            StageDefinition {
                role: Some("default".to_string().into()).into(),
                prompts: stage_prompts_map(valid_path),
                ..Default::default()
            },
        );

        let mut secondary_stages = IndexMap::new();
        secondary_stages.insert(
            Stage::from("broken"),
            StageDefinition {
                role: Some("default".to_string().into()).into(),
                prompts: stage_prompts_map(PathBuf::from("/nonexistent/secondary_prompt.md")),
                ..Default::default()
            },
        );

        let mut pipelines = IndexMap::new();
        pipelines.insert(
            Pipeline::from("main"),
            PipelineConfig {
                stages: Some(main_stages.into_iter().map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v))).collect()),
            },
        );
        pipelines.insert(
            Pipeline::from("secondary"),
            PipelineConfig {
                stages: Some(secondary_stages.into_iter().map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v))).collect()),
            },
        );

        let builder = make_prompt_builder_multi(None, pipelines);
        let result = builder.validate_all_prompts();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("pipeline 'secondary'"),
            "error should identify the failing pipeline: {err}"
        );
        assert!(
            err.contains("stage 'broken'"),
            "error should identify the failing stage: {err}"
        );
    }

    #[test]
    fn validate_all_prompts_call_stages_skipped() {
        let mut stages = IndexMap::new();
        // A call stage has no prompt files — should not cause an error.
        stages.insert(
            Stage::from("delegate"),
            StageDefinition {
                call: Some(Pipeline::from("sub")).into(),
                ..Default::default()
            },
        );
        let builder = make_prompt_builder(None, stages);
        assert!(builder.validate_all_prompts().is_ok());
    }

    #[test]
    fn sample_task_and_comments_has_nontrivial_fields() {
        let (task, comments) = sample_task_and_comments();

        assert!(task.pr_url.is_some(), "task.pr_url should be Some");
        assert!(task.signal.is_some(), "task.signal should be Some");
        assert!(!task.stack.is_empty(), "task.stack should be non-empty");
        assert!(
            !task.context.stages.is_empty(),
            "task.context.stages should be non-empty"
        );
        assert!(
            task.context.stages.iter().all(|s| !s.records.is_empty()),
            "every stage context should have at least one record"
        );
        for comment in &comments {
            assert!(
                comment.url.is_some(),
                "all comments should have url set, but found None for comment by '{}'",
                comment.username
            );
        }
    }

    // --- prompt_files_for_stage three-level merge semantics ---

    fn make_workflow_with_role_main_prompt(
        role_name: &str,
        prompt: Option<PathBuf>,
    ) -> WorkflowConfig {
        use indexmap::IndexMap;
        use zbobr_api::config::RoleDefinition;

        let prompts = prompt.map(|p| IndexMap::from([("main".to_string(), TomlOption::Value(p))]));
        let mut roles = IndexMap::new();
        roles.insert(
            role_name.to_string().into(),
            RoleDefinition {
                mcp: None,
                prompts,
                tool: Default::default(),
            },
        );
        WorkflowConfig {
            prompts: None,
            roles: Some(roles.into_iter().map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v))).collect()),
            pipelines: Some(indexmap::IndexMap::new()),
        }
    }

    #[test]
    fn prompt_files_for_stage_inherits_role_main_prompt() {
        // Stage has no prompts → inherits role's "main" slot.
        let workflow =
            make_workflow_with_role_main_prompt("worker", Some(PathBuf::from("/role/worker.md")));
        let stage = StageDefinition {
            role: Some("worker".to_string().into()).into(),
            ..Default::default()
        };
        let files = prompt_files_for_stage(&stage, &workflow);
        assert_eq!(files, vec![PathBuf::from("/role/worker.md")]);
    }

    #[test]
    fn prompt_files_for_stage_stage_nan_clears_role_slot() {
        // Stage sets "main" slot to nan → clears role-level prompt.
        let workflow =
            make_workflow_with_role_main_prompt("worker", Some(PathBuf::from("/role/worker.md")));
        let stage = StageDefinition {
            role: Some("worker".to_string().into()).into(),
            prompts: Some(indexmap::IndexMap::from([(
                "main".to_string(),
                TomlOption::ExplicitNone,
            )])),
            ..Default::default()
        };
        let files = prompt_files_for_stage(&stage, &workflow);
        assert!(
            files.is_empty(),
            "nan at stage level must clear the role-level main slot; got: {files:?}"
        );
    }

    #[test]
    fn prompt_files_for_stage_stage_overrides_role_slot() {
        // Stage overrides the "main" slot with a different path.
        let workflow =
            make_workflow_with_role_main_prompt("worker", Some(PathBuf::from("/role/worker.md")));
        let stage = StageDefinition {
            role: Some("worker".to_string().into()).into(),
            prompts: Some(indexmap::IndexMap::from([(
                "main".to_string(),
                TomlOption::Value(PathBuf::from("/stage/override.md")),
            )])),
            ..Default::default()
        };
        let files = prompt_files_for_stage(&stage, &workflow);
        assert_eq!(files, vec![PathBuf::from("/stage/override.md")]);
    }

    #[test]
    fn prompt_files_for_stage_three_level_merge() {
        // workflow: { task: "task.md" }, role: { main: "worker.md" }, stage: { task: nan }
        // Result: only ["worker.md"] (task cleared by stage).
        use indexmap::IndexMap;
        use zbobr_api::config::RoleDefinition;

        let mut roles = IndexMap::new();
        roles.insert(
            "worker".to_string().into(),
            RoleDefinition {
                mcp: None,
                prompts: Some(IndexMap::from([(
                    "main".to_string(),
                    TomlOption::Value(PathBuf::from("/prompts/worker.md")),
                )])),
                tool: Default::default(),
            },
        );
        let workflow = WorkflowConfig {
            prompts: Some(IndexMap::from([(
                "task".to_string(),
                TomlOption::Value(PathBuf::from("/prompts/task.md")),
            )])),
            roles: Some(roles.into_iter().map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v))).collect()),
            pipelines: Some(indexmap::IndexMap::new()),
        };
        let stage = StageDefinition {
            role: Some("worker".to_string().into()).into(),
            prompts: Some(IndexMap::from([(
                "task".to_string(),
                TomlOption::ExplicitNone,
            )])),
            ..Default::default()
        };
        let files = prompt_files_for_stage(&stage, &workflow);
        assert_eq!(files, vec![PathBuf::from("/prompts/worker.md")]);
    }

    #[test]
    fn prompt_files_for_stage_preserves_slot_order() {
        // Workflow establishes slot order: main (nan), task.
        // Role overrides main in-place → final order must be [worker.md, task.md], not reversed.
        use indexmap::IndexMap;
        use zbobr_api::config::RoleDefinition;

        let mut roles = IndexMap::new();
        roles.insert(
            "worker".to_string().into(),
            RoleDefinition {
                mcp: None,
                prompts: Some(IndexMap::from([(
                    "main".to_string(),
                    TomlOption::Value(PathBuf::from("/prompts/worker.md")),
                )])),
                tool: Default::default(),
            },
        );
        let workflow = WorkflowConfig {
            // "main" is seeded first (nan placeholder), then "task" — establishes canonical order.
            prompts: Some(IndexMap::from([
                ("main".to_string(), TomlOption::ExplicitNone),
                (
                    "task".to_string(),
                    TomlOption::Value(PathBuf::from("/prompts/task.md")),
                ),
            ])),
            roles: Some(roles.into_iter().map(|(k, v)| (k, zbobr_utility::TomlOption::Value(v))).collect()),
            pipelines: Some(indexmap::IndexMap::new()),
        };
        let stage = StageDefinition {
            role: Some("worker".to_string().into()).into(),
            ..Default::default()
        };
        let files = prompt_files_for_stage(&stage, &workflow);
        // worker.md must come before task.md — role's main slot is at the "main" position.
        assert_eq!(
            files,
            vec![
                PathBuf::from("/prompts/worker.md"),
                PathBuf::from("/prompts/task.md"),
            ],
            "slot order must be preserved: main before task"
        );
    }
}
