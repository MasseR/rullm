use futures::StreamExt;
use rmcp::{model::ServerInfo, schemars, tool, ServerHandler};
use serde::Deserialize;
use crate::mealie;

use crate::env::Env;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NewItemRequest {
  #[schemars{description="Name of the shopping list item"}]
  pub name: String,
}

#[derive(Clone)]
pub struct ShoppingLists {
  env: Env,
}

#[tool(tool_box)]
impl ShoppingLists {
  pub fn new(env: Env) -> ShoppingLists {
    ShoppingLists{env}
  }

  #[tool(description = "A new item to the shopping list")]
  pub async fn add_to_list(&self, #[tool(aggr)] NewItemRequest{name}: NewItemRequest) -> String {
    let list_id = &self.env.conf.list_id;
    match mealie::new_shopping_list_item(&self.env, &list_id, &name).await {
      Ok(_) => String::from(format!("Successfully added '{name}'")),
      Err(err) => String::from(format!("Failed to add the item: {}", err.to_string()))
    }
  }

  #[tool(description = "See what is in the shopping list currently")]
  pub async fn current_items(&self) -> String {
    let list_id = &self.env.conf.list_id;
    let items : Vec<String> = mealie::get_all_shopping_list_items(&self.env, &list_id)
      .filter_map(|x| async move {
        match x {
          Ok(item) => { if item.checked { None } else { Some(format!("- {}", item.display)) } },
          Err(_) => None
        }
      })
      .collect::<Vec<String>>()
      .await;
    String::from(items.join("\n"))
  }
}

#[tool(tool_box)]
impl ServerHandler for ShoppingLists {
  fn get_info(&self) -> ServerInfo {
    ServerInfo {
      instructions: Some("Mealie shopping lists".into()),
      ..Default::default()
    }
  }
}
