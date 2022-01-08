mod action;
mod player;
mod tree;
mod validators;

use crate::action::{Action, ACTIONS};
pub use crate::player::Player;
use crate::tree::Arena;
use anyhow::Result;
use crafty_models::RecipeVariant;
use rand::seq::SliceRandom;

struct CraftState<'a> {
    player: &'a Player,
    recipe: &'a RecipeVariant,

    // the action that led to this state
    action: Option<Action>,
    // the probability that this state occurs
    // (i.e. action chance * condition chance)
    probability: f64,
    // can have fractional wins/playouts,
    // based on the weighted probability of its children
    wins: f64,
    playouts: f64,
    possible_moves: Vec<Action>,

    step: u32,
    progress: u32,
    quality: u32,
    durability: u32,
    cp_remaining: u32,
    // buffs: vec,
}

impl<'a> CraftState<'a> {
    fn new(player: &'a Player, recipe: &'a RecipeVariant) -> Self {
        CraftState {
            player,
            recipe,
            action: None,
            probability: 1f64,
            wins: 0f64,
            playouts: 0f64,
            possible_moves: vec![],
            step: 1,
            progress: 0,
            quality: 0,
            durability: recipe.durability,
            cp_remaining: player.cp,
        }
    }
}

#[derive(Debug)]
pub struct Simulator<'a> {
    tree: Arena<CraftState<'a>>,
}

impl<'a> Simulator<'a> {
    pub fn new(recipe: &'a RecipeVariant, player: &'a Player) -> Result<Self> {
        let mut initial_state = CraftState::new(player, recipe);
        initial_state.possible_moves = ACTIONS.to_vec();
        let sim = Simulator {
            tree: Arena::new(initial_state),
        };
        Ok(sim)
    }

    // expand a node to the end, and return the final node's index
    pub fn expand(&self, node: usize) -> usize {
        let current_node = self.tree.get_mut(node).unwrap();
        let action = current_node
            .value
            .possible_moves
            .choose(&mut rand::thread_rng())
            .unwrap()
            .clone();
        let next_node = self
    }
}

#[cfg(test)]
mod tests {
    use crate::{Player, Simulator};
    use crafty_models::RecipeVariant;

    fn setup() -> (RecipeVariant, Player) {
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
        let player = Player::new(90, 3286, 3394, 586, &recipe);
        (recipe, player)
    }

    #[test]
    fn basic_actions_work() {
        let (recipe, player) = setup();
        let sim = Simulator::new(&recipe, &player).unwrap();
    }
}
