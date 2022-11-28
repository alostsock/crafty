use crate::Recipe;

include!(concat!(env!("OUT_DIR"), "/recipes.rs"));
include!(concat!(env!("OUT_DIR"), "/levels.rs"));

#[allow(clippy::missing_panics_doc)]
pub fn recipes(player_job_level: u32) -> &'static [Recipe] {
    RECIPES.get(&player_job_level).unwrap()
}

pub fn base_recipe_level(player_job_level: u32) -> Option<&'static u32> {
    LEVELS.get(&player_job_level)
}
