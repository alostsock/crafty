use crate::Recipe;

include!(concat!(env!("OUT_DIR"), "/recipes.rs"));

#[allow(clippy::missing_panics_doc)]
pub fn recipes(player_job_level: u32) -> &'static [Recipe] {
    RECIPES.get(&player_job_level).unwrap()
}
