use crate::craft_state::CraftState;

pub struct ActionValues {
    progress_efficiency: Option<f64>,
    quality_efficiency: Option<f64>,
    durability_cost: u32,
    cp_cost: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    BasicSynthesis,
    BasicTouch,
}

fn progress(craft_state: &CraftState, efficiency: Option<f64>) -> u32 {
    if let Some(eff) = efficiency {
        let progress_mult = craft_state.progress_factor;
        (eff * progress_mult).floor() as u32
    } else {
        0
    }
}

fn quality(craft_state: &CraftState, efficiency: Option<f64>) -> u32 {
    if let Some(eff) = efficiency {
        let quality_mult = craft_state.quality_factor;
        (eff * quality_mult).floor() as u32
    } else {
        0
    }
}

impl Action {
    pub fn values(&self) -> ActionValues {
        use Action::*;
        match *self {
            BasicSynthesis => ActionValues {
                progress_efficiency: Some(1.2),
                quality_efficiency: Some(1.0),
                cp_cost: 0,
                durability_cost: 10,
            },
            BasicTouch => ActionValues {
                progress_efficiency: None,
                quality_efficiency: Some(1.0),
                durability_cost: 10,
                cp_cost: 18,
            },
        }
    }

    pub fn execute(&self, craft_state: &CraftState) -> CraftState {
        let mut next_state = CraftState {
            action: Some(*self),
            probability: 1.0,
            wins: 0.0,
            playouts: 0.0,
            possible_moves: vec![],
            step: craft_state.step + 1,
            ..*craft_state
        };

        let ActionValues {
            progress_efficiency,
            quality_efficiency,
            durability_cost,
            cp_cost,
        } = self.values();

        next_state.progress += progress(craft_state, progress_efficiency);
        next_state.quality += quality(craft_state, quality_efficiency);
        next_state.durability_left -= durability_cost;
        next_state.cp_left -= cp_cost;

        next_state
    }
}

pub const ACTIONS: &[Action] = &[Action::BasicSynthesis, Action::BasicTouch];
