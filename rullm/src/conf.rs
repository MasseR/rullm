use std::{
    collections::HashMap,
    path::PathBuf,
};

use anyhow::anyhow;
use config::{Config, File};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Conf {
    pub executables: HashMap<String, String>,
    pub environment: HashMap<String, String>,
    pub llm: LLMConfig,
}

#[derive(Deserialize, Debug)]
pub struct LLMConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

impl Conf {
    pub fn build(override_path: Option<PathBuf>) -> anyhow::Result<Conf> {
        let config_file = dirs_next::config_dir().map(|mut config_dir: PathBuf| {
            config_dir.push("rullm");
            config_dir.push("config.toml");
            config_dir
        });
        let path = override_path
            .or(config_file)
            .ok_or(anyhow!("Configuration file missing"))?;
        let settings = Config::builder().add_source(File::from(path)).build()?;
        let conf = settings.try_deserialize::<Conf>()?;
        Ok(conf)
    }
}
