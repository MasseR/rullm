use std::{env, error::Error};

use reqwest::Client;

// Configuration and env
#[derive(Clone)]
pub struct Env {
  pub api_client: Client,
  pub conf: Conf,
}

#[derive(Clone)]
pub struct Conf {
  pub api_key: String,
  pub base_url: String,
  pub list_id: String,
}

impl Conf {
  pub async fn parse() -> Result<Conf, Box<dyn Error>> {
    let api_key : String = env::var("MEALIE_API_KEY")
      .map_err(|_err| {Box::<dyn Error>::from("Missing MEALIE_API_KEY")})?;
    let base_url : String = env::var("MEALIE_BASE_URL")
      .map_err(|_err| {Box::<dyn Error>::from("Missing MEALIE_BASE_URL")})?;
    let list_id : String = env::var("MEALIE_LIST_ID")
      .map_err(|_err| {Box::<dyn Error>::from("Missing MEALIE_LIST_ID")})?;
    Ok(Conf{api_key, base_url, list_id})
  }
}

impl Env {
  pub async fn build(conf: Conf) -> Result<Env, Box<dyn Error>> {
    let api_client = Client::new();
    Ok(Env{api_client, conf})
  }
}
