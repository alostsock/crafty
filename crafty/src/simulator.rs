use crate::action::Action;
use crate::craft_state::{CraftResult, CraftState};
use crate::player::Player;
use crate::tree::{Arena, Node};
use recipe::Recipe;

include!(concat!(env!("OUT_DIR"), "/levels.rs"));

#[derive(Debug)]
pub struct Simulator {
    pub tree: Arena<CraftState>,
}

impl Simulator {
    fn calculate_factors(player: &Player, recipe: &Recipe) -> (f32, f32) {
        // https://github.com/ffxiv-teamcraft/simulator/blob/72f4a6037baa3cd7cd78dfe34207283b824881a2/src/model/actions/crafting-action.ts#L176

        let progress_div = recipe.progress_div as f32;
        let mut progress_factor: f32 = (player.craftsmanship * 10) as f32 / progress_div + 2.0;

        let quality_div = recipe.quality_div as f32;
        let mut quality_factor: f32 = (player.control * 10) as f32 / quality_div + 35.0;

        if let Some(&player_recipe_level) = LEVELS.get(&player.job_level) {
            if player_recipe_level <= recipe.recipe_level {
                progress_factor *= recipe.progress_mod as f32 / 100.0;
                quality_factor *= recipe.quality_mod as f32 / 100.0;
            }
        }

        (progress_factor.floor(), quality_factor.floor())
    }

    pub fn new(recipe: &Recipe, player: &Player) -> Self {
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

    pub fn node(&self, node: usize) -> &Node<CraftState> {
        self.tree.get(node).unwrap()
    }

    pub fn node_mut(&mut self, node: usize) -> &mut Node<CraftState> {
        self.tree.get_mut(node).unwrap()
    }

    pub fn execute_actions(
        &mut self,
        start_index: usize,
        actions: Vec<Action>,
        max_steps: u8,
    ) -> Result<usize, CraftResult> {
        let mut current_index = start_index;
        for action in actions {
            let current_state = &mut self.node_mut(current_index).state;

            if let Some(result) = current_state.check_result(max_steps) {
                return Err(result);
            }

            let new_state = current_state.execute_action(action);
            let next_index = self.tree.insert(current_index, new_state);
            current_index = next_index;
        }
        Ok(current_index)
    }

    /// Calculate the UCB1 score for a node
    fn eval(&self, state: &CraftState, parent_visits: f64) -> f64 {
        let visits = state.visits as f64;
        let exploitation = state.score_sum / visits;
        let exploration = (2.0 * parent_visits.ln() / visits).sqrt();
        exploitation + exploration
    }

    /// Traverses the tree to find a good candidate node to expand
    fn select(&self, current_index: usize) -> usize {
        let mut selected_index = current_index;
        loop {
            let selected_node = self.node(selected_index);
            let parent_visits = selected_node.state.visits;
            // return this node if there are still available moves, or if there are no children
            if !selected_node.state.available_moves.is_empty() || selected_node.children.is_empty()
            {
                break;
            }
            // select the node with the highest UCB1 score
            selected_index = *selected_node
                .children
                .iter()
                .max_by(|a, b| {
                    let a_reward = self.eval(&self.node(**a).state, parent_visits);
                    let b_reward = self.eval(&self.node(**b).state, parent_visits);
                    a_reward.partial_cmp(&b_reward).unwrap()
                })
                .unwrap();
        }
        selected_index
    }

    /// Randomly select from available moves until we hit a terminal state
    fn expand(&mut self, start_index: usize, max_steps: u8) -> (usize, CraftResult) {
        let mut current_index = start_index;
        loop {
            let current_state = &mut self.node_mut(current_index).state;

            if let Some(result) = current_state.check_result(max_steps) {
                return (current_index, result);
            }

            let new_state = current_state.execute_random_action();
            let next_node = self.tree.insert(current_index, new_state);
            current_index = next_node;
        }
    }

    fn backpropagate(&mut self, start_index: usize, target_index: usize, score: f64) {
        let mut current_index = start_index;
        loop {
            let current_node = &mut self.node_mut(current_index);
            current_node.state.visits += 1.0;
            current_node.state.score_sum += score;
            current_node.state.max_score = current_node.state.max_score.max(score);

            if current_index == target_index {
                break;
            }

            current_index = current_node.parent.unwrap();
        }
    }

    pub fn search(&mut self, start_index: usize, max_iterations: usize, max_steps: u8) {
        for _ in 0..max_iterations {
            // select
            let selected_index = self.select(start_index);
            // expand/simulate
            let (end_index, result) = self.expand(selected_index, max_steps);
            let score = match result {
                CraftResult::Finished(s) => s,
                CraftResult::Failed => 0.0,
            };
            // backup
            self.backpropagate(end_index, start_index, score);
        }
    }

    pub fn solution(&self) -> (Vec<Action>, CraftState) {
        let mut actions = vec![];
        let mut node = self.node(0);
        while !node.children.is_empty() {
            let next_index: usize = *node
                .children
                .iter()
                .max_by(|a, b| {
                    let a_score = self.node(**a).state.max_score;
                    let b_score = self.node(**b).state.max_score;
                    a_score.partial_cmp(&b_score).unwrap()
                })
                .unwrap();
            node = self.node(next_index);
            if node.state.action.is_some() {
                actions.push(node.state.action.unwrap());
            }
        }

        (actions, node.state.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::craft_state::CraftState;

    use super::{Action, Player, Recipe, Simulator};
    use Action::*;

    fn setup_sim() -> Simulator {
        let recipe = Recipe {
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
        Simulator::new(&recipe, &player)
    }

    fn assert_craft(
        sim: &mut Simulator,
        actions: Vec<Action>,
        progress: u32,
        quality: u32,
        durability: u32,
        cp: u32,
    ) -> &CraftState {
        let result_node = sim
            .execute_actions(0, actions, 30)
            .expect("craft finished unexpectedly");
        let result = &sim.node(result_node).state;
        assert_eq!(result.progress, progress);
        assert_eq!(result.quality, quality);
        assert_eq!(result.durability, durability);
        assert_eq!(result.cp, cp);
        result
    }

    #[test]
    fn basic_actions() {
        let actions = vec![BasicTouch, BasicSynthesis, MastersMend];
        assert_craft(&mut setup_sim(), actions, 276, 262, 80, 469);
    }

    #[test]
    fn basic_touch_combo() {
        let actions = vec![
            Innovation,
            BasicTouch,
            StandardTouch,
            AdvancedTouch,
            StandardTouch,
            AdvancedTouch,
        ];
        assert_craft(&mut setup_sim(), actions, 0, 2828, 30, 425);
    }

    #[test]
    fn with_buffs_1() {
        let actions = vec![Reflect, Manipulation, PreparatoryTouch, WasteNotII];
        assert_craft(&mut setup_sim(), actions, 0, 890, 60, 335);
    }

    #[test]
    fn with_buffs_2() {
        let actions = vec![MuscleMemory, GreatStrides, PrudentTouch, DelicateSynthesis];
        assert_craft(&mut setup_sim(), actions, 1150, 812, 55, 480);
    }

    #[test]
    fn with_buffs_3() {
        let actions = vec![
            MuscleMemory,
            Manipulation,
            MastersMend,
            WasteNotII,
            Innovation,
            DelicateSynthesis,
            BasicTouch,
            GreatStrides,
            ByregotsBlessing,
        ];
        assert_craft(&mut setup_sim(), actions, 1150, 1925, 80, 163);
    }

    #[test]
    fn should_not_panic() {
        let actions = vec![
            Reflect,
            Manipulation,
            PreparatoryTouch,
            WasteNotII,
            PreparatoryTouch,
            Innovation,
            PreparatoryTouch,
            PreparatoryTouch,
            GreatStrides,
            ByregotsBlessing,
            Veneration,
            Groundwork,
            Groundwork,
            Groundwork,
        ];
        let mut sim = setup_sim();
        sim.execute_actions(0, actions, 30)
            .expect("craft finished unexpectedly");
    }
}
