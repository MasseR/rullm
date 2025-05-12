use std::env;

use anyhow::Context as _;

#[derive(Clone)]
pub struct Conf {
  pub api_key: String,
  pub base_url: String,
  pub list_id: String,
}

impl Conf {
  pub async fn parse() -> anyhow::Result<Conf> {
    let api_key : String = env::var("MEALIE_API_KEY").context("Missing MEALIE_API_KEY")?;
    let base_url : String = env::var("MEALIE_BASE_URL").context("Missing MEALIE_BASE_URL")?;
    let list_id : String = env::var("MEALIE_LIST_ID").context("Missing MEALIE_LIST_ID")?;
    Ok(Conf{api_key, base_url, list_id})
  }
}
