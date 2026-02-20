//! Module collecting all MCP tester scenario helpers.
//!
//! Each scenario is defined in its own file and re-exported here so the integration
//! test can import them conveniently.

mod dummy;
mod preparator_comprehensive;

pub use dummy::dummy_scenario;
pub use preparator_comprehensive::preparator_comprehensive_scenario;
