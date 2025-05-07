use std::error::Error;

use async_openai::{Client, config::OpenAIConfig};

use crate::{args::Args, mcp::MCP};

pub struct Env {
    pub client: Client<OpenAIConfig>,
    pub args: Args,
    pub mcp: MCP,
}

impl Env {
    pub async fn build(args: Args) -> Result<Env, Box<dyn Error>> {
        let client = Client::new();
        let mcp = MCP::build(&args).await?;
        Ok(Env { client, args, mcp })
    }
}
