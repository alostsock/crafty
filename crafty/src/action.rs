use crate::CraftState;

#[derive(Debug, Clone, Copy)]
pub enum Action {
    BasicSynthesis,
    BasicTouch,
}

fn progress(craft_state: &CraftState, efficiency: f64) -> u32 {
    let progress_mult = craft_state.player.progress_factor;
    (efficiency * progress_mult).floor() as u32
}

fn quality(craft_state: &CraftState, efficiency: f64) -> u32 {
    let quality_mult = craft_state.player.quality_factor;
    (efficiency * quality_mult).floor() as u32
}

fn durability(_craft_state: &CraftState, base: u32) -> u32 {
    base
}

impl Action {
    fn execute<'a>(&self, craft_state: &'a CraftState) -> CraftState<'a> {
        let mut next_state = CraftState {
            action: Some(*self),
            probability: 1.0,
            wins: 0.0,
            playouts: 0.0,
            possible_moves: vec![],
            step: craft_state.step + 1,
            ..*craft_state
        };

        use Action::*;
        match *self {
            BasicSynthesis => {
                next_state.progress += progress(craft_state, 1.2);
                next_state.durability -= durability(craft_state, 10);
            }
            BasicTouch => {
                next_state.quality += quality(craft_state, 1.0);
                next_state.durability -= durability(craft_state, 10);
                next_state.cp_remaining -= 18;
            }
        }

        next_state
    }
}

pub const ACTIONS: &[Action] = &[Action::BasicSynthesis, Action::BasicTouch];
