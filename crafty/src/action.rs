use crate::craft_state::CraftState;
use std::cmp;

pub struct ActionAttributes {
    progress_efficiency: Option<f64>,
    quality_efficiency: Option<f64>,
    durability_cost: u32,
    cp_cost: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    BasicSynthesis,
    BasicTouch,
}

fn progress(craft_state: &CraftState, efficiency: Option<f64>) -> u32 {
    if let Some(eff) = efficiency {
        let progress_mult = craft_state.progress_factor;
        craft_state.progress + (eff * progress_mult).floor() as u32
    } else {
        craft_state.progress
    }
}

fn quality(craft_state: &CraftState, efficiency: Option<f64>) -> u32 {
    if let Some(eff) = efficiency {
        let quality_mult = craft_state.quality_factor;
        craft_state.quality + (eff * quality_mult).floor() as u32
    } else {
        craft_state.quality
    }
}

fn durability(craft_state: &CraftState, cost: u32) -> u32 {
    let next_durability = craft_state.durability - cost;
    cmp::min(next_durability, craft_state.durability_max)
}

fn cp(craft_state: &CraftState, cost: u32) -> u32 {
    let next_cp = craft_state.cp - cost;
    cmp::min(next_cp, craft_state.cp_max)
}

/// TODO: Determine possible moves based on durability, cost, cp, buffs
fn determine_possible_moves() -> Vec<Action> {
    ACTIONS.to_vec()
}

impl Action {
    pub fn values(&self) -> ActionAttributes {
        use Action::*;
        match *self {
            BasicSynthesis => ActionAttributes {
                progress_efficiency: Some(1.2),
                quality_efficiency: None,
                cp_cost: 0,
                durability_cost: 10,
            },
            BasicTouch => ActionAttributes {
                progress_efficiency: None,
                quality_efficiency: Some(1.0),
                durability_cost: 10,
                cp_cost: 18,
            },
        }
    }

    pub fn execute(&self, craft_state: &CraftState) -> CraftState {
        let ActionAttributes {
            progress_efficiency,
            quality_efficiency,
            durability_cost,
            cp_cost,
        } = self.values();

        CraftState {
            step: craft_state.step + 1,
            progress: progress(craft_state, progress_efficiency),
            quality: quality(craft_state, quality_efficiency),
            durability: durability(craft_state, durability_cost),
            cp: cp(craft_state, cp_cost),
            buffs: craft_state.buffs.decrement_timers(),
            action: Some(*self),
            probability: 1.0,
            wins: 0.0,
            playouts: 0.0,
            available_moves: determine_possible_moves(),
            ..*craft_state
        }

        // TODO: decrement buff timers
    }
}

pub const ACTIONS: &[Action] = &[Action::BasicSynthesis, Action::BasicTouch];
