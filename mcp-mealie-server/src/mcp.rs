use std::collections::HashSet;

use crate::mealie::ShoppingListItem;
use futures::StreamExt;
use rmcp::Error;
use rmcp::model::{CallToolResult, Content, IntoContents as _};
use rmcp::{ServerHandler, model::ServerInfo, schemars, tool};
use serde::{Deserialize, Serialize};

use crate::env::Env;

#[derive(Serialize, Debug)]
pub struct FilteredItem {
    pub name: String,
    pub label: Option<String>,
    pub checked: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ItemRequest {
    #[schemars{description="Name of the shopping list item"}]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ManyItemRequest {
    #[schemars{description="List of shopping list items"}]
    pub names: Vec<String>,
}

#[derive(Clone)]
pub struct ShoppingLists {
    env: Env,
}

fn mark_named_items_as_checked(items: impl IntoIterator<Item = ShoppingListItem>, names: &[String]) -> Vec<ShoppingListItem> {
    let names: HashSet<&String> = HashSet::from_iter(names);
    items.into_iter()
    .filter_map(|mut item| {
        if names.contains(&item.note) {
            item.checked = true;
            Some(item)
        } else {
            None
        }
    })
    .collect::<Vec<ShoppingListItem>>()
}

#[tool(tool_box)]
impl ShoppingLists {
    pub fn new(env: Env) -> ShoppingLists {
        ShoppingLists { env }
    }

    #[tool(description = "Mark shopping list items as done")]
    pub async fn mark_as_done(
        &self,
        #[tool(aggr)] ManyItemRequest { names }: ManyItemRequest,
    ) -> Result<CallToolResult, Error> {
        let list_id = &self.env.list_id;
        let items: Vec<ShoppingListItem> = self
            .env
            .api_client
            .get_all_shopping_list_items(&list_id)
            .filter_map(|x| async move { x.ok() })
            .collect::<Vec<ShoppingListItem>>()
            .await;
        let items = mark_named_items_as_checked(items, &names);
        match self.env.api_client.update_shopping_list_items(&items).await {
            Ok(_) => Ok(CallToolResult::success(
                Content::text(format!("Marked as done")).into_contents()
            )),
            Err(err) => Err(Error::internal_error(format!("failed to mark as done: {:?}", err), None))
        }
    }

    #[tool(description = "A new item to the shopping list")]
    pub async fn add_to_list(
        &self,
        #[tool(aggr)] ItemRequest { name }: ItemRequest,
    ) -> Result<CallToolResult, Error> {
        let list_id = &self.env.list_id;
        match self
            .env
            .api_client
            .new_shopping_list_item(&list_id, &name)
            .await
        {
            Ok(_) => Ok(CallToolResult::success(
                Content::text(format!("Successfully added '{name}'")).into_contents(),
            )),
            Err(err) => Err(Error::invalid_request(
                format!("Failed to add item: {:?}", err),
                None,
            )),
        }
    }

    #[tool(description = "See what is in the shopping list currently")]
    pub async fn current_items(&self) -> Result<CallToolResult, Error> {
        let list_id = &self.env.list_id;
        let items: Vec<FilteredItem> = self
            .env
            .api_client
            .get_all_shopping_list_items(&list_id)
            .filter_map(|x| async move {
                match x {
                    Ok(item) => {
                        if item.checked {
                            None
                        } else {
                            Some(simplify(&item))
                        }
                    }
                    Err(_) => None,
                }
            })
            .collect::<Vec<FilteredItem>>()
            .await;
        Ok(CallToolResult::success(
            Content::json(items)?.into_contents(),
        ))
    }
}

fn simplify(item: &ShoppingListItem) -> FilteredItem {
    FilteredItem {
        name: item.note.clone(),
        label: item.label.as_ref().cloned().map(|x| x.name),
        checked: item.checked,
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


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_item() -> impl Strategy<Value = ShoppingListItem> {
        ("[a-z]{1,5}", "[a-z]{1,5}", "[a-z]{1,5}", any::<bool>()).prop_map(|(id,note,shopping_list_id,checked)| {
            ShoppingListItem{ id, note, checked, shopping_list_id, label: None }
        })
    }


    proptest! {

        #[test]
        fn test_foo(subset: usize, items in prop::collection::vec(arb_item(), 1..5)) {
            let subset = items[0..subset % items.len()].to_vec().into_iter().map(|x| x.note).collect::<Vec<String>>();
            let collected = mark_named_items_as_checked(items, &subset);
            for item in subset {
                let found = collected.clone().into_iter().find(|x| x.note == item);
                assert_eq!(Some(item), found.map(|x| x.note));
            }
        }
    }
}
