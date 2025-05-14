use mcp_mealie_server::{conf::Conf, env::Env, mcp::Mealie};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{layer::SubscriberExt as _, Registry};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Set up logging to journald
    // You can access the logs with `journalctl --user -t mcp-mealie-server`
    // Consider adding `--output json | jq | less` if you want to see the extra
    // fields that tracing adds
    let journald_layer = tracing_journald::layer()?;
    let subscriber = Registry::default().with(journald_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    let conf = Conf::parse().await?;
    let env = Env::build(conf).await?;
    let service = Mealie::new(env).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
