use crafty_models::RecipeVariant;

include!(concat!(env!("OUT_DIR"), "/recipes.rs"));

pub fn recipes_by_level(recipe_job_level: u32) -> &'static [RecipeVariant] {
    RECIPES.get(&recipe_job_level).unwrap()
}
