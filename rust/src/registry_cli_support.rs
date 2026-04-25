//! Shared CLI helpers for registry command families.
//!
//! Registry commands share the same app construction and JSON rendering logic, so
//! the read/write/research handlers only need to focus on routing and formatting.

use std::path::PathBuf;

use mempalace_rs::config::AppConfig;
use mempalace_rs::service::App;

use crate::cli_support::apply_cli_overrides;

/// Builds an application facade configured for registry commands.
pub fn build_registry_app(
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
) -> anyhow::Result<App> {
    let mut config = AppConfig::resolve(palace)?;
    apply_cli_overrides(&mut config, hf_endpoint);
    Ok(App::new(config)?)
}

/// Prints a registry payload as pretty JSON.
pub fn print_registry_json<T: serde::Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
