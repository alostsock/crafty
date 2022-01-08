use crafty_models::RecipeVariant;
use std::fmt;

include!(concat!(env!("OUT_DIR"), "/levels.rs"));

pub struct Player {
    pub job_level: u32,
    pub craftsmanship: u32,
    pub control: u32,
    pub cp: u32,
    // multiply by Synthesis action efficiency to get increase in progress
    pub progress_factor: f64,
    // multiply by Touch action efficiency to get increase in quality
    pub quality_factor: f64,
}

impl Player {
    pub fn new(
        job_level: u32,
        craftsmanship: u32,
        control: u32,
        cp: u32,
        recipe: &RecipeVariant,
    ) -> Self {
        let (progress_factor, quality_factor) =
            get_factors_for_recipe(job_level, craftsmanship, control, recipe);
        Player {
            job_level,
            craftsmanship,
            control,
            cp,
            progress_factor,
            quality_factor,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "lv{:>2} / {} craftsmanship / {} control / {} cp",
            self.job_level, self.craftsmanship, self.control, self.cp
        )
    }
}

fn get_factors_for_recipe(
    job_level: u32,
    craftsmanship: u32,
    control: u32,
    recipe: &RecipeVariant,
) -> (f64, f64) {
    // https://github.com/ffxiv-teamcraft/simulator/blob/72f4a6037baa3cd7cd78dfe34207283b824881a2/src/model/actions/crafting-action.ts#L176
    let player_recipe_level = LEVELS.get(&job_level).map(|rlvl| rlvl.to_owned());

    let progress_div = (recipe.progress_div + 2) as f64;
    let quality_div = (recipe.quality_div + 35) as f64;

    let mut progress_factor: f64 = (craftsmanship * 10) as f64 / progress_div;
    let mut quality_factor: f64 = (control * 10) as f64 / quality_div;

    if player_recipe_level.is_some() && player_recipe_level.unwrap() <= recipe.recipe_level {
        progress_factor *= recipe.progress_mod as f64 / 100.0;
        quality_factor *= recipe.quality_mod as f64 / 100.0;
    }

    (progress_factor, quality_factor)
}
