use crate::mealie::{self, ShoppingListItem};
use futures::StreamExt;
use rmcp::Error;
use rmcp::model::{CallToolResult, Content, IntoContents as _};
use rmcp::{ServerHandler, model::ServerInfo, schemars, tool};
use serde::Deserialize;

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
        ShoppingLists { env }
    }

    #[tool(description = "A new item to the shopping list")]
    pub async fn add_to_list(
        &self,
        #[tool(aggr)] NewItemRequest { name }: NewItemRequest,
    ) -> Result<CallToolResult, Error> {
        let list_id = &self.env.conf.list_id;
        match mealie::new_shopping_list_item(&self.env, &list_id, &name).await {
          Ok(_) => {
                Ok(CallToolResult::success(Content::text(format!("Successfully added '{name}'")).into_contents()))
            }
          Err(err) => Err(Error::invalid_request(format!("Failed to add item: {:?}", err), None))
        }
    }

    #[tool(description = "See what is in the shopping list currently")]
    pub async fn current_items(&self) -> Result<CallToolResult, Error> {
        let list_id = &self.env.conf.list_id;
        let items: Vec<ShoppingListItem> = mealie::get_all_shopping_list_items(&self.env, &list_id)
            .filter_map(|x| async move {
                match x {
                    Ok(item) => {
                        if item.checked {
                            None
                        } else {
                            Some(item)
                        }
                    }
                    Err(_) => None,
                }
            })
            .collect::<Vec<ShoppingListItem>>()
            .await;
        Ok(CallToolResult::success(Content::json(items)?.into_contents()))
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
