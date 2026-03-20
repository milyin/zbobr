extern crate self as zbobr_api;

pub mod backend;
pub mod config;
pub mod prompt;
pub mod task;
pub mod tool_executor;

pub use backend::{TaskBackend, TaskBackendExt, TaskMut, TaskWeak, WorktreeBackend};
pub use config::{
    Config, PipelineConfig, StageDefinition, WorkflowArgs, WorkflowConfig, WorkflowToml,
    ZbobrDispatcherArgs, ZbobrDispatcherConfig, ZbobrDispatcherToml,
};
pub use task::{
    ChecklistItem, Comment, CommentTag, HistoryIndex, HistoryIndexEntry, HistoryRecordType, Model,
    Role, StackEntry, Task, TaskIdentity, Tool, build_history_index, extract_repo_name,
    get_history_record_by_index,
};
pub use tool_executor::{ToolExecutor, format_command_for_log};
