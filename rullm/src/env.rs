use anyhow::bail;
use async_openai::{Client, config::OpenAIConfig};
use config::{Config, File};

use crate::{
    args::{Args, Conf},
    mcp::MCP,
};

pub struct Env {
    pub client: Client<OpenAIConfig>,
    pub args: Args,
    pub mcp: MCP,
    pub conf: Conf,
}

impl Env {
    pub async fn build(args: Args) -> anyhow::Result<Env> {
        if let Some(mut config_dir) = dirs_next::config_dir() {
            config_dir.push("rullm");
            config_dir.push("config.toml");
            let settings = Config::builder()
                .add_source(File::from(config_dir))
                .build()?;
            let conf = settings.try_deserialize::<Conf>()?;
            let openai_base = conf
                .llm
                .base_url
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or(String::from("https://api.openai.com/v1"));
            let openai_config = OpenAIConfig::default()
                .with_api_key(&conf.llm.api_key)
                .with_api_base(openai_base);
            let client = Client::with_config(openai_config);
            let mcp = MCP::build(&conf).await?;
            Ok(Env {
                client,
                args,
                mcp,
                conf,
            })
        } else {
            bail!("Config directory not found")
        }
    }
}
