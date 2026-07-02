use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

use crate::paths;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub connection: String,
    #[serde(default)]
    pub database: String,
}

pub fn default_config_file() -> PathBuf {
    paths::config_file()
}

pub fn load(path: Option<&str>) -> Result<Config> {
    let path = path.map(PathBuf::from).unwrap_or_else(default_config_file);
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read config file {}", path.display()))?;
    let mut cfg: Config = serde_yaml::from_str(&text)
        .with_context(|| format!("parse config file {}", path.display()))?;
    trim(&mut cfg.host);
    trim(&mut cfg.connection);
    trim(&mut cfg.database);
    Ok(cfg)
}

fn trim(s: &mut String) {
    *s = s.trim().to_string();
}
