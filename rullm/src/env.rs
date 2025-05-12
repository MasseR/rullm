use crate::{args::Args, conf::Conf, mcp::MCP, openai::OpenAIClient};

pub struct Env {
    pub openai_client: OpenAIClient,
    pub args: Args,
    pub mcp: MCP,
    pub conf: Conf,
}

impl Env {
    pub async fn build(args: Args) -> anyhow::Result<Env> {
        let conf = Conf::build(None)?;
        let mcp = MCP::build(&conf).await?;
        let openai_client = OpenAIClient::build(&conf, &mcp).await?;
        Ok(Env {
            openai_client,
            args,
            mcp,
            conf,
        })
    }
}
