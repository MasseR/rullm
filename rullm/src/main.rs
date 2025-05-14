use clap::Parser;
use rullm::{args::Args, env::Env};
use tracing_subscriber::{Registry, layer::SubscriberExt as _};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let journald_layer = tracing_journald::layer()?;
    let subscriber = Registry::default().with(journald_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();
    let env = Env::build(args).await?;
    rullm::chat::run(env).await
}
