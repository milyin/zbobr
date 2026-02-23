//! Module collecting all MCP tester scenario helpers.
//!
//! Each scenario is defined in its own file and re-exported here so the integration
//! test can import them conveniently.

mod assert_false;
mod dummy;

pub use assert_false::assert_false_scenario;
