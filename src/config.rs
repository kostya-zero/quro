use std::{fs, path::PathBuf};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read a configuration file at '{0}': {1}")]
    ReadFailed(String, String),

    #[error("configuration has a bad format: {0}")]
    BadFormat(String),
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BorderStyle {
    None,
    #[default]
    Rounded,
    Modern,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub appearence: Appearance,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Appearance {
    pub border_style: BorderStyle,
}

pub fn config_path() -> PathBuf {
    dir_spec::config_home()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("quro")
        .join("config.toml")
}

pub fn load_config() -> Result<Config, ConfigError> {
    let path = config_path();
    let content = fs::read_to_string(&path)
        .map_err(|e| ConfigError::ReadFailed(path.to_string_lossy().to_string(), e.to_string()))?;
    let config: Config =
        toml::from_str(&content).map_err(|e| ConfigError::BadFormat(e.to_string()))?;
    Ok(config)
}
