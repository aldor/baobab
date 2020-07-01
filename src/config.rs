use std::fs;

use anyhow;
use anyhow::Context;
use dirs;
use serde::Deserialize;
use serde_yaml;

static CONFIG_NAME: &str = ".bb.yaml";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub teamcity_username: String,
    pub teamcity_password: String,
}

pub fn read_config() -> anyhow::Result<Config> {
    let mut path = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("failed to get home dir"))?;
    path.push(CONFIG_NAME);
    let config = fs::read_to_string(path.as_path())
        .with_context(|| format!("failed to read {}", CONFIG_NAME))?;
    let config = serde_yaml::from_str(config.as_str())
        .with_context(|| format!("failed to parse {}", CONFIG_NAME))?;
    Ok(config)
}
