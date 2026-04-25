//! Shared CLI helpers for bootstrap/init/onboarding commands.
//!
//! These functions reuse the generic project CLI config/app builders but keep
//! bootstrap commands grouped behind their own small surface.

use anyhow::Result;

use crate::project_cli_support::{create_app, print_json, resolve_config};

/// Resolves the app config used by bootstrap-facing CLI commands.
pub fn resolve_bootstrap_config(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
    print_error_human: fn(&str),
    print_error_json: fn(&str) -> Result<()>,
) -> Result<mempalace_rs::config::AppConfig> {
    resolve_config(
        palace,
        hf_endpoint,
        human,
        print_error_human,
        print_error_json,
    )
}

/// Constructs an application facade for bootstrap-facing CLI commands.
pub fn create_bootstrap_app(
    config: mempalace_rs::config::AppConfig,
    human: bool,
    print_error_human: fn(&str),
    print_error_json: fn(&str) -> Result<()>,
) -> Result<mempalace_rs::service::App> {
    create_app(config, human, print_error_human, print_error_json)
}

/// Prints a bootstrap command payload as pretty JSON.
pub fn print_bootstrap_json<T: serde::Serialize>(value: &T) -> Result<()> {
    print_json(value)
}
