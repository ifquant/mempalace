//! CLI façade for bootstrap-related subcommands.
//!
//! The concrete handlers stay split so init and onboarding can evolve
//! independently while sharing output/config helpers.

pub use crate::project_cli_bootstrap_init::handle_init;
pub use crate::project_cli_bootstrap_onboarding::handle_onboarding;
