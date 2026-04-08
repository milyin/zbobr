use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use indexmap::IndexMap;
use zbobr_api::{
    Pipeline, Secret, Stage,
    config::{
        PipelineConfig, Provider, ProviderDefinition, Role, RoleDefinition, StageDefinition,
        StageTransition, Tool, ToolEntry, WorkflowConfig, WorkflowToml,
    },
    config_tools::McpTool,
};
use zbobr_utility::TomlOption;
use zbobr_api::task::{Executor, Model};
use zbobr_dispatcher::config::{ZbobrDispatcherToml, ZbobrExecutorToml};
use zbobr_executor_copilot::ZbobrExecutorCopilotToml;
use zbobr_repo_backend_github::ZbobrRepoBackendGithubToml;
use zbobr_task_backend_github::ZbobrTaskBackendGithubToml;

use super::RootConfigToml;

// Default model names used by init workspace. Copilot paths use dot notation
// for some claude models, while actual Claude executor uses hyphen notation.
const COPILOT_MODEL_HAIKU: Model = Model::new("claude-haiku-4.5");
const CLAUDE_MODEL_HAIKU: Model = Model::new("claude-haiku-4-5");
const COPILOT_MODEL_SONNET: Model = Model::new("claude-sonnet-4.6");
const CLAUDE_MODEL_OPUS: Model = Model::new("claude-opus-4-6");
const CLAUDE_MODEL_SONNET: Model = Model::new("claude-sonnet-4-6");
const COPILOT_MODEL_GPT_5_4: Model = Model::new("gpt-5.4");
const COPILOT_MODEL_GPT_5_MINI: Model = Model::new("gpt-5-mini");

const WORKFLOW_PROMPTS_DIR: &str = "prompts";
const TASK_PROMPT: &str = "task.md";
const LOOP_SCRIPT: &str = "loop.sh";

const LOOP_SCRIPT_CONTENT: &str = r#"#!/usr/bin/env sh
set -eu

ZBOBR_CMD="${ZBOBR_CMD:-zbobr}"
ZBOBR_LOOP_CMD="${ZBOBR_LOOP_CMD:-true}"
ZBOBR_LOOP_INTERVAL="${ZBOBR_LOOP_INTERVAL:-60}"
ZBOBR_CLEANUP_INTERVAL="${ZBOBR_CLEANUP_INTERVAL:-600}"

last_cleanup_ts="$(date +%s)"

while sh -c "$ZBOBR_LOOP_CMD"; do
    eval "$ZBOBR_CMD task advance"

    if eval "$ZBOBR_CMD task process --select"; then
        :
    else
        rc="$?"
        if [ "$rc" -ne 1 ]; then
            echo "task process --select failed with exit code $rc" >&2
            exit "$rc"
        fi
    fi

    now_ts="$(date +%s)"
    if [ $((now_ts - last_cleanup_ts)) -ge "$ZBOBR_CLEANUP_INTERVAL" ]; then
        if ! eval "$ZBOBR_CMD cleanup"; then
            echo "warning: cleanup failed" >&2
        fi
        last_cleanup_ts="$now_ts"
    fi

    sleep "$ZBOBR_LOOP_INTERVAL"
done
"#;

const TOOL_DEVELOPER: Tool = Tool::new("developer");
const TOOL_PLANNER: Tool = Tool::new("planner");
const TOOL_HELPER: Tool = Tool::new("helper");
const TOOL_REVIEWER: Tool = Tool::new("reviewer");
const TOOL_DRUDGE: Tool = Tool::new("drudge");

const ROLE_PLANNER: Role = Role::new("planner");
const ROLE_WORKER: Role = Role::new("worker");
const ROLE_TEST_PLANNER: Role = Role::new("test_planner");
const ROLE_TEST_WORKER: Role = Role::new("test_worker");
const ROLE_REVIEWER: Role = Role::new("reviewer");
const ROLE_TESTER: Role = Role::new("tester");
const ROLE_LINTER: Role = Role::new("linter");
const ROLE_LINTER_WORKER: Role = Role::new("linter_worker");
const ROLE_MERGER: Role = Role::new("merger");

const STAGE_PLANNING: Stage = Stage::new("planning");
const STAGE_WORKING: Stage = Stage::new("working");
const STAGE_REVIEWING: Stage = Stage::new("reviewing");
const STAGE_TEST_PLANNER: Stage = Stage::new("test_planner");
const STAGE_TEST_WORKER: Stage = Stage::new("test_worker");
const STAGE_LINTING: Stage = Stage::new("linting");
const STAGE_LINTER_WORKER: Stage = Stage::new("linter_worker");
const STAGE_TESTING: Stage = Stage::new("testing");
const STAGE_MERGING: Stage = Stage::new("merging");

const PROVIDER_CLAUDE: Provider = Provider::new("claude");
const PROVIDER_COPILOT: Provider = Provider::new("copilot");
const PROVIDER_CLAUDE_PLANNER: Provider = Provider::new("claude_planner");
const PROVIDER_COPILOT_PLANNER: Provider = Provider::new("copilot_planner");

/// Initialize a new zbobr workspace at the given directory.
///
/// Creates the directory (if it does not exist), writes a complete `zbobr.toml`
/// config file, creates prompt files for each predefined role, and creates
/// the required subdirectories.
///
/// If a file already exists with different content, the behavior depends on the
/// `force` flag: when `force` is `true` the existing file is overwritten in
/// place; otherwise the new version is written next to it as `{filename}.new`.
pub async fn init_workspace(dest: &Path, force: bool) -> anyhow::Result<()> {
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
        write_or_new(&path, content, force).await?;
    }

    // Serialize with toml pretty-printer, then post-process with toml_edit
    // to convert stage definitions and dispatcher providers/tools into inline tables.
    let config = default_config_toml();
    let pretty = toml::to_string_pretty(&config)?;
    let mut doc: toml_edit::DocumentMut = pretty.parse()?;
    inline_stage_tables(&mut doc);
    inline_role_prompt_tables(&mut doc);
    inline_dispatcher_tables(&mut doc);
    let config_content = format!(
        "# zbobr configuration\n# See documentation for all available options.\n\n{}",
        doc
    );
    let config_path = dest.join("zbobr.toml");
    write_or_new(&config_path, &config_content, force).await?;

    let loop_script_path = dest.join(LOOP_SCRIPT);
    write_or_new(&loop_script_path, LOOP_SCRIPT_CONTENT, force).await?;

    #[cfg(unix)]
    {
        let mut perms = tokio::fs::metadata(&loop_script_path).await?.permissions();
        perms.set_mode(perms.mode() | 0o755);
        tokio::fs::set_permissions(&loop_script_path, perms).await?;
    }

    println!(
        "\nWorkspace initialized at {}.\nEdit zbobr.toml to configure backends and tokens before running.",
        dest.display()
    );
    Ok(())
}

/// Write `content` to `path`. If the file already exists with identical content,
/// skip it. If it exists with different content, write to `{path}.new` instead
/// — unless `force` is true, in which case overwrite in place.
async fn write_or_new(path: &Path, content: &str, force: bool) -> anyhow::Result<()> {
    if path.exists() {
        let existing = tokio::fs::read_to_string(path).await?;
        if existing == content {
            println!("  unchanged {}", path.display());
            return Ok(());
        }
        if force {
            tokio::fs::write(path, content).await?;
            println!("  overwrote {}", path.display());
        } else {
            let new_path = path.with_extension(format!(
                "{}.new",
                path.extension().unwrap_or_default().to_string_lossy()
            ));
            tokio::fs::write(&new_path, content).await?;
            println!("  wrote {} (existing file differs)", new_path.display());
        }
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
            PROVIDER_CLAUDE,
            ProviderDefinition {
                executor: TomlOption::Value(Executor::claude()),
                parent: TomlOption::Absent,
                priority: TomlOption::Absent,
                plan_mode: TomlOption::Absent,
                access_key: TomlOption::Absent,
            },
        ),
        (
            PROVIDER_COPILOT,
            ProviderDefinition {
                executor: TomlOption::Value(Executor::copilot()),
                parent: TomlOption::Absent,
                priority: TomlOption::Absent,
                plan_mode: TomlOption::Absent,
                access_key: TomlOption::Absent,
            },
        ),
        (
            PROVIDER_CLAUDE_PLANNER,
            ProviderDefinition {
                executor: TomlOption::Absent,
                parent: TomlOption::Value(PROVIDER_CLAUDE),
                priority: TomlOption::Absent,
                plan_mode: TomlOption::Value(true),
                access_key: TomlOption::Absent,
            },
        ),
        (
            PROVIDER_COPILOT_PLANNER,
            ProviderDefinition {
                executor: TomlOption::Absent,
                parent: TomlOption::Value(PROVIDER_COPILOT),
                priority: TomlOption::Absent,
                plan_mode: TomlOption::Value(true),
                access_key: TomlOption::Absent,
            },
        ),
    ]);

    let tools = IndexMap::from([
        (
            TOOL_DEVELOPER,
            vec![
                ToolEntry {
                    provider: PROVIDER_CLAUDE,
                    model: CLAUDE_MODEL_SONNET,
                    priority: None,
                },
                ToolEntry {
                    provider: PROVIDER_COPILOT,
                    model: COPILOT_MODEL_SONNET,
                    priority: Some(0),
                },
            ],
        ),
        (
            TOOL_PLANNER,
            vec![
                ToolEntry {
                    provider: PROVIDER_CLAUDE_PLANNER,
                    model: CLAUDE_MODEL_SONNET,
                    priority: None,
                },
                ToolEntry {
                    provider: PROVIDER_COPILOT_PLANNER,
                    model: COPILOT_MODEL_SONNET,
                    priority: Some(0),
                },
            ],
        ),
        (
            TOOL_HELPER,
            vec![
                ToolEntry {
                    provider: PROVIDER_COPILOT,
                    model: COPILOT_MODEL_HAIKU,
                    priority: None,
                },
                ToolEntry {
                    provider: PROVIDER_CLAUDE,
                    model: CLAUDE_MODEL_HAIKU,
                    priority: Some(0),
                },
            ],
        ),
        (
            TOOL_REVIEWER,
            vec![
                ToolEntry {
                    provider: PROVIDER_COPILOT,
                    model: COPILOT_MODEL_GPT_5_4,
                    priority: None,
                },
                ToolEntry {
                    provider: PROVIDER_CLAUDE,
                    model: CLAUDE_MODEL_SONNET,
                    priority: Some(0),
                },
            ],
        ),
        (
            TOOL_DRUDGE,
            vec![
                ToolEntry {
                    provider: PROVIDER_COPILOT,
                    model: COPILOT_MODEL_GPT_5_MINI,
                    priority: None,
                },
                ToolEntry {
                    provider: PROVIDER_CLAUDE,
                    model: CLAUDE_MODEL_HAIKU,
                    priority: Some(0),
                },
            ],
        ),
    ]);

    RootConfigToml {
        dispatcher: Some(ZbobrDispatcherToml {
            instance: TomlOption::Value("default".into()),
            workspaces: TomlOption::Value(PathBuf::from("./workspaces")),
            base_port: TomlOption::Value(3000),
            agent_github_token: TomlOption::Value(Secret::value("not-configured")),
            providers: Some(providers),
            tools: Some(tools),
            provider_exclusion_secs: TomlOption::Value(3600),
            provider_exclusion_fail_count: TomlOption::Value(3),
            work_branch_prefix: TomlOption::Value("zbobr_fix".into()),
            git_user_name: TomlOption::Value("zbobr".into()),
            git_user_email: TomlOption::Value("zbobr@example.com".into()),
            overwrite_author: TomlOption::Value(false),
            max_task_stage_count: TomlOption::Absent,
            timezone: TomlOption::Absent,
        }),
        tasks: Some(ZbobrTaskBackendGithubToml {
            instance: TomlOption::Absent,
            timezone: TomlOption::Absent,
            default_max_stage_count: TomlOption::Value(zbobr_api::task::DEFAULT_MAX_STAGE_COUNT),
            github_repo: TomlOption::Value("owner/repo".into()),
            github_token: TomlOption::Value(Secret::value(String::new())),
            reports_branch: TomlOption::Absent,
            reports_path: TomlOption::Absent,
            allowed_usernames: TomlOption::Absent,
        }),
        repo: Some(ZbobrRepoBackendGithubToml {
            repository: TomlOption::Value("owner/repo".into()),
            branch: TomlOption::Value("main".into()),
            github_token: TomlOption::Value(Secret::value(String::new())),
            repos_dir: TomlOption::Value(PathBuf::from("./repos")),
        }),
        executor: Some(ZbobrExecutorToml {
            claude: None,
            copilot: Some(ZbobrExecutorCopilotToml {
                copilot_github_token: TomlOption::Absent,
            }),
            mcp_tester: None,
        }),
        workflow: Some(WorkflowToml {
            prompts_dir: workflow.prompts_dir.into(),
            prompts: workflow.prompts,
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

    let main_stages = IndexMap::from([
        (
            STAGE_PLANNING,
            StageDefinition {
                role: TomlOption::Value(ROLE_PLANNER),
                on_intermediate: TomlOption::Value(StageTransition::pause()),
                ..Default::default()
            },
        ),
        (
            STAGE_WORKING,
            StageDefinition {
                role: TomlOption::Value(ROLE_WORKER),
                on_failure: TomlOption::Value(StageTransition {
                    next: Some(STAGE_WORKING),
                    pause: true,
                }),
                on_intermediate: TomlOption::Value(StageTransition::stage(STAGE_REVIEWING)),
                ..Default::default()
            },
        ),
        (
            STAGE_REVIEWING,
            StageDefinition {
                role: TomlOption::Value(ROLE_REVIEWER),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
                on_intermediate: TomlOption::Value(StageTransition::stage(STAGE_TEST_PLANNER)),
                ..Default::default()
            },
        ),
        (
            STAGE_TEST_PLANNER,
            StageDefinition {
                role: TomlOption::Value(ROLE_TEST_PLANNER),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
                on_intermediate: TomlOption::Value(StageTransition::stage(STAGE_TEST_WORKER)),
                ..Default::default()
            },
        ),
        (
            STAGE_TEST_WORKER,
            StageDefinition {
                role: TomlOption::Value(ROLE_TEST_WORKER),
                on_failure: TomlOption::Value(StageTransition {
                    next: Some(STAGE_TEST_WORKER),
                    pause: true,
                }),
                on_intermediate: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
                ..Default::default()
            },
        ),
        (
            STAGE_LINTING,
            StageDefinition {
                role: TomlOption::Value(ROLE_LINTER),
                on_success: TomlOption::Value(StageTransition::stage(STAGE_TESTING)),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_LINTER_WORKER)),
                ..Default::default()
            },
        ),
        (
            STAGE_LINTER_WORKER,
            StageDefinition {
                role: TomlOption::Value(ROLE_LINTER_WORKER),
                on_success: TomlOption::Value(StageTransition::stage(STAGE_LINTING)),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
                ..Default::default()
            },
        ),
        (
            STAGE_TESTING,
            StageDefinition {
                role: TomlOption::Value(ROLE_TESTER),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
                ..Default::default()
            },
        ),
    ]);

    let merge_stages = IndexMap::from([(
        STAGE_MERGING,
        StageDefinition {
            role: TomlOption::Value(ROLE_MERGER),
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

    fn role_prompts(main: &str) -> Option<indexmap::IndexMap<String, TomlOption<PathBuf>>> {
        Some(indexmap::IndexMap::from([(
            "main".to_string(),
            TomlOption::Value(PathBuf::from(main)),
        )]))
    }

    let roles = IndexMap::from([
        (
            ROLE_PLANNER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    StopWithQuestion,
                    ReportSuccess,
                    ReportIntermediate,
                    AddChecklistItem,
                    GetCtxRec,
                ]),
                prompts: role_prompts("planner.md"),
                tool: TomlOption::Value(TOOL_PLANNER),
            },
        ),
        (
            ROLE_WORKER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    ReportIntermediate,
                    StopWithQuestion,
                    AddChecklistItem,
                    CheckChecklistItem,
                    GetCtxRec,
                ]),
                prompts: role_prompts("worker.md"),
                tool: TomlOption::Value(TOOL_DEVELOPER),
            },
        ),
        (
            ROLE_TEST_PLANNER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    StopWithQuestion,
                    ReportSuccess,
                    ReportIntermediate,
                    AddChecklistItem,
                    GetCtxRec,
                ]),
                prompts: role_prompts("test_planner.md"),
                tool: TomlOption::Value(TOOL_PLANNER),
            },
        ),
        (
            ROLE_TEST_WORKER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    ReportIntermediate,
                    StopWithQuestion,
                    AddChecklistItem,
                    CheckChecklistItem,
                    GetCtxRec,
                ]),
                prompts: role_prompts("test_worker.md"),
                tool: TomlOption::Value(TOOL_HELPER),
            },
        ),
        (
            ROLE_REVIEWER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    ReportIntermediate,
                    StopWithQuestion,
                    CheckChecklistItem,
                    GetCtxRec,
                ]),
                prompts: role_prompts("reviewer.md"),
                tool: TomlOption::Value(TOOL_REVIEWER),
            },
        ),
        (
            ROLE_TESTER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    StopWithQuestion,
                    GetCtxRec,
                ]),
                prompts: role_prompts("tester.md"),
                tool: TomlOption::Value(TOOL_HELPER),
            },
        ),
        (
            ROLE_LINTER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    StopWithQuestion,
                    GetCtxRec,
                ]),
                prompts: role_prompts("linter.md"),
                tool: TomlOption::Value(TOOL_DRUDGE),
            },
        ),
        (
            ROLE_LINTER_WORKER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    ReportSuccess,
                    ReportFailure,
                    StopWithQuestion,
                    GetCtxRec,
                ]),
                prompts: role_prompts("linter_worker.md"),
                tool: TomlOption::Value(TOOL_HELPER),
            },
        ),
        (
            ROLE_MERGER,
            RoleDefinition {
                mcp: Some(vec![StopWithError, ReportSuccess, StopWithQuestion]),
                prompts: role_prompts("merger.md"),
                tool: TomlOption::Value(TOOL_HELPER),
            },
        ),
    ]);

    let workflow_prompts = Some(indexmap::IndexMap::from([
        ("main".to_string(), TomlOption::ExplicitNone),
        (
            "task".to_string(),
            TomlOption::Value(PathBuf::from(TASK_PROMPT)),
        ),
    ]));

    WorkflowConfig {
        prompts_dir: Some(PathBuf::from(WORKFLOW_PROMPTS_DIR)),
        prompts: workflow_prompts,
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
        inline_named_children_as_inline_tables(stages);
        stages.set_dotted(true);
    }
}

fn inline_named_children_as_inline_tables(table: &mut toml_edit::Table) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for key in &keys {
        if let Some(item) = table.get_mut(key) {
            inline_item_as_inline_table(item);
        }
        if let Some(mut k) = table.key_mut(key) {
            k.fmt();
        }
    }
}

fn inline_item_as_inline_table(item: &mut toml_edit::Item) {
    if let Some(table) = item.as_table_mut() {
        let inline = table.clone().into_inline_table();
        *item = toml_edit::Item::Value(toml_edit::Value::InlineTable(inline));
    }
}

fn inline_child_table(parent: &mut toml_edit::Table, child_key: &str) {
    if let Some(item) = parent.get_mut(child_key) {
        inline_item_as_inline_table(item);
    }
}

/// Convert `workflow.roles.*.prompts` entries from standard tables to inline tables.
fn inline_role_prompt_tables(doc: &mut toml_edit::DocumentMut) {
    let Some(toml_edit::Item::Table(workflow)) = doc.get_mut("workflow") else {
        return;
    };
    let Some(toml_edit::Item::Table(roles)) = workflow.get_mut("roles") else {
        return;
    };

    let keys: Vec<String> = roles.iter().map(|(k, _)| k.to_string()).collect();
    for key in &keys {
        if let Some(role_item) = roles.get_mut(key) {
            if let Some(role_table) = role_item.as_table_mut() {
                inline_child_table(role_table, "prompts");
            }
        }
        if let Some(mut k) = roles.key_mut(key) {
            k.fmt();
        }
    }
}

/// Convert `dispatcher.providers.*` and `dispatcher.tools.*` entries to inline tables/arrays.
fn inline_dispatcher_tables(doc: &mut toml_edit::DocumentMut) {
    let Some(toml_edit::Item::Table(dispatcher)) = doc.get_mut("dispatcher") else {
        return;
    };

    if let Some(toml_edit::Item::Table(providers)) = dispatcher.get_mut("providers") {
        inline_named_children_as_inline_tables(providers);
    }

    if let Some(toml_edit::Item::Table(tools)) = dispatcher.get_mut("tools") {
        inline_named_children_as_inline_table_arrays(tools);
    }
}

fn inline_named_children_as_inline_table_arrays(table: &mut toml_edit::Table) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for key in &keys {
        if let Some(item) = table.get_mut(key) {
            inline_item_as_inline_table_array(item);
        }
        if let Some(mut k) = table.key_mut(key) {
            k.fmt();
        }
    }
}

fn inline_item_as_inline_table_array(item: &mut toml_edit::Item) {
    if let toml_edit::Item::ArrayOfTables(aot) = item {
        let mut array = toml_edit::Array::new();
        for table in aot.iter() {
            array.push(toml_edit::Value::InlineTable(table.clone().into_inline_table()));
        }
        *item = toml_edit::Item::Value(toml_edit::Value::Array(array));
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
    ("linter_worker", LINTER_WORKER_PROMPT),
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
5. Verify that all changes are related to the task and are necessary for the implementation. But accept the unrelated changes if they are formatting and linting changes or if they were introduced by the user according to the git history.
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

Check code formatting and linting and report any issues found.

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
4. Call `{mcp_report_success}` if all checks pass, or `{mcp_report_failure}` with a detailed list of ALL issues found if any checks fail.

## Important Notes

- **Only check formatting and linting** — do not modify logic, tests, or functionality.
- **Do not fix anything** — fixing is handled by a separate stage.
- **Do not run tests** — functional testing is handled by a separate stage."#,
);

const LINTER_WORKER_PROMPT: &str = concat!(
    r#"# Linter Worker Agent

Fix formatting and linting issues in the code.

"#,
    get_ctx_rec_guidance!(),
    r#"
## Access Model

You have access to the task context and the repository:
- The task description, work plan, worker's reports, and context are provided below in this prompt. The full history and checklist are available in the context section.
- Your current working directory is the repository with the work branch checked out
- Use `{mcp_stop_with_error}` only to report technical errors

## Workflow

1. Read the task context and failure reports to identify which formatting and linting issues need to be fixed.
2. **Discover formatting and linting setup** by examining CI and build configuration files:
   - `.github/workflows/` — look for formatting/linting steps (e.g., `cargo fmt --check`, `cargo clippy`, `prettier`, `black`, `gofmt`, `eslint`)
   - `Makefile`, `Cargo.toml`, `package.json`, `pyproject.toml`, or equivalent — identify lint/fmt commands
3. **Run the linting/formatting tools** to confirm which issues remain.
4. **Apply fixes**:
   - Apply tool-based auto-fixes (e.g., `cargo fmt`, `gofmt -w`, `black .`, `prettier --write`)
   - Apply manual fixes for linting warnings/errors that require code changes
5. Call `{mcp_report_success}` if all issues were fixed.
6. Call `{mcp_report_failure}` with details if some issues cannot be fixed.

## Important Notes

- **Only fix formatting and linting** — do not modify logic, tests, or functionality.
- **Do not run tests** — functional testing is handled separately."#,
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
model = "claude-sonnet-4.6"

[[dispatcher.tools.developer]]
provider = "copilot"
model = "claude-sonnet-4.6"
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

    #[test]
    fn inline_role_prompt_tables_converts_prompts_to_inline() {
        let toml_str = r#"
[workflow.roles.worker]
tool = "developer"

[workflow.roles.worker.prompts]
main = "worker.md"
"#;
        let mut doc: toml_edit::DocumentMut = toml_str.parse().unwrap();
        inline_role_prompt_tables(&mut doc);
        let output = doc.to_string();
        assert!(
            output.contains("prompts= { main = \"worker.md\" }")
                || output.contains("prompts = { main = \"worker.md\" }"),
            "role prompts should be inline table, got: {output}"
        );
        assert!(
            !output.contains("[workflow.roles.worker.prompts]"),
            "nested prompts table header should be removed, got: {output}"
        );
    }

    // ── default_workflow validation tests ──────────────────────────────

    #[test]
    fn default_workflow_is_valid() {
        let workflow = default_workflow();
        assert!(
            workflow.validate().is_ok(),
            "default workflow must pass validation"
        );
    }

    // ── linting and linter_worker stage transition routing tests ──────

    #[test]
    fn linting_on_success_routes_to_testing() {
        let wf = default_workflow();
        let main = wf.pipelines.get(&Pipeline::Main).unwrap();
        let linting = main.stages.get(&Stage::from("linting")).unwrap();
        let target = linting.on_success().and_then(|t| t.next.as_deref());
        assert_eq!(target, Some("testing"));
    }

    #[test]
    fn linting_on_failure_routes_to_linter_worker() {
        let wf = default_workflow();
        let main = wf.pipelines.get(&Pipeline::Main).unwrap();
        let linting = main.stages.get(&Stage::from("linting")).unwrap();
        let target = linting.on_failure().and_then(|t| t.next.as_deref());
        assert_eq!(target, Some("linter_worker"));
    }

    #[test]
    fn linter_worker_on_success_routes_to_linting() {
        let wf = default_workflow();
        let main = wf.pipelines.get(&Pipeline::Main).unwrap();
        let lw = main.stages.get(&Stage::from("linter_worker")).unwrap();
        let target = lw.on_success().and_then(|t| t.next.as_deref());
        assert_eq!(target, Some("linting"));
    }

    #[test]
    fn linter_worker_on_failure_routes_to_working() {
        let wf = default_workflow();
        let main = wf.pipelines.get(&Pipeline::Main).unwrap();
        let lw = main.stages.get(&Stage::from("linter_worker")).unwrap();
        let target = lw.on_failure().and_then(|t| t.next.as_deref());
        assert_eq!(target, Some("working"));
    }

    // ── PROMPT_FILES completeness tests ────────────────────────────────

    #[test]
    fn all_default_workflow_role_prompts_are_registered() {
        let wf = default_workflow();
        let registered: std::collections::HashSet<&str> =
            PROMPT_FILES.iter().map(|(name, _)| *name).collect();
        for (role_name, role_def) in &wf.roles {
            for (slot, prompt_opt) in role_def.prompts.iter().flatten() {
                if let Some(prompt_path) = prompt_opt.as_option() {
                    let key = prompt_path
                        .file_stem()
                        .and_then(|s: &std::ffi::OsStr| s.to_str())
                        .expect("prompt path has no file stem");
                    assert!(
                        registered.contains(key),
                        "Role '{}' slot '{}' references prompt file '{}' but it is not in PROMPT_FILES",
                        role_name,
                        slot,
                        key
                    );
                }
            }
        }
    }

    // ── write_or_new function tests ────────────────────────────────────

    #[tokio::test]
    async fn write_or_new_force_overwrites_existing_file() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test.toml");

        // Write initial content
        tokio::fs::write(&file_path, "old content".as_bytes())
            .await
            .expect("Failed to write initial file");

        // Call write_or_new with force=true and different content
        write_or_new(&file_path, "new content", true)
            .await
            .expect("write_or_new failed");

        // Check that original file was overwritten
        let result = tokio::fs::read_to_string(&file_path)
            .await
            .expect("Failed to read file");
        assert_eq!(result, "new content", "File should be overwritten in place");

        // Check that no .new file was created
        let new_file_path = file_path.with_extension("toml.new");
        assert!(
            !new_file_path.exists(),
            "No .new file should exist when force=true"
        );
    }

    #[tokio::test]
    async fn write_or_new_no_force_creates_dot_new_file() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("example.toml");

        // Write initial content
        tokio::fs::write(&file_path, "old content".as_bytes())
            .await
            .expect("Failed to write initial file");

        // Call write_or_new with force=false and different content
        write_or_new(&file_path, "new content", false)
            .await
            .expect("write_or_new failed");

        // Check that original file is untouched
        let original_content = tokio::fs::read_to_string(&file_path)
            .await
            .expect("Failed to read original file");
        assert_eq!(
            original_content, "old content",
            "Original file should not be modified"
        );

        // Check that .new file was created with new content
        let new_file_path = file_path.with_extension("toml.new");
        assert!(new_file_path.exists(), ".new file should be created");
        let new_content = tokio::fs::read_to_string(&new_file_path)
            .await
            .expect("Failed to read .new file");
        assert_eq!(
            new_content, "new content",
            ".new file should contain new content"
        );
    }

    #[tokio::test]
    async fn write_or_new_skips_identical_content() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("unchanged.toml");

        // Write initial content
        tokio::fs::write(&file_path, "same content".as_bytes())
            .await
            .expect("Failed to write initial file");

        // Call write_or_new with identical content and force=true
        write_or_new(&file_path, "same content", true)
            .await
            .expect("write_or_new failed");

        // Check that file still contains the original content
        let result = tokio::fs::read_to_string(&file_path)
            .await
            .expect("Failed to read file");
        assert_eq!(result, "same content", "File should remain unchanged");

        // Check that no .new file was created
        let new_file_path = file_path.with_extension("toml.new");
        assert!(
            !new_file_path.exists(),
            "No .new file should exist when content is identical"
        );
    }

    #[tokio::test]
    async fn write_or_new_creates_new_file() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("newfile.toml");

        // Path doesn't exist yet
        assert!(!file_path.exists(), "File should not exist initially");

        // Call write_or_new on non-existing path
        write_or_new(&file_path, "new content", false)
            .await
            .expect("write_or_new failed");

        // Check that file was created with correct content
        assert!(file_path.exists(), "File should be created");
        let result = tokio::fs::read_to_string(&file_path)
            .await
            .expect("Failed to read file");
        assert_eq!(result, "new content", "File should contain the new content");
    }

    #[tokio::test]
    async fn init_workspace_creates_loop_script_with_default_cmd() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

        init_workspace(temp_dir.path(), false)
            .await
            .expect("init_workspace failed");

        let loop_script_path = temp_dir.path().join("loop.sh");
        assert!(loop_script_path.exists(), "loop.sh should be created");

        let loop_script = tokio::fs::read_to_string(&loop_script_path)
            .await
            .expect("Failed to read loop.sh");
        assert!(
            loop_script.contains("ZBOBR_CMD=\"${ZBOBR_CMD:-zbobr}\""),
            "loop.sh should default ZBOBR_CMD to zbobr"
        );
        assert!(
            loop_script.contains("ZBOBR_LOOP_CMD=\"${ZBOBR_LOOP_CMD:-true}\""),
            "loop.sh should default ZBOBR_LOOP_CMD to true"
        );
        assert!(
            loop_script.contains("sh -c \"$ZBOBR_LOOP_CMD\""),
            "loop.sh should check ZBOBR_LOOP_CMD before each iteration"
        );
        assert!(
            loop_script.contains("eval \"$ZBOBR_CMD task advance\""),
            "loop.sh should run task advance"
        );
        assert!(
            loop_script.contains("eval \"$ZBOBR_CMD task process --select\""),
            "loop.sh should run task process --select"
        );
        assert!(
            loop_script.contains("eval \"$ZBOBR_CMD cleanup\""),
            "loop.sh should run cleanup"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = tokio::fs::metadata(&loop_script_path)
                .await
                .expect("Failed to stat loop.sh")
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111, "loop.sh should be executable");
        }
    }
}
