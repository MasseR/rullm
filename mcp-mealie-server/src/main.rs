use std::error::Error;
use mcp_mealie_server::{conf::Conf, env::Env, mcp::ShoppingLists};
use rmcp::{ServiceExt, transport::stdio};


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let conf = Conf::parse().await?;
  let env = Env::build(conf).await?;
  let service = ShoppingLists::new(env).serve(stdio()).await?;
  service.waiting().await?;
  Ok(())
}
