use crate::action::Action;
use rand::Rng;

pub struct CraftState {
    // multiply by Synthesis action efficiency to get increase in progress
    pub progress_factor: f64,
    // multiply by Touch action efficiency to get increase in quality
    pub quality_factor: f64,

    // the action that led to this state
    pub action: Option<Action>,
    // the probability that this state occurs
    // (i.e. action chance * condition chance)
    pub probability: f64,
    // can have fractional wins/playouts,
    // based on the weighted probability of its children
    pub wins: f64,
    pub playouts: f64,
    pub possible_moves: Vec<Action>,

    pub step: u32,
    pub progress: u32,
    pub quality: u32,
    pub durability_left: u32,
    pub cp_left: u32,
    // buffs: vec,
}

impl CraftState {
    pub fn new(
        progress_factor: f64,
        quality_factor: f64,
        durability_left: u32,
        cp_left: u32,
    ) -> Self {
        CraftState {
            progress_factor,
            quality_factor,
            action: None,
            probability: 1f64,
            wins: 0f64,
            playouts: 0f64,
            possible_moves: vec![],
            step: 1,
            progress: 0,
            quality: 0,
            durability_left,
            cp_left,
        }
    }

    fn pick_random_action(&mut self) -> Option<Action> {
        let mut rng = rand::thread_rng();
        let random_index = rng.gen_range(0..self.possible_moves.len());
        if let Some(random_action) = self.possible_moves.get(random_index).cloned() {
            self.possible_moves.swap_remove(random_index);
            Some(random_action)
        } else {
            None
        }
    }

    pub fn execute_random_action(&mut self) -> Option<CraftState> {
        if let Some(random_action) = self.pick_random_action() {
            let new_state = random_action.execute(self);
            Some(new_state)
        } else {
            None
        }
    }
}
