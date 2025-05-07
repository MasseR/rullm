use std::error::Error;

use clap::Parser;
use rullm::{args::Args, env::Env};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let env = Env::build(args).await?;
    rullm::run(env).await
}
