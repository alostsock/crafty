use crate::action::{Action, ACTIONS};
use rand::Rng;

#[derive(Debug, Clone)]
pub struct CraftState {
    /// Multiply by synthesis action efficiency for increase in progress
    pub progress_factor: f64,
    /// Multiply by touch action efficiency for increase in quality
    pub quality_factor: f64,
    /// Current step number, starting from 1
    pub step: u32,
    pub progress: u32,
    pub quality: u32,
    /// Remaining durability
    pub durability: u32,
    /// Remaining CP
    pub cp: u32,

    /// The action that led to this state
    pub action: Option<Action>,
    /// The probability that this state occurs (action chance * condition chance)
    pub probability: f64,
    /// Number of wins based on the weighted outcomes
    pub wins: f64,
    /// Number of playouts based on weighted outcomes
    pub playouts: f64,
    pub available_moves: Vec<Action>,
}

impl CraftState {
    pub fn new(progress_factor: f64, quality_factor: f64, durability: u32, cp: u32) -> Self {
        CraftState {
            progress_factor,
            quality_factor,
            step: 1,
            progress: 0,
            quality: 0,
            durability,
            cp,
            action: None,
            probability: 1.0,
            wins: 0.0,
            playouts: 0.0,
            available_moves: ACTIONS.to_vec(),
        }
    }

    fn pick_action(&mut self, action: Action) -> Option<Action> {
        if let Some(picked_index) = self.available_moves.iter().position(|&m| m == action) {
            self.available_moves.swap_remove(picked_index);
            Some(action)
        } else {
            None
        }
    }

    pub fn execute_action(&mut self, action: Action) -> Option<CraftState> {
        if let Some(action) = self.pick_action(action) {
            let new_state = action.execute(self);
            Some(new_state)
        } else {
            None
        }
    }

    fn pick_random_action(&mut self) -> Option<Action> {
        let mut rng = rand::thread_rng();
        let random_index = rng.gen_range(0..self.available_moves.len());
        if let Some(random_action) = self.available_moves.get(random_index).cloned() {
            self.available_moves.swap_remove(random_index);
            Some(random_action)
        } else {
            None
        }
    }

    pub fn execute_random_action(&mut self) -> Option<CraftState> {
        if let Some(random_action) = self.pick_random_action() {
            self.execute_action(random_action)
        } else {
            None
        }
    }
}
