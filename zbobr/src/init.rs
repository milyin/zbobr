#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use toml_edit::{DocumentMut, Item};
use zbobr_api::{
    Pipeline, Secret, Stage,
    config::{
        PipelineConfig, Provider, ProviderDefinition, Role, RoleDefinition, StageDefinition,
        StageTransition, Tool, ToolEntry, WorkflowConfig, WorkflowToml,
    },
    config_tools::McpTool,
    task::{Executor, Model},
};
use zbobr_dispatcher::config::{ZbobrDispatcherToml, ZbobrExecutorToml};
use zbobr_executor_copilot::ZbobrExecutorCopilotToml;
use zbobr_repo_backend_github::ZbobrRepoBackendGithubToml;
use zbobr_task_backend_github::ZbobrTaskBackendGithubToml;
use zbobr_utility::{
    TomlOption,
    toml_edit_util::{
        inline_child_table, inline_named_children_as_inline_table_arrays,
        inline_named_children_as_inline_tables, set_child_table_dotted,
    },
};

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

const PROMPTS_SUBDIR: &str = "prompts";
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
const ROLE_PLAN_REVIEWER: Role = Role::new("plan_reviewer");
const ROLE_WORKER: Role = Role::new("worker");
const ROLE_REVIEWER: Role = Role::new("reviewer");
const ROLE_TESTER: Role = Role::new("tester");
const ROLE_LINTER: Role = Role::new("linter");
const ROLE_LINTER_WORKER: Role = Role::new("linter_worker");
const ROLE_MERGER: Role = Role::new("merger");

const STAGE_PLANNING: Stage = Stage::new("planning");
const STAGE_PLAN_REVIEW_ADVERSARIAL: Stage = Stage::new("plan_review_adversarial");
const STAGE_PLAN_REVIEW_USER: Stage = Stage::new("plan_review_user");
const STAGE_CALL_WORK: Stage = Stage::new("call_work");
const STAGE_WORKING: Stage = Stage::new("working");
const STAGE_PAUSE_RETRY_WORKING: Stage = Stage::new("pause_retry_working");
const STAGE_REVIEWING: Stage = Stage::new("reviewing");
const STAGE_LINTING: Stage = Stage::new("linting");
const STAGE_LINTER_WORKER: Stage = Stage::new("linter_worker");
const STAGE_TESTING: Stage = Stage::new("testing");
const STAGE_MERGING: Stage = Stage::new("merging");

const PIPELINE_PLAN: Pipeline = Pipeline::new("plan");
const PIPELINE_WORK: Pipeline = Pipeline::new("work");
const PIPELINE_MERGE: Pipeline = Pipeline::new("merge");

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
    let mut doc: DocumentMut = pretty.parse()?;
    inline_stage_tables(&mut doc);
    inline_role_prompt_tables(&mut doc);
    inline_dispatcher_tables(&mut doc);
    set_child_table_dotted(&mut doc, "repo", "github_token");
    set_child_table_dotted(&mut doc, "tasks", "github_token");
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

fn prompt_path(name: &str) -> PathBuf {
    PathBuf::from(PROMPTS_SUBDIR).join(name)
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
                    model: CLAUDE_MODEL_OPUS,
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
                    model: CLAUDE_MODEL_OPUS,
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
            providers: Some(providers).into(),
            tools: Some(tools).into(),
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
            prompts: workflow.prompts.into(),
            roles: workflow.roles.into(),
            pipelines: workflow.pipelines.into(),
            on_start: workflow.on_start.into(),
            on_merge: workflow.on_merge.into(),
        }),
    }
}

/// Build the default workflow configuration with predefined pipelines and roles.
fn default_workflow() -> WorkflowConfig {
    use McpTool::{
        AddChecklistItem, CheckChecklistItem, GetCtxRec, ReportFailure, ReportIntermediate,
        ReportSuccess, SetDestinationBranch, StopWithError, StopWithQuestion,
    };

    let plan_stages = IndexMap::from([
        (
            STAGE_PLANNING,
            StageDefinition {
                role: TomlOption::Value(ROLE_PLANNER),
                on_success: TomlOption::Value(StageTransition::stage(STAGE_PLAN_REVIEW_ADVERSARIAL)),
                on_intermediate: TomlOption::Value(StageTransition::stage(STAGE_PLAN_REVIEW_ADVERSARIAL)),
                ..Default::default()
            },
        ),
        (
            STAGE_PLAN_REVIEW_ADVERSARIAL,
            StageDefinition {
                role: TomlOption::Value(ROLE_PLAN_REVIEWER),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_PLANNING)),
                on_intermediate: TomlOption::Value(StageTransition::stage(STAGE_PLAN_REVIEW_USER)),
                ..Default::default()
            },
        ),
        (
            STAGE_PLAN_REVIEW_USER,
            StageDefinition {
                pause: true,
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_PLANNING)),
                ..Default::default()
            },
        ),
        (
            STAGE_CALL_WORK,
            StageDefinition {
                call: TomlOption::Value(PIPELINE_WORK),
                ..Default::default()
            },
        ),
    ]);

    let work_stages = IndexMap::from([
        (
            STAGE_WORKING,
            StageDefinition {
                role: TomlOption::Value(ROLE_WORKER),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_PAUSE_RETRY_WORKING)),
                on_intermediate: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
                on_success: TomlOption::Value(StageTransition::stage(STAGE_REVIEWING)),
                ..Default::default()
            },
        ),
        (
            STAGE_PAUSE_RETRY_WORKING,
            StageDefinition {
                pause: true,
                on_success: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
                on_no_report: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_REVIEWING)),
                ..Default::default()
            },
        ),
        (
            STAGE_REVIEWING,
            StageDefinition {
                role: TomlOption::Value(ROLE_REVIEWER),
                on_failure: TomlOption::Value(StageTransition::stage(STAGE_WORKING)),
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
            prompts: Some(indexmap::IndexMap::from([(
                "task".to_string(),
                TomlOption::ExplicitNone,
            )])),
            ..Default::default()
        },
    )]);

    let mut pipelines = IndexMap::new();
    pipelines.insert(
        PIPELINE_PLAN,
        PipelineConfig {
            stages: Some(
                plan_stages
                    .into_iter()
                    .map(|(k, v)| (k, TomlOption::Value(v)))
                    .collect(),
            ),
        },
    );
    pipelines.insert(
        PIPELINE_WORK,
        PipelineConfig {
            stages: Some(
                work_stages
                    .into_iter()
                    .map(|(k, v)| (k, TomlOption::Value(v)))
                    .collect(),
            ),
        },
    );
    pipelines.insert(
        PIPELINE_MERGE,
        PipelineConfig {
            stages: Some(
                merge_stages
                    .into_iter()
                    .map(|(k, v)| (k, TomlOption::Value(v)))
                    .collect(),
            ),
        },
    );

    fn role_prompts(main: &str) -> Option<indexmap::IndexMap<String, TomlOption<PathBuf>>> {
        Some(indexmap::IndexMap::from([(
            "main".to_string(),
            TomlOption::Value(prompt_path(main)),
        )]))
    }

    let roles = IndexMap::from([
        (
            ROLE_PLANNER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    StopWithQuestion,
                    ReportIntermediate,
                    ReportSuccess,
                    GetCtxRec,
                    SetDestinationBranch,
                ]),
                prompts: role_prompts("planner.md"),
                tool: TomlOption::Value(TOOL_PLANNER),
            },
        ),
        (
            ROLE_PLAN_REVIEWER,
            RoleDefinition {
                mcp: Some(vec![
                    StopWithError,
                    StopWithQuestion,
                    ReportSuccess,
                    ReportFailure,
                    GetCtxRec,
                ]),
                prompts: role_prompts("plan_reviewer.md"),
                tool: TomlOption::Value(TOOL_REVIEWER),
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
            TomlOption::Value(prompt_path(TASK_PROMPT)),
        ),
    ]));

    WorkflowConfig {
        prompts: workflow_prompts,
        pipelines: Some(
            pipelines
                .into_iter()
                .map(|(k, v)| (k, TomlOption::Value(v)))
                .collect(),
        ),
        roles: Some(
            roles
                .into_iter()
                .map(|(k, v)| (k, TomlOption::Value(v)))
                .collect(),
        ),
        on_start: Some("plan".into()),
        on_merge: Some("merge".into()),
    }
}

/// Convert `workflow.pipelines.*.stages.*` entries from standard tables to inline tables.
fn inline_stage_tables(doc: &mut DocumentMut) {
    let Some(Item::Table(workflow)) = doc.get_mut("workflow") else {
        return;
    };
    let Some(Item::Table(pipelines)) = workflow.get_mut("pipelines") else {
        return;
    };
    for (_pname, pipeline_item) in pipelines.iter_mut() {
        let Some(pipeline) = pipeline_item.as_table_mut() else {
            continue;
        };
        let Some(Item::Table(stages)) = pipeline.get_mut("stages") else {
            continue;
        };
        inline_named_children_as_inline_tables(stages);
        stages.set_dotted(true);
    }
}

/// Convert `workflow.roles.*.prompts` entries from standard tables to inline tables.
fn inline_role_prompt_tables(doc: &mut DocumentMut) {
    let Some(Item::Table(workflow)) = doc.get_mut("workflow") else {
        return;
    };
    let Some(Item::Table(roles)) = workflow.get_mut("roles") else {
        return;
    };

    let keys: Vec<String> = roles.iter().map(|(k, _)| k.to_string()).collect();
    for key in &keys {
        if let Some(role_item) = roles.get_mut(key)
            && let Some(role_table) = role_item.as_table_mut()
        {
            inline_child_table(role_table, "prompts");
        }
        if let Some(mut k) = roles.key_mut(key) {
            k.fmt();
        }
    }
}

/// Convert `dispatcher.providers.*` and `dispatcher.tools.*` entries to inline tables/arrays.
fn inline_dispatcher_tables(doc: &mut DocumentMut) {
    let Some(Item::Table(dispatcher)) = doc.get_mut("dispatcher") else {
        return;
    };

    if let Some(Item::Table(providers)) = dispatcher.get_mut("providers") {
        inline_named_children_as_inline_tables(providers);
    }

    if let Some(Item::Table(tools)) = dispatcher.get_mut("tools") {
        inline_named_children_as_inline_table_arrays(tools);
    }
}

// ---------------------------------------------------------------------------
// Default prompt files
// ---------------------------------------------------------------------------

const PROMPT_FILES: &[(&str, &str)] = &[
    ("planner", PLANNER_PROMPT),
    ("plan_reviewer", PLAN_REVIEWER_PROMPT),
    ("worker", WORKER_PROMPT),
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

Read the task description and comments provided below in this prompt. Design an implementation plan for the task. The plan posted with `{mcp_report_success}` must be a standalone, ready-to-use document, not a conversational reply in the discussion. See more detailed workflow instructions below.

Work autonomously, try to solve problems independently. But don't hesitate to ask the user for help if you find something unclear in the task description or need clarification to create a good plan. Use `{mcp_stop_with_question}` for this purpose.

**You MUST end every session by calling exactly one MCP tool** — `{mcp_report_success}`, `{mcp_stop_with_question}`, or `{mcp_stop_with_error}`. Finishing without calling one of these tools is a protocol error.

## Access Model

- You can access the internet and run local commands.
- Use MCP `{mcp_report_success}` to submit the plan for review and implementation — **mandatory at the end of every successful planning session**
- Use MCP `{mcp_report_intermediate}` for responses to plan-review comments or direct user requests; keep those responses separate from the implementation plan document
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
    - The plan content must be systematic and logically organized so it can be executed without reading surrounding discussion.
    - If you need to answer plan-review feedback or direct user requests, send those answers separately via `{mcp_report_intermediate}` instead of mixing them into the plan.
5. If some instrument is required and you can't install it yourself, ask the user to install it with `{mcp_stop_with_question}`.
6. **Determine if the plan is clear and ready**:
   - If something is unclear or you have doubts, use `{mcp_stop_with_question}` to ask only focused question(s) with sufficient context to understand the question. Finish the session after asking.
   - Only if the plan is clear and no questions were posted, proceed to step 7.
7. **Call `{mcp_report_success}`** with the plan document written as a standalone artifact (not a conversational discussion reply). The plan must be systematic, logically organized, directly actionable, and include a brief rationale (why this approach was chosen, key design decisions, important constraints, chosen analog).

It's critical to finish work with either `{mcp_report_success}` or `{mcp_stop_with_question}` / `{mcp_stop_with_error}`. Only data returned with the mcp tools is recorded.
"#,
);

const PLAN_REVIEWER_PROMPT: &str = concat!(
    r#"# Plan Reviewer Agent

Review the proposed implementation plan and evaluate its soundness, completeness, and quality. You are an adversarial reviewer — your role is to find weaknesses, missing cases, architectural problems, or better alternatives.

"#,
    get_ctx_rec_guidance!(),
    r#"
## Access Model

- You can access the internet and run local commands.
- Use `{mcp_report_success}` if the plan is sound and ready for implementation.
- Use `{mcp_report_failure}` if the plan has significant issues that must be addressed before implementation.
- Use `{mcp_stop_with_question}` when you need clarification on the plan.
- Use `{mcp_stop_with_error}` only to report technical errors.

## Workspace isolation

Your working directory is already the repository with the work branch checked out. Inspect the codebase to validate the plan. Do NOT make any code changes.

## Workflow

1. Read the task description and the plan provided in the context.
2. **Inspect the codebase** to verify the plan's assumptions — check that the referenced analogs exist and that the proposed approach is consistent with existing conventions.
3. **Evaluate the plan critically** for:
   - Correctness: Does the proposed approach actually solve the problem?
   - Consistency: Does it follow the same patterns and style as existing code? Is the chosen analog appropriate?
   - Direction: Is the approach clear enough for a worker to implement without going in the wrong direction?
   - Risk: Are there simpler or safer alternatives that would better fit the codebase?
4. The plan is **architecture-level** — do not penalize it for lacking code snippets, exact file paths, or enumerated edge cases. The worker looks up those details. Only flag missing information if it would cause the worker to make fundamentally wrong choices.
5. Finish by calling one of:
   - `{mcp_report_success}` — the plan is sound and ready for implementation. You may include minor suggestions or observations in the message, but they must not block progress.
   - `{mcp_report_failure}` — the plan has significant architectural issues or fundamental misunderstandings; provide specific, actionable feedback so the planner can revise."#,
);

const WORKER_PROMPT: &str = concat!(
    r#"# Worker Agent

Implement the task according to the plan in the context. There can be multiple plan versions in the history — always use the **latest** (most recent) plan. Do not use earlier plan versions even if they appear first in the context.

**Your first job in every session is to maintain the checklist:**
- If there are no checklist items yet, read the plan and create them with `{mcp_add_checklist_item}` before writing any code. Break the plan into concrete implementation steps, including any tests you determine are needed.
- If checklist items exist, skip already-checked ones and process the remaining ones in order.
- Use `{mcp_check_checklist_item}` to mark an item done when its subtask is complete.
- Use `{mcp_add_checklist_item}` to add a new item whenever new work is identified: you discover something during implementation, the user requests changes in comments, or a reviewer's report requires follow-up that isn't covered by existing items.

**You own testing.** After implementing the feature, decide whether new tests are needed and add them as checklist items. Prefer tests that validate observable behavior, transitions, and integration boundaries over tests that snapshot static content.
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

Your working directory is already the repository with the work branch checked out. Do not make changes in the destination branch: this is for reference only. Do NOT fetch or use any other branches. If you need temporary or experimental branches, prefix their names with the work branch name to avoid interfering with other agents.

Work autonomously. Do not ask the user for anything unless the task genuinely requires human input.

## Workflow

1. Read the task description, context, and comments provided below in this prompt. The full history and checklist are available in the context section. **Identify the latest plan** — if multiple plan iterations exist, use only the most recent one; earlier versions are superseded.
2. **Maintain the checklist** (see above): create items from the plan if none exist, otherwise continue from unchecked items.
3. **Identify the analog referenced in the plan.** Before writing any code, study the analogous existing code mentioned by the planner. Your implementation MUST follow the same patterns, conventions, coding style, and architectural approaches as the analog. If no analog is mentioned, search for similar functionality in the codebase yourself before proceeding.
4. Implement the task by going through unchecked checklist items one by one. Commit work after implementing each item. **Follow the same patterns and style as the identified analog if one is available.**
5. When implementation for an item is complete, mark it done with `{mcp_check_checklist_item}` (pass the ctx_rec_N id).
6. If you sense your context window is getting close to its limit, finish your current item to a buildable state, commit your work, mark completed items as done, call `{mcp_report_intermediate}` with a summary of what you accomplished and what remains, and finish the session.
7. If you need human clarification or intervention, call `{mcp_stop_with_question}`. If the plan is unclear or requires adjustment, call `{mcp_report_failure}`. In case of technical errors use `{mcp_stop_with_error}`.
8. If some instrument is required and you can't install it yourself, ask the user to install it with `{mcp_stop_with_question}`.
9. When your current session's work is done, decide how to finish:
    - If **all checklist items are completed** (the full plan is done), call `{mcp_report_success}` to report final success.
    - If **some items remain unchecked** (more work is needed in future sessions), call `{mcp_report_intermediate}` to report what you accomplished so far.

## Coding Guidelines

- **Prefer deriving values from types and constants** rather than using hardcoded string literals. If a value can be computed from an existing type, enum variant, or constant, do it. Avoid duplicating the value as literals or constants."#,
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
5. Commit the fixes and changes made by linting/formatting tools
6. Call `{mcp_report_success}` if all issues were fixed and the fixes were committed.
7. Call `{mcp_report_failure}` with details if some issues cannot be fixed.

## Important Notes

- **Only fix formatting and linting** — do not modify logic, tests, or functionality.
- **Do not run tests** — functional testing is handled separately."#,
);

const MERGER_PROMPT: &str = r#"# Merger Agent

Resolve merge conflicts when the work branch cannot be automatically synchronized and commit the merge result.

## When Merger Runs

The framework attempted to merge changes into the work branch and encountered conflicts. The conflicts may come from merging the upstream base branch or from merging concurrent remote changes. The repository is in a mid-merge state with conflict markers in the affected files. Your job is to resolve those conflicts and complete the merge commit.

You are called exactly because both sides contain changes that Git could not combine automatically. Assume the work branch changes and the incoming branch changes are both intentional until you verify otherwise. The default goal is not to pick a side, but to produce a merged result that preserves the intended behavior from both sides.


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
    - Review the code in both branches to understand the intent of each side before editing
3. **Attempt automatic resolution:**
    - For simple, non-overlapping changes (e.g., formatting, imports, unrelated edits), apply manual fixes that combine both changes
    - For semantic conflicts, build the merged result deliberately: preserve behavior, flags, config keys, validation, and other logic introduced on both sides unless you can prove one side intentionally replaced the other
    - Treat conflicts in lists, feature arrays, dependency options, config maps, and struct fields as merge problems, not winner-take-all choices. If one side added or kept a capability and the other changed an adjacent value, usually the correct result keeps both unless they are truly incompatible
   - Edit each conflicted file to remove all conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) and produce a correct merged version
   - Use `git add <file>` for each resolved file, then `git commit -m "chore: merge conflicts resolved"` to complete the merge commit
    - Do NOT run `git merge` again — just resolve the markers and commit
    - Do NOT resolve a semantic conflict by blindly taking `ours`, `theirs`, or the shorter side without verifying that no intended behavior is lost
4. **If automatic resolution is not possible:**
    - Use `{mcp_stop_with_question}` when the intended merged behavior is unclear, when the two sides appear mutually exclusive, or when preserving both sides would require product or design judgement
    - Describe what each side changed, what options you considered, and why the correct merged result is ambiguous
   - Wait for user input before proceeding
5. **After successful resolution:**
   - Ensure all your changes are explicitly committed using `git commit` to the local work branch
6. Call `{mcp_report_success}` to provide a brief and concise report of your work and finish the session. This report is critical context for further agent calls, so it MUST be compact.

## Conflict Resolution Principles

- Start from the assumption that both sides contain valuable changes and try to preserve the intent of both sides in the final merged file
- Combine non-overlapping changes from both branches (destination and work) when possible
- Prefer semantic merges over textual merges: keep version bumps together with feature additions, keep refactors together with bug fixes, and keep new options unless there is clear evidence they must be removed
- Watch for silent regressions where a manually resolved conflict drops a capability from one side, such as removing a feature flag, config key, dependency option, or validation rule while keeping an unrelated change from the other side
- If the correct merged behavior is unclear after reviewing both sides, ask the user instead of guessing which version is preferred
- Do NOT delete either branch's work without explicit user guidance"#;

#[cfg(test)]
mod tests {
    use super::*;

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
        let work = wf.pipeline(&PIPELINE_WORK).unwrap();
        let linting = work.stage(&Stage::from("linting")).unwrap();
        let target = linting.on_success().and_then(|t| t.next.as_deref());
        assert_eq!(target, Some("testing"));
    }

    #[test]
    fn merge_stage_task_prompt_is_cleared() {
        let wf = default_workflow();
        let merge = wf.pipeline(&PIPELINE_MERGE).unwrap();
        let merging = merge.stage(&Stage::from("merging")).unwrap();
        let task_prompt = merging.prompts.as_ref().and_then(|map| map.get("task"));
        assert_eq!(task_prompt, Some(&TomlOption::ExplicitNone));
    }

    #[test]
    fn linting_on_failure_routes_to_linter_worker() {
        let wf = default_workflow();
        let work = wf.pipeline(&PIPELINE_WORK).unwrap();
        let linting = work.stage(&Stage::from("linting")).unwrap();
        let target = linting.on_failure().and_then(|t| t.next.as_deref());
        assert_eq!(target, Some("linter_worker"));
    }

    #[test]
    fn linter_worker_on_success_routes_to_linting() {
        let wf = default_workflow();
        let work = wf.pipeline(&PIPELINE_WORK).unwrap();
        let lw = work.stage(&Stage::from("linter_worker")).unwrap();
        let target = lw.on_success().and_then(|t| t.next.as_deref());
        assert_eq!(target, Some("linting"));
    }

    #[test]
    fn linter_worker_on_failure_routes_to_working() {
        let wf = default_workflow();
        let work = wf.pipeline(&PIPELINE_WORK).unwrap();
        let lw = work.stage(&Stage::from("linter_worker")).unwrap();
        let target = lw.on_failure().and_then(|t| t.next.as_deref());
        assert_eq!(target, Some("working"));
    }

    // ── PROMPT_FILES completeness tests ────────────────────────────────

    #[test]
    fn all_default_workflow_role_prompts_are_registered() {
        let wf = default_workflow();
        let registered: std::collections::HashSet<&str> =
            PROMPT_FILES.iter().map(|(name, _)| *name).collect();
        for (role_name, role_def) in wf.get_roles().unwrap_or(&indexmap::IndexMap::new()) {
            if let Some(role_def) = role_def.as_option() {
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
    async fn init_workspace_creates_loop_script() {
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
            loop_script.contains("task advance"),
            "loop.sh should run task advance"
        );
        assert!(
            loop_script.contains("task process --select"),
            "loop.sh should run task process --select"
        );
        assert!(
            loop_script.contains("cleanup"),
            "loop.sh should run cleanup"
        );
    }

}
