use recipe::Recipe;

include!(concat!(env!("OUT_DIR"), "/recipes.rs"));

pub fn recipes_by_level(recipe_job_level: u32) -> &'static [Recipe] {
    RECIPES.get(&recipe_job_level).unwrap()
}
