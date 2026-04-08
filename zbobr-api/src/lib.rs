extern crate self as zbobr_api;

pub mod backend;
pub mod config;
pub mod config_tools;
pub mod context;
pub mod prompt;
pub mod secret;
pub mod task;
pub mod tool_executor;

pub use backend::{
    ERROR_PREFIX, PAUSE_PREFIX, QUESTION_PREFIX, TaskBackend, TaskBackendExt, TaskMut, TaskWeak,
    WorktreeBackend, format_status,
};
pub use config::{
    Config, PipelineConfig, StageDefinition, StageTransition, WorkflowArgs, WorkflowConfig,
    WorkflowToml, ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
};
pub use context::format_timestamp;
pub use secret::Secret;
pub use task::{
    Comment, ContextRecord, ContextRecordType, Executor, HistoryRecordType, Model, Pipeline,
    Signal, StackEntry, Stage, StageContext, StageInfo, State, Task, TaskContext, TaskIdentity,
    classify_comment, extract_repo_name,
};
pub use tool_executor::{ExecutorOutput, ToolExecutor, format_command_for_log};
