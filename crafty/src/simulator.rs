use crate::{data, tree::Arena, Action, CraftResult, CraftState, Player, Recipe};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
pub struct SearchOptions {
    /// Number of simulations to run
    pub iterations: u32,
    /// Maximum number of steps allowed for the craft
    pub max_steps: u8,
    /// Numerical seed to use for RNG. Randomly picked if None
    pub rng_seed: Option<u64>,
    /// A memory optimization option that specifies the minimum score a craft has to reach for
    /// action history to be stored. Only stores ~100% HQ states if None.
    pub score_storage_threshold: Option<f32>,
    pub max_score_weighting_constant: Option<f32>,
    pub exploration_constant: Option<f32>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            iterations: 10_000,
            max_steps: 15,
            rng_seed: Some(SmallRng::from_entropy().gen()),
            score_storage_threshold: Some(1.0),
            max_score_weighting_constant: Some(0.1),
            exploration_constant: Some(1.5),
        }
    }
}

#[derive(Debug)]
pub struct Simulator {
    pub tree: Arena<CraftState>,
    iterations: u32,
    /// Amount of "dead ends" encountered. This means a node was selected, but there weren't any
    /// available moves.
    pub dead_ends_selected: u64,
    pub rng_seed: u64,
    rng: SmallRng,
    score_storage_threshold: f32,
    /// The higher the weight, the more a node's potential max score is valued over its average
    /// score. A weight of 1.0 means only max scores will be used; 0.0 means only average scores
    /// will be used.
    max_score_weighting_constant: f32,
    /// Higher values prioritize exploring less promising nodes.
    exploration_constant: f32,
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

    pub fn new(recipe: &Recipe, player: &Player, options: SearchOptions) -> Self {
        let (progress_factor, quality_factor) = Self::factors(player, recipe);

        let initial_state = CraftState::new(
            progress_factor,
            quality_factor,
            recipe.progress,
            recipe.quality,
            options.max_steps,
            recipe.durability,
            player.cp,
        );

        Self::from_state(&initial_state, options)
    }

    pub fn from_state(state: &CraftState, options: SearchOptions) -> Self {
        let defaults = SearchOptions::default();
        let rng_seed = options.rng_seed.or(defaults.rng_seed).unwrap();

        Self {
            tree: Arena::new(state.clone()),
            iterations: options.iterations,
            dead_ends_selected: 0,
            rng_seed,
            rng: SmallRng::seed_from_u64(rng_seed),
            score_storage_threshold: options
                .score_storage_threshold
                .or(defaults.score_storage_threshold)
                .unwrap(),
            max_score_weighting_constant: options
                .max_score_weighting_constant
                .or(defaults.max_score_weighting_constant)
                .unwrap(),
            exploration_constant: options
                .exploration_constant
                .or(defaults.exploration_constant)
                .unwrap(),
        }
    }

    pub fn execute_actions(
        &mut self,
        start_index: usize,
        actions: Vec<Action>,
    ) -> (usize, Option<CraftResult>) {
        let mut current_index = start_index;
        for action in actions {
            let current_state = &mut self.tree.get_mut(current_index).state;

            if let Some(result) = current_state.check_result() {
                return (current_index, Some(result));
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
        // check state after performing last action
        let current_state = &self.tree.get_mut(current_index).state;
        (current_index, current_state.check_result())
    }

    /// Calculate the UCB1 score for a node
    fn eval(&self, state: &CraftState, parent_state: &CraftState) -> f32 {
        let w = self.max_score_weighting_constant;
        let c = self.exploration_constant;

        let visits = state.visits as f32;
        let average_score = state.score_sum / visits;

        let exploitation = (1.0 - w) * average_score + w * state.max_score;
        let exploration = (c * parent_state.visits.ln() / visits).sqrt();

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
            selected_index = *selected_node
                .children
                .iter()
                .max_by(|&a, &b| {
                    let a_score = self.eval(&self.tree.get(*a).state, &selected_node.state);
                    let b_score = self.eval(&self.tree.get(*b).state, &selected_node.state);
                    a_score.partial_cmp(&b_score).unwrap()
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
        if let Some(result) = initial_state.check_result() {
            return (initial_index, result);
        }
        let random_index = self.rng.gen_range(0..initial_state.available_moves.len());
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
            let random_index = self.rng.gen_range(0..current_state.available_moves.len());
            let random_action = current_state.available_moves.get(random_index).unwrap();
            action_history.push(*random_action);
            current_state = random_action.execute(&current_state);
        };

        // store the result if a max score was reached
        match result {
            CraftResult::Finished(score)
                if score >= self.score_storage_threshold
                    && score >= self.tree.nodes[0].state.max_score =>
            {
                let (terminal_index, _) = self.execute_actions(expanded_index, action_history);
                (terminal_index, result)
            }
            _ => (expanded_index, result),
        }
    }

    fn backpropagate(&mut self, start_index: usize, target_index: usize, score: f32) {
        let mut current_index = start_index;
        loop {
            // Mutate current node stats
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
            self.backpropagate(end_index, start_index, score);
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
                .max_by(|&a, &b| {
                    let a_score = self.tree.get(*a).state.max_score;
                    let b_score = self.tree.get(*b).state.max_score;
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

    /// Search for good actions step by step. Runs a new simulation from
    /// scratch for each action, and picks the best next action.
    pub fn search_stepwise(
        state: CraftState,
        action_history: Vec<Action>,
        search_options: SearchOptions,
    ) -> (Vec<Action>, CraftState) {
        // only store perfect scores to save on memory
        let search_options = SearchOptions {
            score_storage_threshold: None,
            ..search_options
        };

        let mut state = state;
        let mut actions = action_history;
        while state.check_result().is_none() {
            let mut sim = Self::from_state(&state, search_options);
            let (solution_actions, solution_state) = sim.search(0).solution();

            if solution_state.max_score >= 1.0 {
                return ([actions, solution_actions].concat(), solution_state);
            }

            let chosen_action = solution_actions[0];
            state = chosen_action.execute(&state);
            actions.push(chosen_action);
        }

        (actions, state)
    }

    /// Constructs a single large tree and picks the action path that results in
    /// the highest score.
    pub fn search_oneshot(
        state: CraftState,
        action_history: Vec<Action>,
        search_options: SearchOptions,
    ) -> (Vec<Action>, CraftState) {
        let mut sim = Self::from_state(&state, search_options);
        let (actions, result_state) = sim.search(0).solution();
        ([action_history, actions].concat(), result_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let options = SearchOptions {
            max_steps: 15,
            rng_seed: Some(0),
            ..Default::default()
        };
        Simulator::new(&recipe, &player, options)
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
        let options = SearchOptions {
            max_steps: 25,
            rng_seed: Some(123),
            ..Default::default()
        };
        Simulator::new(&recipe, &player, options)
    }

    fn assert_craft(
        sim: &mut Simulator,
        actions: Vec<Action>,
        progress: u32,
        quality: u32,
        durability: u32,
        cp: u32,
    ) -> &CraftState {
        let (result_node, _) = sim.execute_actions(0, actions);
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
    fn trained_finesse_procs() {
        let actions = vec![
            Reflect,
            WasteNot,
            PreparatoryTouch,
            PreparatoryTouch,
            BasicTouch,
            StandardTouch,
            PrudentTouch,
            PreparatoryTouch,
        ];
        let mut sim = setup_sim_1();
        let (index, _) = sim.execute_actions(0, actions);
        let state = &sim.tree.get(index).state;
        // 10 stacks of IQ
        assert_eq!(10, state.buffs.inner_quiet);
        // should proc Trained Finesse
        assert!(state.available_moves.iter().any(|&a| a == TrainedFinesse));
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
        sim.execute_actions(0, actions);
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
        sim.execute_actions(0, actions);
    }

    #[test]
    fn search_should_not_panic() {
        let mut sim = setup_sim_2();
        sim.search(0).solution();
    }
}
