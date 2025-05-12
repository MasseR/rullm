use std::{error::Error, pin::Pin};

use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::env::Env;

type DynSendSyncError = Box<dyn Error + Send + Sync + 'static>;

// API types

// All the GET responses are paginated
#[derive(Deserialize, Debug)]
pub struct Page<T> {
  total_pages: i32,
  items: Vec<T>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingList{
  pub name: String,
  pub id: String,
  // There are plenty of labels that could be valuable
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShoppingListItem {
  pub id: String,
  pub note: String,
  pub display: String,
  pub checked: bool,
}

// Fetch a single pageful of shopping lists
async fn fetch_shopping_list(env: &Env, page: i32) -> Result<Page<ShoppingList>, DynSendSyncError> {
  let url = format!("{}/households/shopping/lists", env.conf.base_url);
  let resp = env.api_client.get(url)
    .query(&[("page", page)])
    .bearer_auth(&env.conf.api_key)
    .send()
    .await?;
  let resp = resp.error_for_status()?;
  Ok(resp.json().await?)
}

pub fn get_all_shopping_lists(env : &Env) -> impl Stream<Item = Result<ShoppingList, DynSendSyncError>> {
  futures::stream::unfold(Some(1), move |opage| async move {
    match opage {
      None => None,
      Some(page) => {
        match fetch_shopping_list(&env, page).await {
          Ok(res_page) => {
            let stream = futures::stream::iter(res_page.items.into_iter().map(Ok));
            let cont = if page < res_page.total_pages { Some(page + 1) } else { None };
            Some((Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<ShoppingList, DynSendSyncError>>>>, cont))
          }
          Err(err) => {
            let stream = futures::stream::once(async { Err(err) });
            Some((Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<ShoppingList, DynSendSyncError>>>>, None))
          }
        }
      }
    }
  }).flatten()
}


pub async fn fetch_shopping_list_item(env: &Env, list_id: &str, page: i32) -> Result<Page<ShoppingListItem>, DynSendSyncError> {
  let url = format!("{}/households/shopping/items", env.conf.base_url);
  let resp = env.api_client.get(url)
    .query(&[("page", page.to_string()), ("queryFilter", format!("shoppingListId={list_id}"))])
    .bearer_auth(&env.conf.api_key)
    .send()
    .await?;
  let resp = resp.error_for_status()?;
  Ok(resp.json().await?)
}

pub fn get_all_shopping_list_items(env : &Env, list_id: &str) -> impl Stream<Item = Result<ShoppingListItem, DynSendSyncError>> {
  futures::stream::unfold(Some(1), move |opage| async move {
    match opage {
      None => None,
      Some(page) => {
        match fetch_shopping_list_item(&env, &list_id, page).await {
          Ok(res_page) => {
            let stream = futures::stream::iter(res_page.items.into_iter().map(Ok));
            let cont = if page < res_page.total_pages { Some(page + 1) } else { None };
            Some((Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<ShoppingListItem, DynSendSyncError>> + Send>>, cont))
          }
          Err(err) => {
            let stream = futures::stream::once(async { Err(err) });
            Some((Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<ShoppingListItem, DynSendSyncError>> + Send>>, None))
          }
        }
      }
    }
  }).flatten()
}

// Internal API entity
#[derive(Serialize,Deserialize,Debug)]
#[serde(rename_all = "camelCase")]
struct PostShoppingListItem {
  quantity: f32,
  note: String,
  display: String,
  shopping_list_id: String,
}

pub async fn new_shopping_list_item(env: &Env, list_id: &str, name: &str) -> Result<(), DynSendSyncError> {
  let url = format!("{}/households/shopping/items", env.conf.base_url);
  let item = PostShoppingListItem {
    quantity: 1.0,
    note: String::from(name),
    display: String::from(name),
    shopping_list_id: String::from(list_id),
  };
  let resp = env.api_client.post(url)
    .bearer_auth(&env.conf.api_key)
    .json(&item)
    .send()
    .await?;
  resp.error_for_status()?;
  Ok(())
}
