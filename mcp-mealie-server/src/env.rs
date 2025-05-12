use crate::{conf::Conf, mealie::MealieClient};

// Configuration and env
#[derive(Clone)]
pub struct Env {
    pub api_client: MealieClient,
    pub list_id: String,
}

impl Env {
    pub async fn build(conf: Conf) -> anyhow::Result<Env> {
        let list_id = conf.list_id.clone();
        let api_client = MealieClient::build(conf)?;
        Ok(Env { api_client, list_id })
    }
}
