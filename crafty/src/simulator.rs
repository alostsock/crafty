use crate::action::Action;
use crate::craft_state::CraftState;
use crate::player::Player;
use crate::tree::Arena;
use crafty_models::RecipeVariant;

include!(concat!(env!("OUT_DIR"), "/levels.rs"));

pub struct Simulator {
    tree: Arena<CraftState>,
}

impl Simulator {
    fn calculate_factors(player: &Player, recipe: &RecipeVariant) -> (f64, f64) {
        // https://github.com/ffxiv-teamcraft/simulator/blob/72f4a6037baa3cd7cd78dfe34207283b824881a2/src/model/actions/crafting-action.ts#L176

        let progress_div = recipe.progress_div as f64;
        let mut progress_factor: f64 = (player.craftsmanship * 10) as f64 / progress_div + 2.0;

        let quality_div = recipe.quality_div as f64;
        let mut quality_factor: f64 = (player.control * 10) as f64 / quality_div + 35.0;

        if let Some(&player_recipe_level) = LEVELS.get(&player.job_level) {
            if player_recipe_level <= recipe.recipe_level {
                progress_factor *= recipe.progress_mod as f64 / 100.0;
                quality_factor *= recipe.quality_mod as f64 / 100.0;
            }
        }

        (progress_factor, quality_factor)
    }

    pub fn new(recipe: &RecipeVariant, player: &Player) -> Self {
        let (progress_factor, quality_factor) = Simulator::calculate_factors(player, recipe);
        let initial_state = CraftState::new(
            progress_factor,
            quality_factor,
            recipe.progress,
            recipe.quality,
            recipe.durability,
            player.cp,
        );

        Simulator {
            tree: Arena::new(initial_state),
        }
    }

    pub fn execute_actions(&mut self, node: usize, actions: Vec<Action>) -> usize {
        let mut current_node = node;
        for action in actions {
            let current_state = &mut self.tree.get_mut(current_node).unwrap().state;
            if let Some(new_state) = current_state.execute_action(action) {
                let next_node = self.tree.insert(current_node, new_state).unwrap();
                current_node = next_node;
            } else {
                break;
            }
        }
        current_node
    }

    // expand a node to the end, and return the final node's index
    pub fn expand(&mut self, node: usize) -> usize {
        let mut current_node = node;
        loop {
            let current_state = &mut self.tree.get_mut(current_node).unwrap().state;

            if let Some(new_state) = current_state.execute_random_action() {
                let next_node = self.tree.insert(current_node, new_state).unwrap();
                current_node = next_node;
            } else {
                break;
            }
        }
        current_node
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, Player, RecipeVariant, Simulator};

    fn setup() -> (RecipeVariant, Player, Simulator) {
        let recipe = RecipeVariant {
            recipe_level: 560,
            job_level: 90,
            stars: 0,
            progress: 3500,
            quality: 7200,
            durability: 80,
            progress_div: 130,
            progress_mod: 90,
            quality_div: 115,
            quality_mod: 80,
            is_expert: false,
            conditions_flag: 15,
        };
        let player = Player::new(90, 3304, 3374, 575);
        let sim = Simulator::new(&recipe, &player);
        (recipe, player, sim)
    }

    #[test]
    fn basic_actions() {
        let (_recipe, _player, mut sim) = setup();
        let result_node = sim.execute_actions(0, vec![Action::BasicTouch, Action::BasicSynthesis]);
        let result = &sim.tree.get(result_node).unwrap().state;
        assert_eq!(result.progress, 276);
        assert_eq!(result.quality, 262);
        assert_eq!(result.durability, 60);
        assert_eq!(result.cp, 557);
    }
}
