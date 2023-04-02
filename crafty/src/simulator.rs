use crate::{tree::Arena, Action, CraftContext, CraftResult, CraftState};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use serde::Deserialize;
use ts_type::{wasm_bindgen, TsType};

#[derive(Clone, Copy, Deserialize, TsType)]
pub struct SearchOptions {
    /// Number of simulations to run
    pub iterations: u32,
    /// Numerical seed to use for RNG. Randomly picked if None
    pub rng_seed: Option<u64>,
    /// A memory optimization option that specifies the minimum score a craft has
    /// to reach for action history to be stored. Only stores ~100% HQ states if None.
    pub score_storage_threshold: Option<f32>,
    pub max_score_weighting_constant: Option<f32>,
    pub exploration_constant: Option<f32>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            iterations: 10_000,
            rng_seed: Some(SmallRng::from_entropy().gen()),
            score_storage_threshold: Some(1.0),
            max_score_weighting_constant: Some(0.1),
            exploration_constant: Some(1.5),
        }
    }
}

#[derive(Debug)]
pub struct Simulator<'a> {
    tree: Arena<CraftState<'a>>,
    iterations: u32,
    /// Amount of "dead ends" encountered. This means a node was selected, but
    /// there weren't any available moves.
    pub dead_ends_selected: u64,
    pub rng_seed: u64,
    rng: SmallRng,
    score_storage_threshold: f32,
    /// The higher the weight, the more a node's potential max score is valued
    /// over its average score. A weight of 1.0 means only max scores will be used;
    /// 0.0 means only average scores will be used.
    max_score_weighting_constant: f32,
    /// Higher values prioritize exploring less promising nodes.
    exploration_constant: f32,
}

impl<'a> Simulator<'a> {
    fn from_state(state: CraftState<'a>, options: SearchOptions) -> Self {
        let defaults = SearchOptions::default();
        let rng_seed = options.rng_seed.or(defaults.rng_seed).unwrap();

        Self {
            tree: Arena::new(state),
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

    fn from_context(context: &'a CraftContext, options: SearchOptions) -> Self {
        Self::from_state(CraftState::new(context), options)
    }

    /// Executes a series of actions with most game-valid moves available. Will
    /// return early with `CraftResult::InvalidActionFailure` if an illegal move
    /// is chosen.
    fn execute_actions(
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

            // the next action must be available to use
            if current_state.available_moves.contains(action) {
                current_state.available_moves.unset(action);
            } else {
                return (current_index, Some(CraftResult::InvalidActionFailure));
            }

            let next_state = current_state.execute(&action);
            let next_index = self.tree.insert(current_index, next_state);
            current_index = next_index;
        }

        // check state after performing the last action
        let current_state = &mut self.tree.get_mut(current_index).state;
        (current_index, current_state.check_result())
    }

    /// Executes a series of actions with strict move pruning enabled.
    fn execute_actions_strict(
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

            // the next action must be available to use
            if current_state.available_moves.contains(action) {
                current_state.available_moves.unset(action);
            } else {
                return (current_index, Some(CraftResult::InvalidActionFailure));
            }

            let next_state = current_state.execute_strict(&action);
            let next_index = self.tree.insert(current_index, next_state);

            current_index = next_index;
        }

        // check state after performing the last action
        let current_state = &self.tree.get_mut(current_index).state;
        (current_index, current_state.check_result())
    }

    /// Calculate the UCB1 score for a node
    fn eval(&self, state: &CraftState, parent_state: &CraftState) -> f32 {
        let w = self.max_score_weighting_constant;
        let c = self.exploration_constant;

        let visits = state.visits;
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
        let random_action = initial_state.available_moves.pick(&mut self.rng);
        let expanded_state = initial_state.execute_strict(&random_action);
        let expanded_index = self.tree.insert(initial_index, expanded_state);

        // playout to a terminal state
        let mut current_state = self.tree.get(expanded_index).state.clone();
        let mut action_history: Vec<Action> = vec![];
        let result = loop {
            if let Some(result) = current_state.check_result() {
                break result;
            }
            let random_action = current_state.available_moves.sample(&mut self.rng);
            action_history.push(random_action);
            current_state = current_state.execute_strict(&random_action);
        };

        // store the result if a max score was reached
        match result {
            CraftResult::Finished(score)
                if score >= self.score_storage_threshold
                    && score >= self.tree.nodes[0].state.max_score =>
            {
                let (terminal_index, _) =
                    self.execute_actions_strict(expanded_index, action_history);
                (terminal_index, result)
            }
            _ => (expanded_index, result),
        }
    }

    /// From a starting node, follow parent nodes back to the root node, updating
    /// statistics for each node along the way.
    fn backpropagate(&mut self, start_index: usize, target_index: usize, score: f32) {
        let mut current_index = start_index;
        loop {
            // Mutate current node stats
            let current_node = self.tree.get_mut(current_index);
            current_node.state.visits += 1.0;
            current_node.state.score_sum += score;
            current_node.state.max_score = current_node.state.max_score.max(score);

            if current_index == target_index {
                break;
            }

            current_index = current_node.parent.unwrap();
        }
    }

    /// The starting point for one round of MCTS.
    fn search(&mut self, start_index: usize) -> &mut Self {
        for _ in 0..self.iterations {
            let selected_index = self.select(start_index);
            let (end_index, result) = self.expand_and_rollout(selected_index);

            if selected_index == end_index {
                self.dead_ends_selected += 1;
            }

            let score = match result {
                CraftResult::Finished(s) => s,
                _ => 0.0,
            };
            self.backpropagate(end_index, start_index, score);
        }
        self
    }

    /// Traverses the current tree, following actions that result in the highest
    /// score to find the best solution. This is a convenient way to extract a
    /// solution after running `search`.
    fn solution(&self) -> (Vec<Action>, CraftState<'a>) {
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

    /// A standalone method to obtain a `CraftState` from a series of actions.
    pub fn simulate(
        context: &'a CraftContext,
        actions: Vec<Action>,
    ) -> (CraftState<'a>, Option<CraftResult>) {
        let mut sim = Self::from_context(context, SearchOptions::default());
        let (index, result) = sim.execute_actions(0, actions);
        (sim.tree.get(index).state.clone(), result)
    }

    /// Searches for good actions step by step. Creates a fresh tree and runs a
    /// new search from scratch for each action picked.
    pub fn search_stepwise(
        context: &'a CraftContext,
        action_history: Vec<Action>,
        search_options: SearchOptions,
        action_callback: Option<&dyn Fn(Action)>,
    ) -> (Vec<Action>, CraftState<'a>) {
        // only store perfect scores to reduce memory usage
        let search_options = SearchOptions {
            score_storage_threshold: None,
            ..search_options
        };

        let (start_state, result) = Self::simulate(context, action_history.clone());
        if result.is_some() {
            return (action_history, start_state);
        }

        let mut state = start_state.clone_strict();
        let mut actions = action_history;
        while state.check_result().is_none() {
            let mut sim = Self::from_state(state.clone(), search_options);
            let (solution_actions, solution_state) = sim.search(0).solution();

            if solution_state.max_score >= 1.0 {
                return ([actions, solution_actions].concat(), solution_state);
            }

            let chosen_action = solution_actions[0];
            state = state.execute_strict(&chosen_action);
            actions.push(chosen_action);

            if let Some(action_callback) = action_callback {
                action_callback(chosen_action);
            }
        }

        (actions, state)
    }

    /// Constructs a single large tree, storing good candidate paths in memory
    /// based on the `score_storage_threshold` option. When the iteration limit
    /// is reached, the action path that results in the highest score is returned.
    pub fn search_oneshot(
        context: &'a CraftContext,
        action_history: Vec<Action>,
        search_options: SearchOptions,
    ) -> (Vec<Action>, CraftState<'a>) {
        let mut sim = Self::from_context(context, search_options);
        let (actions, result_state) = sim.search(0).solution();
        ([action_history, actions].concat(), result_state)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Action, CraftContext, Player, Recipe, SearchOptions, Simulator};
    use Action::*;

    fn setup_1() -> (CraftContext, SearchOptions) {
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
        let context = CraftContext::new(&player, &recipe, 25);
        let options = SearchOptions {
            rng_seed: Some(0),
            ..Default::default()
        };
        (context, options)
    }

    fn setup_2() -> (CraftContext, SearchOptions) {
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
        let context = CraftContext::new(&player, &recipe, 25);
        let options = SearchOptions {
            rng_seed: Some(123),
            ..Default::default()
        };
        (context, options)
    }

    fn assert_craft(
        context: &CraftContext,
        actions: Vec<Action>,
        progress: u32,
        quality: u32,
        durability: i8,
        cp: u32,
    ) {
        let (end_state, _) = Simulator::simulate(context, actions);
        assert_eq!(end_state.progress, progress);
        assert_eq!(end_state.quality, quality);
        assert_eq!(end_state.durability, durability);
        assert_eq!(end_state.cp, cp);
    }

    #[test]
    fn basic_actions() {
        let actions = vec![BasicTouch, BasicSynthesis, MastersMend];
        let (context, _) = setup_1();
        assert_craft(&context, actions, 276, 262, 80, 469);
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
        let (context, _) = setup_1();
        assert_craft(&context, actions, 0, 2828, 30, 425);
    }

    #[test]
    fn with_buffs_1() {
        let actions = vec![Reflect, Manipulation, PreparatoryTouch, WasteNotII];
        let (context, _) = setup_1();
        assert_craft(&context, actions, 0, 890, 60, 335);
    }

    #[test]
    fn with_buffs_2() {
        let actions = vec![MuscleMemory, GreatStrides, PrudentTouch, DelicateSynthesis];
        let (context, _) = setup_1();
        assert_craft(&context, actions, 1150, 812, 55, 480);
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
        let (context, _) = setup_1();
        assert_craft(&context, actions, 1150, 1925, 80, 163);
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
        let (context, _) = setup_1();
        let (end_state, _) = Simulator::simulate(&context, actions);
        // 10 stacks of IQ
        assert_eq!(10, end_state.buffs.inner_quiet);
        // should proc Trained Finesse
        assert!(end_state.available_moves.contains(TrainedFinesse));
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
        let (context, _) = setup_1();
        Simulator::simulate(&context, actions);
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
        let (context, _) = setup_2();
        Simulator::simulate(&context, actions);
    }

    #[test]
    fn search_should_not_panic() {
        let (context, options) = setup_2();
        Simulator::search_oneshot(&context, vec![], options);
    }
}
