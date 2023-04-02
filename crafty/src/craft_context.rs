use crate::{data, Action, ActionSet, Player, Recipe};

#[derive(Debug, Clone)]
pub struct CraftContext {
    /// Multiply by synthesis action efficiency for increase in progress
    pub progress_factor: f32,
    /// Multiply by touch action efficiency for increase in quality
    pub quality_factor: f32,
    pub step_max: u8,
    pub progress_target: u32,
    pub quality_target: u32,
    pub durability_max: i8,
    pub cp_max: u32,
    pub action_pool: ActionSet,
}

impl CraftContext {
    #[allow(clippy::cast_precision_loss)]
    fn factors(player: &Player, recipe: &Recipe) -> (f32, f32) {
        // https://github.com/ffxiv-teamcraft/simulator/blob/72f4a6037baa3cd7cd78dfe34207283b824881a2/src/model/actions/crafting-action.ts#L176

        let progress_div = recipe.progress_div as f32;
        let mut progress_factor: f32 = (player.craftsmanship * 10) as f32 / progress_div + 2.0;

        let quality_div = recipe.quality_div as f32;
        let mut quality_factor: f32 = (player.control * 10) as f32 / quality_div + 35.0;

        if let Some(&base_recipe_level) = data::base_recipe_level(player.job_level) {
            if base_recipe_level <= recipe.recipe_level {
                progress_factor *= recipe.progress_mod as f32 / 100.0;
                quality_factor *= recipe.quality_mod as f32 / 100.0;
            }
        }

        (progress_factor.floor(), quality_factor.floor())
    }

    pub fn new(player: &Player, recipe: &Recipe, max_steps: u8) -> Self {
        let (progress_factor, quality_factor) = Self::factors(player, recipe);
        Self {
            progress_factor,
            quality_factor,
            step_max: max_steps,
            progress_target: recipe.progress,
            quality_target: recipe.quality,
            durability_max: recipe.durability,
            cp_max: player.cp,
            action_pool: ActionSet::from_vec(&Action::ACTIONS.to_vec()),
        }
    }
}
