pub mod common;
pub mod instructions;
pub mod traits;

// Re-export commonly used tool name modules from `common` so they are
// available as `zbobr_dispatcher::mcp::<role>_tools` for tests and callers.
pub use common::{
    analyser_tools, decompose_planner_tools, decomposer_tools, merger_tools, planner_tools, 
    preparator_tools, reviewer_tools, tester_tools, worker_tools,
};

pub mod analyser;
pub mod decompose_planner;
pub mod decomposer;
pub mod merger;
pub mod planner;
pub mod preparator;
pub mod reviewer;
pub mod tester;
pub mod worker;

pub use analyser::AnalyserMcp;
pub use common::{
    CheckChecklistItemParam, DeleteChecklistItemParam, DescriptionParam, InsertChecklistItemParam,
    MessageParam, SetDestinationBranchParam, SetDestinationRepositoryParam, SetWorkBranchParam,
    UpdateChecklistItemParam, run_role_mcp_server,
};
pub use decompose_planner::DecomposePlannerMcp;
pub use decomposer::DecomposerMcp;
pub use instructions::{
    analyser_instructions, decompose_planner_instructions, decomposer_instructions,
    merger_instructions, planner_instructions, preparator_instructions,
    reviewer_instructions, tester_instructions, worker_instructions,
};
pub use merger::MergerMcp;
pub use planner::PlannerMcp;
pub use preparator::PreparatorMcp;
pub use reviewer::ReviewerMcp;
pub use tester::TesterMcp;
pub use worker::WorkerMcp;
