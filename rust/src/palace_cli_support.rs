use std::path::PathBuf;

use anyhow::Result;
use mempalace_rs::config::AppConfig;
use mempalace_rs::service::App;

use crate::cli_support::apply_cli_overrides;

pub fn resolve_config(
    palace: Option<&PathBuf>,
    hf_endpoint: Option<&str>,
    human: bool,
    print_human_error: fn(&str),
    print_json_error: fn(&str) -> Result<()>,
) -> Result<AppConfig> {
    let mut config = match AppConfig::resolve(palace) {
        Ok(config) => config,
        Err(err) if human => {
            print_human_error(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_json_error(&err.to_string())?;
            std::process::exit(1);
        }
    };
    apply_cli_overrides(&mut config, hf_endpoint);
    Ok(config)
}

pub fn create_app(
    config: AppConfig,
    human: bool,
    print_human_error: fn(&str),
    print_json_error: fn(&str) -> Result<()>,
) -> Result<App> {
    match App::new(config) {
        Ok(app) => Ok(app),
        Err(err) if human => {
            print_human_error(&err.to_string());
            std::process::exit(1);
        }
        Err(err) => {
            print_json_error(&err.to_string())?;
            std::process::exit(1);
        }
    }
}

pub fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
