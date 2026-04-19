use anyhow::Result;

use crate::project_cli_support::{create_app, print_json, resolve_config};

pub fn resolve_mining_config(
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

pub fn create_mining_app(
    config: mempalace_rs::config::AppConfig,
    human: bool,
    print_error_human: fn(&str),
    print_error_json: fn(&str) -> Result<()>,
) -> Result<mempalace_rs::service::App> {
    create_app(config, human, print_error_human, print_error_json)
}

pub fn print_mining_json<T: serde::Serialize>(value: &T) -> Result<()> {
    print_json(value)
}
