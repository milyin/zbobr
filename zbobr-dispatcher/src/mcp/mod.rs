pub mod common;
pub mod instructions;
pub mod traits;

// Re-export commonly used tool name modules from `common` so they are
// available as `zbobr_dispatcher::mcp::<role>_tools` for tests and callers.
pub use common::merger_tools;
pub use common::planner_tools;
pub use common::preparator_tools;
pub use common::reviewer_tools;
pub use common::worker_tools;

pub mod merger;
pub mod planner;
pub mod preparator;
pub mod reviewer;
pub mod worker;

pub use merger::MergerMcp;
pub use planner::PlannerMcp;
pub use preparator::PreparatorMcp;
pub use reviewer::ReviewerMcp;
pub use worker::WorkerMcp;

pub use instructions::{
    merger_instructions, planner_instructions, preparator_instructions, reviewer_instructions,
    worker_instructions,
};

pub use common::{
    CheckChecklistItemParam, DeleteChecklistItemParam, DescriptionParam, InsertChecklistItemParam,
    MessageParam, SetDestinationBranchParam, SetDestinationRepositoryParam, SetWorkBranchParam,
    UpdateChecklistItemParam, run_role_mcp_server,
};
