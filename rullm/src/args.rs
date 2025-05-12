use std::collections::HashMap;

use clap::Parser;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    // This should be a vector eventually
    pub conf_file: Option<String>,
}

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
