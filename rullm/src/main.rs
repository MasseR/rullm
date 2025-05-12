use clap::Parser;
use rullm::{args::Args, env::Env};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let env = Env::build(args).await?;
    rullm::chat::run(env).await
}
