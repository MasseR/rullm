use std::{error::Error, pin::Pin};

use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::conf::Conf;

#[derive(Clone)]
pub struct MealieClient {
    client: Client,
    // Not convinced it makes sense to move the ownership
    // of conf here. But right now this is the only place that uses config so...
    conf: Conf,
}

impl MealieClient {
    pub fn build(conf: Conf) -> anyhow::Result<MealieClient> {
        let client = Client::new();
        Ok(MealieClient{ client, conf })
    }

    pub async fn new_shopping_list_item(
        &self,
        list_id: &str,
        name: &str,
    ) -> anyhow::Result<()> {
        let url = format!("{}/households/shopping/items", self.conf.base_url);
        let item = PostShoppingListItem {
            quantity: 1.0,
            note: String::from(name),
            display: String::from(name),
            shopping_list_id: String::from(list_id),
        };
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.conf.api_key)
            .json(&item)
            .send()
            .await?;
        resp.error_for_status()?;
        Ok(())
    }

    // Fetch a single pageful of shopping lists
    async fn fetch_shopping_list(&self, page: i32) -> Result<Page<ShoppingList>, DynSendSyncError> {
        let url = format!("{}/households/shopping/lists", self.conf.base_url);
        let resp = self
            .client
            .get(url)
            .query(&[("page", page)])
            .bearer_auth(&self.conf.api_key)
            .send()
            .await?;
        let resp = resp.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub fn get_all_shopping_lists(
        &self,
    ) -> impl Stream<Item = Result<ShoppingList, DynSendSyncError>> {
        futures::stream::unfold(Some(1), move |opage| async move {
            match opage {
                None => None,
                Some(page) => match self.fetch_shopping_list(page).await {
                    Ok(res_page) => {
                        let stream = futures::stream::iter(res_page.items.into_iter().map(Ok));
                        let cont = if page < res_page.total_pages {
                            Some(page + 1)
                        } else {
                            None
                        };
                        Some((
                            Box::pin(stream)
                                as Pin<Box<dyn Stream<Item = Result<ShoppingList, DynSendSyncError>>>>,
                            cont,
                        ))
                    }
                    Err(err) => {
                        let stream = futures::stream::once(async { Err(err) });
                        Some((
                            Box::pin(stream)
                                as Pin<Box<dyn Stream<Item = Result<ShoppingList, DynSendSyncError>>>>,
                            None,
                        ))
                    }
                },
            }
        })
        .flatten()
    }

    pub async fn fetch_shopping_list_item(
        &self,
        list_id: &str,
        page: i32,
    ) -> Result<Page<ShoppingListItem>, DynSendSyncError> {
        let url = format!("{}/households/shopping/items", self.conf.base_url);
        let resp = self
            .client
            .get(url)
            .query(&[
                ("page", page.to_string()),
                ("queryFilter", format!("shoppingListId={list_id}")),
            ])
            .bearer_auth(&self.conf.api_key)
            .send()
            .await?;
        let resp = resp.error_for_status()?;
        Ok(resp.json().await?)
    }

    pub fn get_all_shopping_list_items(
        &self,
        list_id: &str,
    ) -> impl Stream<Item = Result<ShoppingListItem, DynSendSyncError>> {
        futures::stream::unfold(Some(1), move |opage| async move {
            match opage {
                None => None,
                Some(page) => match self.fetch_shopping_list_item(&list_id, page).await {
                    Ok(res_page) => {
                        let stream = futures::stream::iter(res_page.items.into_iter().map(Ok));
                        let cont = if page < res_page.total_pages {
                            Some(page + 1)
                        } else {
                            None
                        };
                        Some((
                            Box::pin(stream)
                                as Pin<
                                    Box<
                                        dyn Stream<Item = Result<ShoppingListItem, DynSendSyncError>>
                                            + Send,
                                    >,
                                >,
                            cont,
                        ))
                    }
                    Err(err) => {
                        let stream = futures::stream::once(async { Err(err) });
                        Some((
                            Box::pin(stream)
                                as Pin<
                                    Box<
                                        dyn Stream<Item = Result<ShoppingListItem, DynSendSyncError>>
                                            + Send,
                                    >,
                                >,
                            None,
                        ))
                    }
                },
            }
        })
        .flatten()
    }
}

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
pub struct ShoppingList {
    pub name: String,
    pub id: String,
    // There are plenty of labels that could be valuable
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Label {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShoppingListItem {
    pub id: String,
    pub note: String,
    pub checked: bool,
    pub label: Option<Label>
}



// Internal API entity
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PostShoppingListItem {
    quantity: f32,
    note: String,
    display: String,
    shopping_list_id: String,
}

