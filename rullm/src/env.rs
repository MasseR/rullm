use crate::{args::Args, conf::Conf, mcp::MCP, openai::OpenAIClient};

pub struct Env {
    pub openai_client: OpenAIClient,
    pub mcp: MCP,
    pub conf: Conf,
}

impl Env {
    pub async fn build(args: Args) -> anyhow::Result<Env> {
        let conf = Conf::build(args.conf_file)?;
        let mcp = MCP::build(&conf).await?;
        let openai_client = OpenAIClient::build(&conf, &mcp).await?;
        Ok(Env {
            openai_client,
            mcp,
            conf,
        })
    }
}
