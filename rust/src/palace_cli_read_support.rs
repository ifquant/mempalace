use anyhow::Result;
use mempalace_rs::config::AppConfig;

use crate::cli_support::{palace_exists, print_no_palace};
use crate::palace_cli_support::{create_app, print_json, resolve_config};

pub fn resolve_read_config(
    palace: Option<&std::path::PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
    print_error_human: fn(&str),
    print_error_json: fn(&str) -> Result<()>,
) -> Result<AppConfig> {
    resolve_config(
        palace,
        hf_endpoint,
        human,
        print_error_human,
        print_error_json,
    )
}

pub fn create_read_app(
    config: AppConfig,
    human: bool,
    print_error_human: fn(&str),
    print_error_json: fn(&str) -> Result<()>,
) -> Result<mempalace_rs::service::App> {
    create_app(config, human, print_error_human, print_error_json)
}

pub fn exit_if_no_palace_human_or_json(
    config: &AppConfig,
    human: bool,
    print_no_palace_human: fn(&AppConfig),
) -> Result<()> {
    if !palace_exists(config) {
        if human {
            print_no_palace_human(config);
        } else {
            print_no_palace(config)?;
        }
        std::process::exit(1);
    }
    Ok(())
}

pub fn print_read_json<T: serde::Serialize>(value: &T) -> Result<()> {
    print_json(value)
}
