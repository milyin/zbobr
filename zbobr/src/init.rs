use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use indexmap::IndexMap;
use zbobr_api::{
    Pipeline, Secret, Stage,
    config::{
        PipelineConfig, ProviderDefinition, RoleDefinition, StageDefinition, StageTransition,
        ToolEntry, WorkflowConfig, WorkflowToml,
    },
    config_tools::McpTool,
};
use zbobr_dispatcher::config::{ZbobrDispatcherToml, ZbobrExecutorToml};
use zbobr_executor_copilot::ZbobrExecutorCopilotToml;
use zbobr_repo_backend_github::ZbobrRepoBackendGithubToml;
use zbobr_task_backend_github::ZbobrTaskBackendGithubToml;

use super::RootConfigToml;

// Default model names used by init workspace. Copilot paths use dot notation
// for some claude models, while actual Claude executor uses hyphen notation.
const COPILOT_MODEL_HAIKU: &str = "claude-haiku-4.5";
const CLAUDE_MODEL_HAIKU: &str = "claude-haiku-4-5";
const COPILOT_MODEL_SONNET: &str = "claude-sonnet-4.6";
const CLAUDE_MODEL_OPUS: &str = "claude-opus-4-6";
const CLAUDE_MODEL_SONNET: &str = "claude-sonnet-4.6";
const COPILOT_MODEL_GPT_5_4: &str = "gpt-5.4";
const COPILOT_MODEL_GPT_5_MINI: &str = "gpt-5-mini";

/// Initialize a new zbobr workspace at the given directory.
///
/// Creates the directory (if it does not exist), writes a complete `zbobr.toml`
/// config file, creates prompt files for each predefined role, and creates
/// the required subdirectories.
///
/// If a file already exists with different content, the new version is written
/// next to it as `{filename}.new` instead of overwriting or refusing.
pub async fn init_workspace(dest: &Path) -> anyhow::Result<()> {
    // Create destination directory
    tokio::fs::create_dir_all(dest).await?;

    // Create subdirectories
    let prompts_dir = dest.join("prompts");
    let workspaces_dir = dest.join("workspaces");
    let repos_dir = dest.join("repos");
    tokio::fs::create_dir_all(&prompts_dir).await?;
    tokio::fs::create_dir_all(&workspaces_dir).await?;
    tokio::fs::create_dir_all(&repos_dir).await?;

    // Write prompt files
    for (name, content) in PROMPT_FILES {
        let path = prompts_dir.join(format!("{name}.md"));
        write_or_new(&path, content).await?;
    }

    // Serialize with toml pretty-printer, then post-process with toml_edit
    // to convert stage definitions and dispatcher providers/tools into inline tables.
    let config = default_config_toml();
    let pretty = toml::to_string_pretty(&config)?;
    let mut doc: toml_edit::DocumentMut = pretty.parse()?;
    inline_stage_tables(&mut doc);
    inline_dispatcher_tables(&mut doc);
    let config_content = format!(
        "# zbobr configuration\n# See documentation for all available options.\n\n{}",
        doc
    );
    let config_path = dest.join("zbobr.toml");
    write_or_new(&config_path, &config_content).await?;

    println!(
        "\nWorkspace initialized at {}.\nEdit zbobr.toml to configure backends and tokens before running.",
        dest.display()
    );
    Ok(())
}

/// Write `content` to `path`. If the file already exists with identical content,
/// skip it. If it exists with different content, write to `{path}.new` instead.
async fn write_or_new(path: &Path, content: &str) -> anyhow::Result<()> {
    if path.exists() {
        let existing = tokio::fs::read_to_string(path).await?;
        if existing == content {
            println!("  unchanged {}", path.display());
            return Ok(());
        }
        let new_path = path.with_extension(format!(
            "{}.new",
            path.extension().unwrap_or_default().to_string_lossy()
        ));
        tokio::fs::write(&new_path, content).await?;
        println!("  wrote {} (existing file differs)", new_path.display());
    } else {
        tokio::fs::write(path, content).await?;
        println!("  wrote {}", path.display());
    }
    Ok(())
}

/// Build a default `RootConfigToml` with sensible example values.
fn default_config_toml() -> RootConfigToml {
    let workflow = default_workflow();

    let providers = IndexMap::from([
        (
            "claude".to_string(),
            ProviderDefinition {
                executor: Some("claude".to_string()),
                parent: None,
                priority: None,
                plan_mode: None,
                access_key: None,
            },
        ),
        (
            "copilot".to_string(),
            ProviderDefinition {
                executor: Some("copilot".to_string()),
                parent: None,
                priority: None,
                plan_mode: None,
                access_key: None,
            },
        ),
        (
            "claude_planner".to_string(),
            ProviderDefinition {
                executor: None,
                parent: Some("claude".to_string()),
                priority: None,
                plan_mode: Some(true),
                access_key: None,
            },
        ),
        (
            "copilot_planner".to_string(),
            ProviderDefinition {
                executor: None,
                parent: Some("copilot".to_string()),
                priority: None,
                plan_mode: Some(true),
                access_key: None,
            },
        ),
    ]);

    let tools = IndexMap::from([
        (
            "developer".to_string(),
            vec![
                ToolEntry {
                    provider: "claude".to_string(),
                    model: CLAUDE_MODEL_OPUS.parse().unwrap(),
                    priority: None,
                },
                ToolEntry {
                    provider: "copilot".to_string(),
                    model: COPILOT_MODEL_SONNET.parse().unwrap(),
                    priority: Some(0),
                },
            ],
        ),
        (
            "planner".to_string(),
            vec![
                ToolEntry {
                    provider: "claude_planner".to_string(),
                    model: CLAUDE_MODEL_OPUS.parse().unwrap(),
                    priority: None,
                },
                ToolEntry {
                    provider: "copilot_planner".to_string(),
                    model: COPILOT_MODEL_SONNET.parse().unwrap(),
                    priority: Some(0),
                },
            ],
        ),
        (
            "helper".to_string(),
            vec![
                ToolEntry {
                    provider: "copilot".to_string(),
                    model: COPILOT_MODEL_HAIKU.parse().unwrap(),
                    priority: None,
                },
                ToolEntry {
                    provider: "claude".to_string(),
                    model: CLAUDE_MODEL_HAIKU.parse().unwrap(),
                    priority: Some(0),
                },
            ],
        ),
        (
            "reviewer".to_string(),
            vec![
                ToolEntry {
                    provider: "copilot".to_string(),
                    model: COPILOT_MODEL_GPT_5_4.parse().unwrap(),
                    priority: None,
                },
                ToolEntry {
                    provider: "claude".to_string(),
                    model: CLAUDE_MODEL_SONNET.parse().unwrap(),
                    priority: Some(0),
                },
            ],
        ),
        (
            "drudge".to_string(),
            vec![
                ToolEntry {
                    provider: "copilot".to_string(),
                    model: COPILOT_MODEL_GPT_5_MINI.parse().unwrap(),
                    priority: None,
                },
                ToolEntry {
                    provider: "claude".to_string(),
                    model: CLAUDE_MODEL_HAIKU.parse().unwrap(),
                    priority: Some(0),
                },
            ],
        ),
    ]);

    RootConfigToml {
        dispatcher: Some(ZbobrDispatcherToml {
            instance: Some("default".into()),
            workspaces: Some(PathBuf::from("./workspaces")),
            base_port: Some(3000),
            agent_github_token: Some(Secret::value("not-configured")),
            providers: Some(providers),
            tools: Some(tools),
            provider_exclusion_secs: Some(3600),
            provider_exclusion_fail_count: Some(3),
            work_branch_prefix: Some("zbobr_fix".into()),
            git_user_name: Some("zbobr".into()),
            git_user_email: Some("zbobr@example.com".into()),
            overwrite_author: Some(false),
            max_task_stage_count: None,
            timezone: None,
        }),
        tasks: Some(ZbobrTaskBackendGithubToml {
            instance: None,
            default_max_stage_count: Some(zbobr_api::task::DEFAULT_MAX_STAGE_COUNT),
            github_repo: Some("owner/repo".into()),
            github_token: Some(Secret::value(String::new())),
            reports_branch: None,
            reports_path: None,
            allowed_usernames: None,
        }),
        repo: Some(ZbobrRepoBackendGithubToml {
            repository: Some("owner/repo".into()),
            branch: Some("main".into()),
            github_token: Some(Secret::value(String::new())),
            repos_dir: Some(PathBuf::from("./repos")),
        }),
        executor: Some(ZbobrExecutorToml {
            claude: None,
            copilot: Some(ZbobrExecutorCopilotToml {
                copilot_github_token: None,
            }),
            mcp_tester: None,
        }),
        workflow: Some(WorkflowToml {
            prompts_dir: workflow.prompts_dir,
            roles: Some(workflow.roles),
            pipelines: Some(workflow.pipelines),
        }),
    }
}

/// Build the default workflow configuration with predefined pipelines and roles.
fn default_workflow() -> WorkflowConfig {
    use McpTool::{
        AddChecklistItem, CheckChecklistItem, GetCtxRec, ReportFailure, ReportIntermediate,
        ReportSuccess, StopWithError, StopWithQuestion,
    };

    let task_prompt = vec![PathBuf::from("task.md")];

    let main_stages = IndexMap::from([
        (
            Stage::from("planning"),
            StageDefinition {
                role: Some("planner".into()),
                prompts: task_prompt.clone(),
                on_intermediate: Some(StageTransition::pause()),
                ..Default::default()
            },
        ),
        (
            Stage::from("working"),
            StageDefinition {
                role: Some("worker".into()),
                prompts: task_prompt.clone(),
                on_intermediate: Some(StageTransition::stage("reviewing")),
                ..Default::default()
            },
        ),
        (
            Stage::from("reviewing"),
            StageDefinition {
                role: Some("reviewer".into()),
                prompts: task_prompt.clone(),
                on_failure: Some(StageTransition::stage("working")),
                on_intermediate: Some(StageTransition::stage("test_planner")),
                ..Default::default()
            },
        ),
        (
            Stage::from("test_planner"),
            StageDefinition {
                role: Some("test_planner".into()),
                prompts: task_prompt.clone(),
                on_failure: Some(StageTransition::stage("working")),
                on_intermediate: Some(StageTransition::stage("test_worker")),
                ..Default::default()
            },
        ),
        (
            Stage::from("test_worker"),
            StageDefinition {
                role: Some("test_worker".into()),
                prompts: task_prompt.clone(),
                on_failure: Some(StageTransition::stage("working")),
                on_intermediate: Some(StageTransition::stage("working")),
                ..Default::default()
            },
        ),
        (
            Stage::from("linting"),
            StageDefinition {
                role: Some("linter".into()),
                prompts: task_prompt.clone(),
                on_failure: Some(StageTransition::stage("working")),
                ..Default::default()
            },
        ),
        (
            Stage::from("testing"),
            StageDefinition {
                role: Some("tester".into()),
                prompts: task_prompt.clone(),
                on_failure: Some(StageTransition::stage("working")),
                ..Default::default()
            },
        ),
    ]);

    let merge_stages = IndexMap::from([(
        Stage::from("merging"),
        StageDefinition {
            role: Some("merger".into()),
            prompts: task_prompt,
            ..Default::default()
        },
    )]);

    let mut pipelines = HashMap::new();
    pipelines.insert(
        Pipeline::Main,
        PipelineConfig {
            stages: main_stages,
        },
    );
    pipelines.insert(
        Pipeline::Merge,
        PipelineConfig {
            stages: merge_stages,
        },
    );

    let roles = IndexMap::from([
        (
            "planner".into(),
            RoleDefinition {
                mcp: vec![
                    StopWithError,
                    StopWithQuestion,
                    ReportSuccess,
                    ReportIntermediate,
                    AddChecklistItem,
                    GetCtxRec,
                ],
                prompt: Some(PathBuf::from("planner.md")),
                tool: Some("planner".to_string()),
            },
        ),
        (
            "worker".into(),
            RoleDefinition {
                mcp: vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    ReportIntermediate,
                    StopWithQuestion,
                    AddChecklistItem,
                    CheckChecklistItem,
                    GetCtxRec,
                ],
                prompt: Some(PathBuf::from("worker.md")),
                tool: Some("developer".to_string()),
            },
        ),
        (
            "test_planner".into(),
            RoleDefinition {
                mcp: vec![
                    StopWithError,
                    StopWithQuestion,
                    ReportSuccess,
                    ReportIntermediate,
                    AddChecklistItem,
                    GetCtxRec,
                ],
                prompt: Some(PathBuf::from("test_planner.md")),
                tool: Some("planner".to_string()),
            },
        ),
        (
            "test_worker".into(),
            RoleDefinition {
                mcp: vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    ReportIntermediate,
                    StopWithQuestion,
                    AddChecklistItem,
                    CheckChecklistItem,
                    GetCtxRec,
                ],
                prompt: Some(PathBuf::from("test_worker.md")),
                tool: Some("developer".to_string()),
            },
        ),
        (
            "reviewer".into(),
            RoleDefinition {
                mcp: vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    ReportIntermediate,
                    StopWithQuestion,
                    CheckChecklistItem,
                    GetCtxRec,
                ],
                prompt: Some(PathBuf::from("reviewer.md")),
                tool: Some("developer".to_string()),
            },
        ),
        (
            "tester".into(),
            RoleDefinition {
                mcp: vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    StopWithQuestion,
                    GetCtxRec,
                ],
                prompt: Some(PathBuf::from("tester.md")),
                tool: Some("developer".to_string()),
            },
        ),
        (
            "linter".into(),
            RoleDefinition {
                mcp: vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    StopWithQuestion,
                    GetCtxRec,
                ],
                prompt: Some(PathBuf::from("linter.md")),
                tool: Some("drudge".to_string()),
            },
        ),
        (
            "merger".into(),
            RoleDefinition {
                mcp: vec![StopWithError, ReportSuccess, StopWithQuestion],
                prompt: Some(PathBuf::from("merger.md")),
                tool: Some("silly".to_string()),
            },
        ),
    ]);

    WorkflowConfig {
        prompts_dir: Some(PathBuf::from("prompts")),
        pipelines,
        roles,
    }
}

/// Convert `workflow.pipelines.*.stages.*` entries from standard tables to inline tables.
fn inline_stage_tables(doc: &mut toml_edit::DocumentMut) {
    let Some(toml_edit::Item::Table(workflow)) = doc.get_mut("workflow") else {
        return;
    };
    let Some(toml_edit::Item::Table(pipelines)) = workflow.get_mut("pipelines") else {
        return;
    };
    for (_pname, pipeline_item) in pipelines.iter_mut() {
        let Some(pipeline) = pipeline_item.as_table_mut() else {
            continue;
        };
        let Some(toml_edit::Item::Table(stages)) = pipeline.get_mut("stages") else {
            continue;
        };
        let keys: Vec<String> = stages.iter().map(|(k, _)| k.to_string()).collect();
        for key in &keys {
            let Some(stage_item) = stages.get_mut(key) else {
                continue;
            };
            if let Some(table) = stage_item.as_table_mut() {
                let inline = table.clone().into_inline_table();
                *stage_item = toml_edit::Item::Value(toml_edit::Value::InlineTable(inline));
            }
            if let Some(mut k) = stages.key_mut(key) {
                k.fmt();
            }
        }
        stages.set_dotted(true);
    }
}

/// Convert `dispatcher.providers.*` and `dispatcher.tools.*` entries to inline tables/arrays.
fn inline_dispatcher_tables(doc: &mut toml_edit::DocumentMut) {
    let Some(toml_edit::Item::Table(dispatcher)) = doc.get_mut("dispatcher") else {
        return;
    };

    if let Some(toml_edit::Item::Table(providers)) = dispatcher.get_mut("providers") {
        let keys: Vec<String> = providers.iter().map(|(k, _)| k.to_string()).collect();
        for key in &keys {
            let Some(provider_item) = providers.get_mut(key) else {
                continue;
            };
            if let Some(table) = provider_item.as_table_mut() {
                let inline = table.clone().into_inline_table();
                *provider_item = toml_edit::Item::Value(toml_edit::Value::InlineTable(inline));
            }
            if let Some(mut k) = providers.key_mut(key) {
                k.fmt();
            }
        }
    }

    if let Some(toml_edit::Item::Table(tools)) = dispatcher.get_mut("tools") {
        let keys: Vec<String> = tools.iter().map(|(k, _)| k.to_string()).collect();
        for key in &keys {
            let Some(tool_item) = tools.get_mut(key) else {
                continue;
            };
            let inline_tables: Option<Vec<toml_edit::InlineTable>> =
                if let toml_edit::Item::ArrayOfTables(aot) = tool_item {
                    Some(aot.iter().map(|t| t.clone().into_inline_table()).collect())
                } else {
                    None
                };
            if let Some(inline_tables) = inline_tables {
                let mut array = toml_edit::Array::new();
                for inline in inline_tables {
                    array.push(toml_edit::Value::InlineTable(inline));
                }
                *tool_item = toml_edit::Item::Value(toml_edit::Value::Array(array));
            }
            if let Some(mut k) = tools.key_mut(key) {
                k.fmt();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Default prompt files
// ---------------------------------------------------------------------------

const PROMPT_FILES: &[(&str, &str)] = &[
    ("planner", PLANNER_PROMPT),
    ("worker", WORKER_PROMPT),
    ("test_planner", TEST_PLANNER_PROMPT),
    ("test_worker", TEST_WORKER_PROMPT),
    ("reviewer", REVIEWER_PROMPT),
    ("linter", LINTER_PROMPT),
    ("tester", TESTER_PROMPT),
    ("merger", MERGER_PROMPT),
    ("task", TASK_TEMPLATE),
];

const TASK_TEMPLATE: &str = r#"---

# Current task: {title}

# Task description

{description}

# Destination branch: {destination_branch}

# Work branch: {work_branch}

# Context

{context}
"#;

macro_rules! get_ctx_rec_guidance {
    () => {
        "- When the context references a detailed record by `ctx_rec_*` ID, use `{mcp_get_ctx_rec}` to fetch the full content before you make decisions or continue your work.\n"
    };
}

const PLANNER_PROMPT: &str = concat!(
    r#"# Planner Agent

Read the task description and comments provided below in this prompt. Design an implementation plan for the task. See more detailed workflow instructions below.

Work autonomously, try to solve problems independently. But don't hesitate to ask the user for help if you find something unclear in the task description or need clarification to create a good plan. Use `{mcp_stop_with_question}` for this purpose.

## Access Model

- You can access the internet and run local commands.
- Use MCP `{mcp_report_intermediate}` to present the plan for user review when plan is not yet approved
- Use MCP `{mcp_add_checklist_item}` and `{mcp_report_success}` to send the the plan to implementation when the plan is approved
- Use MCP `{mcp_stop_with_question}` when you have doubts or something is unclear — send only focused question(s) with context, do NOT include the full plan in your response
- Use MCP `{mcp_stop_with_error}` only to report technical errors
"#,
    get_ctx_rec_guidance!(),
    r#"
- NEVER use git/gh for writing, pushing, or sending data to GitHub

## Workspace isolation

Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

## Workflow

1. Read the task description, context, and comments provided in the context section.
2. Inspect already made changes using `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in the work branch.

3. **Identify the closest analog in the codebase BEFORE designing the plan.** Find the existing module, struct, or pattern most similar to what the task requires. This is critical: the implementation must follow the same approaches, conventions, and style as the analog to keep the codebase consistent.
4. **Design an architecture-level plan**. Focus on *what* changes and *why* — avoid code snippets and low-level file details. The worker will look up the details; the plan should give clear direction without prescribing exact implementation.
5. If some instrument is required and you can't install it yourself, ask the user to install it with `{mcp_stop_with_question}`.
6. **Determine if the plan is clear and ready**:
   - If something is unclear or you have doubts, use `{mcp_stop_with_question}` to ask only focused question(s) with sufficient context to understand the question. Do NOT add checklist items yet. Finish the session after asking.
   - Only if the plan is clear and no questions were posted, proceed to step 7.
7. **Check for user approval**:
   - Review the most recent (last) comment below to determine if the user unambiguously approves this plan
   - Check the task description to see if it explicitly states that confirmation is not needed (e.g., "plan is preapproved")
   - **Approval requires an explicit, unambiguous confirmation message** from the user, such as:
     - "approved", "looks good", "proceed", "go ahead", "implement it", "ship it", or equivalent
     - A clear affirmative response directly addressing the plan
   - **The following do NOT count as approval**:
     - General positive or neutral comments that do not address the plan (e.g., "ok", "thanks", "interesting")
     - Questions or requests for clarification
     - Comments about the task description rather than the plan
     - Silence or absence of a comment
     - Any ambiguous message that could be interpreted as something other than plan approval
   - If approval is confirmed (in the last comment or task description):
     - Proceed to step 8: create checklist items
     - Then call `{mcp_report_success}` to finalize and proceed to implementation
   - If approval is NOT confirmed (including any doubt):
     - Proceed to step 8.5: present the plan for review
     - Call `{mcp_report_intermediate}` and wait for user feedback
     - Do NOT create checklist items yet (to avoid noise if plan is rejected)
     - **When in doubt, always present the plan for review rather than proceeding**
8. **Prepare checklist items for the worker** (only when plan is approved):
   - Review the unchecked checklist items in the context below (if any).
   - Use `{mcp_add_checklist_item}` to add implementation steps for the worker. Each item has two parts: a **brief** summary (shown inline in the context) and a **full_report** with detailed instructions (stored as a linked file). Put concise step title in brief; put the *what* and *why* in full_report — which components or modules to change, which interfaces or data flows are affected, which patterns from the analog to follow. Do NOT include code snippets, exact file paths, or prescriptive implementation details — the worker will look those up.
   - The checklist items ARE the plan — they should fully describe what the worker needs to do
   - After creating checklist items, call `{mcp_report_success}` with a brief rationale (why this approach was chosen, key design decisions, important constraints, chosen analog).
8.5. **If approval is NOT confirmed**: Present the plan by calling `{mcp_report_intermediate}` with a brief description of the proposed approach. Do NOT include checklist items yet — present only the plan structure and rationale."#,
);

const WORKER_PROMPT: &str = concat!(
    r#"# Worker Agent

Implement the task accordingly to the final plan in the context. Notice that there can be multiple plan versions in the history, work on the last one. If the plan is accompanied by checklist items, process them one by one, skip the checked ones. If there are no checklst items, analyze the pan and create checklist items for the implementation steps yourself.

- Use `{mcp_check_checklist_item}` to mark item as done when you complete the subtask in it.
- Use `{mcp_add_checklist_item}` to add new item when you discover new job to do or user made additional request in comments.
"#,
    get_ctx_rec_guidance!(),
    r#"

## Access Model

You can access the internet and run local commands. Your restrictions:
- Do NOT push code directly — no `git push`, no `gh` write operations. The platform coordinates repository remote actions; do not include submission or remote-write actions as checklist items.
- Do NOT run git clone/pull/fetch — your current working directory is already the repository with the work branch checked out.
- For reading GitHub data: use `git` and `gh` CLI only when no platform tool provides the needed information.
- NEVER use git/gh for writing, pushing, or sending data to GitHub.
- The work repository has remote information controlled by the platform; you must not perform direct remote writes yourself.

## Workspace isolation

Workspace branch isolation. Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

Work autonomously. Do not ask the user for anything unless the task genuinely requires human input.

## Workflow

1. Read the task description, context, and comments provided below in this prompt. The full history and checklist are available in the context section.
2. **Identify the analog referenced in the plan.** Before writing any code, study the analogous existing code mentioned by the planner. Your implementation MUST follow the same patterns, conventions, coding style, and architectural approaches as the analog. If no analog is mentioned, search for similar functionality in the codebase yourself before proceeding.
3. Implement the task by going through unchecked checklist items one by one. Commit work after implementing each item.  **Follow the same patterns and style as the identified analog if one is available.**
4. When implementation for an item is complete, mark the item done with `{mcp_check_checklist_item}` (pass the ctx_rec_N id).
5. If you sense your context window is getting close to its limit, finish your current item to a buildable state, commit your work, mark completed items as done, call `{mcp_report_intermediate}` with a summary of what you accomplished and what remains and finish the session.
6. If you need human clarification or intervention, call `{mcp_stop_with_question}`. If the plan is unclear or requires adjustment, call `{mcp_report_failure}`. In case of technical errors use `{mcp_stop_with_error}`.
7. If some instrument is required and you can't install it yourself, ask the user to install it with `{mcp_stop_with_question}`.
8. When your current session's work is done, decide how to finish:
    - If **all checklist items are completed** (the full plan is done), call `{mcp_report_success}` to report final success.
    - If **some items remain unchecked** (more work is needed in future sessions), call `{mcp_report_intermediate}` to report what you accomplished so far.

## Coding Guidelines

- **Prefer deriving values from types and constants** rather than using hardcoded string literals. If a value can be computed from an existing type, enum variant, or constant, do it. Avoid duplicating the value as literals or constants."#,
);

const TEST_PLANNER_PROMPT: &str = concat!(
    r#"#Analyze the implementation changes and determine if additional tests are required. Your job is to produce a test plan with list of tests to be added.

"#,
    get_ctx_rec_guidance!(),
    r#"

## Workflow

1. Read recent plan and recent implemetation report.
2. Inspect changes in the working branch (e.g., `git diff origin/{destination_branch}...HEAD`) to understand implemented behavior.
3. Decide whether the new feature/bugfix needs additional tests beyond existing coverage. If no new tests are needed, call `{mcp_report_success}` with only a brief rationale and finish.
4. Do NOT propose tests that only assert static prompt text or default config literal values.
5. Treat prompt files and default config examples as source-of-truth authoring artifacts, not behavior contracts to snapshot.
6. Prefer tests that validate behavior and contracts: transitions/routing, parser/serializer invariants, error handling, and externally observable outcomes.
7. Add content-based assertions only when exact text/value stability is itself an explicit product/API contract.
8. Prepare a plan for implementing the required tests as an overview document and set of checklist items
9. Call `{mcp_add_checklist_item}` for each test or group of related tests.
10. Call `{mcp_report_success}` with the overview report test-planning work is complete.
"#,
);

const TEST_WORKER_PROMPT: &str = concat!(
    r#"Implement the requested tests and run them.

"#,
    get_ctx_rec_guidance!(),
    r#"

## Workflow

1. For each unchecked checklist item related to tests, implement the corresponding test. Commit your work after implementing each item.
2. Run the implemented tests.
3. If tests fail, call `{mcp_report_failure}` and include failure details.
4. If tests pass, call `{mcp_report_success}`.

## Important
Do not implement any functionality, your job is only to implement and run tests according to the unchecked checklist items.
"#,
);

const REVIEWER_PROMPT: &str = concat!(
    r#"# Reviewer Agent

Review the implementation changes and ensure they meet coding standards and task requirements.

"#,
    get_ctx_rec_guidance!(),
    r#"

## Access Model

    You have read-only access to the task plan and access to the repository for inspection:
    - The task description, work plan, worker's reports, and context are provided below in this prompt. The full history and checklist are available in the context section.
    - Your current working directory is already the repository with the work branch checked out — examine changes directly
    - Use `{mcp_stop_with_error}` only to report technical errors
    - You can send multiple success or failure reports to provide detailed feedback on different aspects.

## Workflow

1. Read the task description, work plan, worker's reports, and context provided below in this prompt. Note if the analog solution in the existing code is referenced in the plan.
2. **Inspect all changes made in this task**: Use `git diff origin/<destination_branch>...HEAD` (three dots) to see ALL changes introduced by this task relative to the base branch. Do NOT checkout the base branch (it may conflict with worktree setup). You can also use `git log origin/<destination_branch>..HEAD` to see all commits in this branch.
3. **Verify the analog choice and pattern consistency**: Check that the planner chose an appropriate analog for the new functionality. Then verify that the implementation consistently follows the same patterns, conventions, coding style, and architectural approaches as the analog. Flag any deviations — new code should look like it was written by the same author as the existing analogous code. If the analog was poorly chosen, note this as a review finding.
4. **Review code quality and correctness**: Examine the implementation for correctness, code style, design patterns, and adherence to the plan. **Do not run any tests yourself; testing is handled separately.**
5. Verify that all changes are related to the task and are necessary for the implementation. Flag any extraneous changes that do not directly contribute to the task requirements or plan.
6. Additionally review each unchecked checklist item in the task context:
    - If you verify the item is correctly implemented or just became obsolete due to further changes, call `{mcp_check_checklist_item}` with the item’s ID
    - If the item's implementation is missing and it's still relevant, leave it unchecked and report this in the review findings.
7. Prepare a detailed review report describing any issues found, suggested fixes, and overall assessment. Include your assessment of analog consistency.
8. Finish the review by calling one of:
    - `{mcp_report_success}` — the implementation is correct and **all checklist items are completed**.
    - `{mcp_report_intermediate}` — the implementation of completed items looks correct, but **some checklist items remain unchecked**.
    - `{mcp_report_failure}` — issues were found in the implementation that must be fixed.
   Pass the review report as a parameter.

## Review Guidelines

- **Check compile-time validation**: Verify whether code correctness can be enforced at compile time (e.g., through type system, constants, enums) rather than relying on runtime checks or string matching. Flag opportunities to strengthen compile-time guarantees.
- **Check robustness against inconsistent changes**: Verify that the code is resilient to partial updates — e.g., changing a constant or literal in one place and forgetting to update it elsewhere. Flag hardcoded string literals that could be derived from existing types or constants. But don't be overzealous — not every literal needs to be served as a constant, especially in examples, demonstrations, or tests.
- **Check type specificity**: Verify that all newly introduced fields, variables, parameters, and return types use the most specific type available for their purpose. Suspect all base types (numbers, strings, booleans) — search the codebase for existing custom types, newtypes, or domain-specific wrappers that should be used instead.
- **Check test value**: Flag tests that only verify static prompt/config content as low-value and brittle unless exact text/value is an explicit runtime or API contract.
- **Prefer behavior-oriented tests**: Favor findings and suggestions toward tests that validate observable behavior, transitions, integration boundaries, and failure paths."#,
);

const TESTER_PROMPT: &str = concat!(
    r#"# Tester Agent

Run comprehensive tests to verify the implementation meets all testing requirements and CI/build standards.

"#,
    get_ctx_rec_guidance!(),
    r#"
## Access Model

You have access to the task context and the repository for testing:
- The task description, work plan, worker's reports, and context are provided below in this prompt. The full history and checklist are available in the context section.
- Your current working directory is the repository with the work branch checked out
- Use `{mcp_stop_with_error}` only to report technical errors
- You can send multiple success or failure reports to provide detailed feedback on different aspects.

## Workflow

1. Read the task description, work plan, worker's reports, and context provided below in this prompt.
2. **Independently discover testing infrastructure:**
   - Examine CI and build configuration files (`.github/workflows/`, `Makefile`, `Cargo.toml`, `tox.ini`, `CMakeLists.txt`, or equivalent)
   - Identify test frameworks and commands (cargo test, npm test, pytest, etc.)
   - Identify multiplatform or cross-compilation requirements
   - Document any other automated checks that code must pass (security scans, type checking)
3. **Run comprehensive test suite** matching the project's requirements:
   - Execute all test commands you identified from the CI configuration
   - Record test framework versions, commands executed, and full output
   - Measure code coverage if available
   - Verify all CI requirements are met
4. In case of test failures run the failed tests on the original branch to determine if the failure is due to new changes or existing issues in the codebase.
5. **Document all testing performed:**
   - Test frameworks and versions used
   - All commands executed with full output
   - Test results (passed/failed/skipped counts)
   - Any failures found
   - Code coverage metrics
6. Call `{mcp_report_success}` if all tests pass and all requirements are met, or `{mcp_report_failure}` if any tests fail or requirements are not met. Pass your comprehensive test report as a parameter.

## Important Notes

- **Linting and formatting checks are handled by a separate stage — do not run them here.**
- **Do not modify logic or formatting**: Any substantive code changes must go back to the worker.
- **Comprehensive testing**: Run all test commands discovered from the CI unless they require complex environment configuration. Mention skipped tests in the report.
- **Concise but exhaustive reporting**: Include to the report exact command line of each test executed. In case of error append the extract of test log with the error message.
- **Early termination if necessary**: If some test run shows massive failures indicating a fundamental issue with the implementation, you may stop further testing and make `{mcp_report_failure}` report immediately. Otherwise execute full test suite."#,
);

const LINTER_PROMPT: &str = concat!(
    r#"# Linter Agent

Check code formatting and linting, and fix any issues found.

"#,
    get_ctx_rec_guidance!(),
    r#"
## Access Model

You have access to the task context and the repository:
- The task description, work plan, worker's reports, and context are provided below in this prompt. The full history and checklist are available in the context section.
- Your current working directory is the repository with the work branch checked out
- Use `{mcp_stop_with_error}` only to report technical errors

## Workflow

1. Read the task description and context provided below in this prompt.
2. **Discover formatting and linting setup** by examining CI and build configuration files:
   - `.github/workflows/` — look for formatting/linting steps (e.g., `cargo fmt --check`, `cargo clippy`, `prettier`, `black`, `gofmt`, `eslint`)
   - `Makefile`, `Cargo.toml`, `package.json`, `pyproject.toml`, or equivalent — identify lint/fmt commands
   - Note exact commands and flags used in CI so you run the same checks
3. **Run all formatting and linting checks** identified from CI:
   - Record each command executed and its full output
4. **Fix auto-fixable issues only**:
   - Apply tool-based auto-fixes (e.g., `cargo fmt`, `gofmt -w`, `black .`, `prettier --write`)
   - Address linting warnings or errors that can be auto-fixed by these tools
   - Do not attempt manual fixes for issues that cannot be resolved automatically by formatter/linter
   - If any issue remains after auto-fix and cannot be auto-fixed, call `{mcp_report_failure}` with a detailed report of the remaining issues
   - Commit only auto-fix changes with a message like `chore: fix formatting and linting`
5. Re-run the checks after fixing to confirm everything passes.
6. Call `{mcp_report_success}` if all formatting and linting checks pass, or `{mcp_report_failure}` if issues remain that cannot be auto-fixed. Pass a brief report of what was checked and what was fixed.

## Important Notes

- **Only fix formatting and linting** — do not modify logic, tests, or functionality.
- **Do not run tests** — functional testing is handled by a separate stage.
- **Fix issues autonomously**: You are allowed and expected to fix formatting/linting issues directly and commit them."#,
);

const MERGER_PROMPT: &str = r#"# Merger Agent

Resolve merge conflicts when the work branch cannot be automatically synchronized and commit the merge result.

## When Merger Runs

The framework attempted to merge changes into the work branch and encountered conflicts. The conflicts may come from merging the upstream base branch or from merging concurrent remote changes. The repository is in a mid-merge state with conflict markers in the affected files. Your job is to resolve those conflicts and complete the merge commit.


## Access Model

You have read access to the task and repository:
- The task description, work plan, reports, and context are provided below in this prompt.
- Your current working directory is already the repository with the work branch checked out and the merge in progress (conflict markers present)
- Use `{mcp_stop_with_question}` to ask the user for clarification on conflict resolution
- Use `{mcp_stop_with_error}` to report when conflicts cannot be resolved

## Workspace isolation

    Workspace branch isolation. Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

## Workflow

1. Read the task description, work plan, reports, and context provided below in this prompt. The full history and checklist are available in the context section.
2. Your current working directory is the repository in a mid-merge conflict state. Examine the conflicts:
   - `git status` to see which files have conflicts
   - `git diff` to examine conflict markers and understand what changed in each branch
   - Review the code in both branches to understand the intent
3. **Attempt automatic resolution:**
   - For simple, non-overlapping changes (e.g., formatting, imports, unrelated edits), apply manual fixes that combine both changes
   - Edit each conflicted file to remove all conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) and produce a correct merged version
   - Use `git add <file>` for each resolved file, then `git commit -m "chore: merge conflicts resolved"` to complete the merge commit
   - Do NOT run `git merge` again — just resolve the markers and commit
4. **If automatic resolution is not possible:**
   - Use `{mcp_stop_with_question}` to describe the conflicts and ask which version should be preferred, or ask for guidance
   - Wait for user input before proceeding
5. **After successful resolution:**
   - Ensure all your changes are explicitly committed using `git commit` to the local work branch
6. Call `{mcp_report_success}` to provide a brief and concise report of your work and finish the session. This report is critical context for further agent calls, so it MUST be compact.

## Conflict Resolution Principles

- Combine non-overlapping changes from both branches (destination and work) when possible
- For conflicting edits to the same code, ask the user which version is preferred
- Preserve the intent of both branches' changes if both changes are valid
- Do NOT delete either branch's work without explicit user guidance"#;

#[cfg(test)]
mod tests {
    use super::*;

    // ── inline_dispatcher_tables unit tests ──────────────────────────────

    #[test]
    fn inline_dispatcher_tables_converts_providers_to_inline() {
        let toml_str = r#"
[dispatcher.providers.copilot]
executor = "copilot"

[dispatcher.providers.claude]
executor = "claude"
"#;
        let mut doc: toml_edit::DocumentMut = toml_str.parse().unwrap();
        inline_dispatcher_tables(&mut doc);
        let output = doc.to_string();
        // Should use inline table syntax, not section headers
        assert!(
            output.contains("copilot = {"),
            "copilot should be inline table, got: {output}"
        );
        assert!(
            output.contains("claude = {"),
            "claude should be inline table, got: {output}"
        );
        assert!(
            !output.contains("[dispatcher.providers.copilot]"),
            "section header should be gone, got: {output}"
        );
    }

    #[test]
    fn inline_dispatcher_tables_converts_tools_to_inline_array() {
        let toml_str = r#"
[[dispatcher.tools.developer]]
provider = "claude"
model = "claude-opus-4.6"

[[dispatcher.tools.developer]]
provider = "copilot"
model = "claude-opus-4.6"
"#;
        let mut doc: toml_edit::DocumentMut = toml_str.parse().unwrap();
        inline_dispatcher_tables(&mut doc);
        let output = doc.to_string();
        assert!(
            output.contains("developer = ["),
            "developer should be inline array, got: {output}"
        );
        assert!(
            !output.contains("[[dispatcher.tools.developer]]"),
            "array-of-tables header should be gone, got: {output}"
        );
    }

    #[test]
    fn inline_dispatcher_tables_noop_when_dispatcher_absent() {
        let toml_str = r#"
[workflow]
name = "test"
"#;
        let mut doc: toml_edit::DocumentMut = toml_str.parse().unwrap();
        // Should not panic
        inline_dispatcher_tables(&mut doc);
    }
}
