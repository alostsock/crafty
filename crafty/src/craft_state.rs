use crate::action::{Action, ACTIONS};
use rand::Rng;

#[derive(Default, Debug, Clone)]
pub struct Buffs {
    pub inner_quiet: u8,
    pub waste_not: u8,
    pub waste_not_ii: u8,
    pub manipulation: u8,
    pub great_strides: u8,
    pub innovation: u8,
    pub veneration: u8,
    pub makers_mark: u8,
    pub muscle_memory: u8,
}

impl Buffs {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn decrement_timers(&self) -> Self {
        Buffs {
            inner_quiet: self.inner_quiet.saturating_sub(1),
            waste_not: self.waste_not.saturating_sub(1),
            waste_not_ii: self.waste_not_ii.saturating_sub(1),
            manipulation: self.manipulation.saturating_sub(1),
            great_strides: self.great_strides.saturating_sub(1),
            innovation: self.innovation.saturating_sub(1),
            veneration: self.veneration.saturating_sub(1),
            makers_mark: self.makers_mark.saturating_sub(1),
            muscle_memory: self.muscle_memory.saturating_sub(1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CraftState {
    /// Multiply by synthesis action efficiency for increase in progress
    pub progress_factor: f64,
    /// Multiply by touch action efficiency for increase in quality
    pub quality_factor: f64,
    /// Current step number, starting from 1
    pub step: u8,
    pub progress: u32,
    pub progress_target: u32,
    pub quality: u32,
    pub quality_target: u32,
    pub durability: u32,
    pub durability_max: u32,
    pub cp: u32,
    pub cp_max: u32,

    pub buffs: Buffs,

    /// The action that led to this state
    pub action: Option<Action>,
    /// The probability that this state occurs (action chance * condition chance)
    pub probability: f64,
    /// Number of wins based on weighted outcomes from this node onward
    pub wins: f64,
    /// Number of playouts based on weighted outcomes from this node onward
    pub playouts: f64,
    pub available_moves: Vec<Action>,
}

impl CraftState {
    pub fn new(
        progress_factor: f64,
        quality_factor: f64,
        progress_target: u32,
        quality_target: u32,
        durability: u32,
        cp: u32,
    ) -> Self {
        CraftState {
            progress_factor,
            quality_factor,
            step: 1,
            progress: 0,
            progress_target,
            quality: 0,
            quality_target,
            durability,
            durability_max: durability,
            cp,
            cp_max: cp,
            buffs: Buffs::new(),
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
