use futures::StreamExt as _;
use mcp_mealie_server::{
    conf::Conf,
    env::Env,
    mealie::{RecipeIngredient, RecipeInstruction, ShoppingList},
};
use tracing::{debug, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let conf = Conf::parse().await?;
    let env = Env::build(conf).await?;
    let slug = env.api_client.create_recipe_slug("Pinaattiletut").await?;
    // let slug = "test-recipe-1".to_string();
    if let Some(mut recipe) = env.api_client.get_recipe(&slug).await? {
        let steps = vec![
            RecipeInstruction::new().with_text("Sekoita kananmunat ja maito kulhossa"),
            RecipeInstruction::new().with_text("Lisää vehnäjauhot ja suola vähitellen koko ajan sekoittaen, jotta vältät paakkuuntumisen"),
            RecipeInstruction::new().with_text("Lisää muskottipähkinä ja sulatettu pinaatti taikinaan. Sekoita hyvin."),
            RecipeInstruction::new().with_text("Anna taikinan levätä noin 30 minuuttia, jotta jauhot turpoavat"),
            RecipeInstruction::new().with_text("Kuumenna voi paistinpannussa keskilämmöllä"),
            RecipeInstruction::new().with_text("Kaada taikinaa pannulle sopiva määrä kerrallaan ja paista letut molemmin puolin kullanruskeiksi"),
            RecipeInstruction::new().with_text("Tarjoile pinaattiletut valitsemiesi lisukkeiden kanssa, esimerkiksi puolukkahillon tai smetanan kera"),
        ];
        let ingredients = vec![
            RecipeIngredient::new().with_note("2 kananmunaa"),
            RecipeIngredient::new().with_note("5 dl maitoa"),
            RecipeIngredient::new().with_note("2.5 dl vehnäjauhoja"),
            RecipeIngredient::new().with_note("1 tl suolaa"),
            RecipeIngredient::new().with_note("ripaus muskottipähkinää (valinnainen)"),
            RecipeIngredient::new().with_note("150 g pakastepinaattia (sulatettuna ja nesteestä puristettuna)"),
            RecipeIngredient::new().with_note("2 rkl voita paistamiseen"),
        ];
        recipe.recipe_ingredient = Some(ingredients);
        recipe.recipe_instructions = Some(steps);
        info!(recipe = format!("{:?}", recipe), "patching");
        env.api_client.patch_recipe(dbg!(&recipe)).await?;
    }


    env.api_client.get_recipes().for_each(|x| async move {
        println!("{:?}", x);
    }).await;
    Ok(())
}
