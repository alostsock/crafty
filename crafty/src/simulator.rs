use crate::tree::Arena;
use crate::{data, Action, CraftResult, CraftState, Player, Recipe};
use rand::{rngs::SmallRng, Rng, SeedableRng};

#[derive(Debug)]
pub struct Simulator {
    pub tree: Arena<CraftState>,
    pub iterations: u32,
    pub dead_ends_selected: u64,
    rng: SmallRng,
}

impl Simulator {
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

    pub fn new(
        recipe: &Recipe,
        player: &Player,
        iterations: u32,
        max_steps: u8,
        rng_seed: Option<u64>,
    ) -> Self {
        let (progress_factor, quality_factor) = Simulator::factors(player, recipe);
        let initial_state = CraftState::new(
            progress_factor,
            quality_factor,
            recipe.progress,
            recipe.quality,
            max_steps,
            recipe.durability,
            player.cp,
        );

        Simulator {
            tree: Arena::new(initial_state),
            iterations,
            rng: {
                if let Some(seed) = rng_seed {
                    SmallRng::seed_from_u64(seed)
                } else {
                    SmallRng::from_entropy()
                }
            },
            dead_ends_selected: 0,
        }
    }

    pub fn execute_actions(
        &mut self,
        start_index: usize,
        actions: Vec<Action>,
    ) -> Result<usize, CraftResult> {
        let mut current_index = start_index;
        for action in actions {
            let current_state = &mut self.tree.get_mut(current_index).state;

            if let Some(result) = current_state.check_result() {
                return Err(result);
            }

            if let Some(index) = current_state
                .available_moves
                .iter()
                .position(|&m| m == action)
            {
                current_state.available_moves.swap_remove(index);
            }

            let next_state = action.execute(current_state);
            let next_index = self.tree.insert(current_index, next_state);
            current_index = next_index;
        }
        Ok(current_index)
    }

    /// Calculate the UCB1 score for a node
    fn eval(&self, state: &CraftState, parent_visits: f32) -> f32 {
        let visits = state.visits as f32;
        let exploitation = state.score_sum / visits;
        let exploration = (2.0 * parent_visits.ln() / visits).sqrt();
        exploitation + exploration
    }

    /// Traverses the tree to find a good candidate node to expand.
    fn select(&self, current_index: usize) -> usize {
        let mut selected_index = current_index;
        loop {
            let selected_node = self.tree.get(selected_index);

            let expandable = !selected_node.state.available_moves.is_empty();
            let likely_terminal = selected_node.children.is_empty();
            if expandable || likely_terminal {
                break;
            }

            // select the node with the highest score
            let parent_visits = selected_node.state.visits;
            selected_index = *selected_node
                .children
                .iter()
                .max_by(|a, b| {
                    let a_reward = self.eval(&self.tree.get(**a).state, parent_visits);
                    let b_reward = self.eval(&self.tree.get(**b).state, parent_visits);
                    a_reward.partial_cmp(&b_reward).unwrap()
                })
                .unwrap();
        }
        selected_index
    }

    /// Expands the tree, then randomly selects from available moves until a
    /// terminal state is encountered. To decrease memory usage, the tree should
    /// only expand by one node per iteration unless we hit a good score, in
    /// which case the the whole path should be stored.
    fn expand_and_rollout(&mut self, initial_index: usize) -> (usize, CraftResult) {
        // expand once
        let initial_state = &mut self.tree.get_mut(initial_index).state;
        let move_count = initial_state.available_moves.len();
        // TODO: Currently there is a high chance of selecting a "dead end" due to how
        // selection works. Additional heuristics should be added to avoid these.
        if move_count == 0 {
            return (initial_index, initial_state.check_result().unwrap());
        }
        let random_index = self.rng.gen_range(0..move_count);
        let random_action = initial_state.available_moves.swap_remove(random_index);
        let expanded_state = random_action.execute(initial_state);
        let expanded_index = self.tree.insert(initial_index, expanded_state);

        // playout to a terminal state
        let mut current_state = self.tree.get(expanded_index).state.clone();
        let mut action_history: Vec<Action> = vec![];
        let result = loop {
            if let Some(result) = current_state.check_result() {
                break result;
            }
            let move_count = current_state.available_moves.len();
            let random_index = self.rng.gen_range(0..move_count);
            let random_action = current_state.available_moves.get(random_index).unwrap();
            action_history.push(*random_action);
            current_state = random_action.execute(&current_state);
        };

        // store the result if a max score was reached
        match result {
            CraftResult::Finished(score)
                if score >= 0.75 && score >= self.tree.nodes[0].state.max_score =>
            {
                let finished_index = self
                    .execute_actions(expanded_index, action_history)
                    .unwrap();
                (finished_index, result)
            }
            _ => (expanded_index, result),
        }
    }

    fn backup(&mut self, start_index: usize, target_index: usize, score: f32) {
        let mut current_index = start_index;
        loop {
            let current_node = &mut self.tree.get_mut(current_index);
            current_node.state.visits += 1.0;
            current_node.state.score_sum += score;
            current_node.state.max_score = current_node.state.max_score.max(score);

            if current_index == target_index {
                break;
            }

            current_index = current_node.parent.unwrap();
        }
    }

    pub fn search(&mut self, start_index: usize) -> &mut Self {
        for _ in 0..self.iterations {
            let selected_index = self.select(start_index);
            let (end_index, result) = self.expand_and_rollout(selected_index);

            if selected_index == end_index {
                self.dead_ends_selected += 1;
            }

            let score = match result {
                CraftResult::Finished(s) => s,
                CraftResult::Failed => 0.0,
            };
            self.backup(end_index, start_index, score);
        }
        self
    }

    pub fn solution(&self) -> (Vec<Action>, CraftState) {
        let mut actions = vec![];
        let mut node = self.tree.get(0);
        while !node.children.is_empty() {
            let next_index: usize = *node
                .children
                .iter()
                .max_by(|a, b| {
                    let a_score = self.tree.get(**a).state.max_score;
                    let b_score = self.tree.get(**b).state.max_score;
                    a_score.partial_cmp(&b_score).unwrap()
                })
                .unwrap();
            node = self.tree.get(next_index);
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

    fn setup_sim_1() -> Simulator {
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
        Simulator::new(&recipe, &player, 10_000, 15, Some(0))
    }

    fn setup_sim_2() -> Simulator {
        let recipe = Recipe {
            recipe_level: 580,
            job_level: 90,
            stars: 2,
            progress: 3900,
            quality: 10920,
            durability: 70,
            progress_div: 130,
            progress_mod: 80,
            quality_div: 115,
            quality_mod: 70,
            is_expert: false,
            conditions_flag: 15,
        };
        let player = Player::new(90, 3290, 3541, 649);
        Simulator::new(&recipe, &player, 10_000, 25, Some(123))
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
            .execute_actions(0, actions)
            .expect("craft finished unexpectedly");
        let result = &sim.tree.get(result_node).state;
        assert_eq!(result.progress, progress);
        assert_eq!(result.quality, quality);
        assert_eq!(result.durability, durability);
        assert_eq!(result.cp, cp);
        result
    }

    #[test]
    fn basic_actions() {
        let actions = vec![BasicTouch, BasicSynthesis, MastersMend];
        assert_craft(&mut setup_sim_1(), actions, 276, 262, 80, 469);
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
        assert_craft(&mut setup_sim_1(), actions, 0, 2828, 30, 425);
    }

    #[test]
    fn with_buffs_1() {
        let actions = vec![Reflect, Manipulation, PreparatoryTouch, WasteNotII];
        assert_craft(&mut setup_sim_1(), actions, 0, 890, 60, 335);
    }

    #[test]
    fn with_buffs_2() {
        let actions = vec![MuscleMemory, GreatStrides, PrudentTouch, DelicateSynthesis];
        assert_craft(&mut setup_sim_1(), actions, 1150, 812, 55, 480);
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
        assert_craft(&mut setup_sim_1(), actions, 1150, 1925, 80, 163);
    }

    #[test]
    fn rotation_should_not_panic_1() {
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
        let mut sim = setup_sim_1();
        sim.execute_actions(0, actions).unwrap();
    }

    #[test]
    fn rotation_should_not_panic_2() {
        let actions = vec![
            MuscleMemory,
            Manipulation,
            Veneration,
            WasteNotII,
            Groundwork,
            Groundwork,
            StandardTouch,
            Innovation,
            PreparatoryTouch,
            PreparatoryTouch,
            PreparatoryTouch,
            PreparatoryTouch,
            GreatStrides,
            Innovation,
            PreparatoryTouch,
            TrainedFinesse,
            GreatStrides,
            ByregotsBlessing,
        ];
        let mut sim = setup_sim_2();
        sim.execute_actions(0, actions).unwrap();
    }

    #[test]
    fn search_should_not_panic() {
        let mut sim = setup_sim_2();
        sim.search(0).solution();
    }
}
