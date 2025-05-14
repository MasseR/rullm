use std::collections::HashSet;

use crate::mealie::{Recipe, RecipeIngredient, RecipeInstruction, ShoppingListItem};
use anyhow::bail;
use futures::StreamExt;
use rmcp::Error;
use rmcp::model::{CallToolResult, Content, IntoContents as _};
use rmcp::{ServerHandler, model::ServerInfo, schemars, tool};
use serde::{Deserialize, Serialize};
use tracing::{instrument, trace};

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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NewRecipe {
    #[schemars{description="The name of the recipe"}]
    pub name: String,

    #[schemars{description="The list of ingredients, for example '3 tablespoons flour'"}]
    pub ingredients: Vec<String>,

    #[schemars{description="The list of steps"}]
    pub steps: Vec<String>,
}

#[derive(Clone)]
pub struct Mealie {
    env: Env,
}

fn mark_named_items_as_checked(
    items: impl IntoIterator<Item = ShoppingListItem>,
    names: &[String],
) -> Vec<ShoppingListItem> {
    let names: HashSet<&String> = HashSet::from_iter(names);
    items
        .into_iter()
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

#[derive(Debug, Deserialize, Serialize)]
pub struct FilteredRecipe {
    pub slug: String,
    pub name: String,
    pub description: String,
}

impl From<Recipe> for FilteredRecipe {
    fn from(value: Recipe) -> Self {
        FilteredRecipe {
            name: value.name,
            slug: value.slug,
            description: value.description,
        }
    }
}

#[tool(tool_box)]
impl Mealie {
    pub fn new(env: Env) -> Mealie {
        Mealie { env }
    }

    #[instrument(skip(self))]
    #[tool(description = "Create a new recipe")]
    pub async fn add_recipe(&self, #[tool(aggr)] new_recipe : NewRecipe) -> Result<CallToolResult, Error> {
        trace!("Adding a new recipe");
        match self.add_recipe_to_mealie(new_recipe).await {
            Ok(recipe) => Ok(CallToolResult::success(
                Content::json(recipe)?.into_contents(),
            )),
            Err(err) => Err(Error::internal_error(
                format!("Failed to create the recipe: {}", err),
                None,
            )),
        }
    }

    // Using the mealie api requires some data shuffling
    async fn add_recipe_to_mealie(&self, NewRecipe{name, ingredients, steps}: NewRecipe) -> anyhow::Result<Recipe> {
        let slug = self.env.api_client.create_recipe_slug(&name).await?;
        if let Some(mut recipe) = self.env.api_client.get_recipe(&slug).await? {
            let steps : Vec<RecipeInstruction> = steps.into_iter().map(|s| RecipeInstruction::new().with_text(&s)).collect();
            let ingredients = ingredients.into_iter().map(|i| RecipeIngredient::new().with_note(&i)).collect::<Vec<RecipeIngredient>>();
            recipe.recipe_ingredient = Some(ingredients);
            recipe.recipe_instructions = Some(steps);
            self.env.api_client.patch_recipe(dbg!(&recipe)).await?;
            return Ok(recipe);
        }
        bail!("Failed to create")
    }

    #[instrument(skip(self))]
    #[tool(description = "Return all the existing recipes")]
    pub async fn get_recipes(&self) -> Result<CallToolResult, Error> {
        trace!("Fetching all recipes");
        let recipes: Vec<FilteredRecipe> = self
            .env
            .api_client
            .get_recipes()
            .filter_map(|x| async move { x.ok().map(|r| FilteredRecipe::from(r)) })
            .collect::<Vec<FilteredRecipe>>()
            .await;
        Ok(CallToolResult::success(
            Content::json(recipes)?.into_contents(),
        ))
    }

    #[instrument(skip(self))]
    #[tool(description = "Mark shopping list items as done")]
    pub async fn mark_as_done(
        &self,
        #[tool(aggr)] ManyItemRequest { names }: ManyItemRequest,
    ) -> Result<CallToolResult, Error> {
        trace!("Mark item as done");
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
                Content::text(format!("Marked as done")).into_contents(),
            )),
            Err(err) => Err(Error::internal_error(
                format!("failed to mark as done: {:?}", err),
                None,
            )),
        }
    }

    #[instrument(skip(self))]
    #[tool(description = "Add a new item to the shopping list")]
    pub async fn add_to_list(
        &self,
        #[tool(aggr)] ItemRequest { name }: ItemRequest,
    ) -> Result<CallToolResult, Error> {
        trace!("Add a new item to the shopping list");
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

    #[instrument(skip(self))]
    #[tool(description = "See what is in the shopping list currently")]
    pub async fn current_items(&self) -> Result<CallToolResult, Error> {
        trace!("Getting all the current items");
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
impl ServerHandler for Mealie {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Mealie server, recipes and shopping lists".into()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_item() -> impl Strategy<Value = ShoppingListItem> {
        ("[a-z]{1,5}", "[a-z]{1,5}", "[a-z]{1,5}", any::<bool>()).prop_map(
            |(id, note, shopping_list_id, checked)| ShoppingListItem {
                id,
                note,
                checked,
                shopping_list_id,
                label: None,
            },
        )
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
