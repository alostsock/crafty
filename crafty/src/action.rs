use crate::craft_state::CraftState;
use std::{cmp, fmt};

pub struct ActionAttributes {
    pub progress_efficiency: Option<f32>,
    pub quality_efficiency: Option<f32>,
    pub durability_cost: Option<u32>,
    pub cp_cost: Option<u32>,
    effect: Option<fn(&mut CraftState)>,
}

macro_rules! optional {
    () => {
        None
    };
    ($e:expr) => {
        Some($e)
    };
}

macro_rules! create_actions {
    (
        $(
            $action_name:ident (
                $(progress $progress:expr,)?
                $(quality $quality:expr,)?
                $(durability $durability:expr,)?
                $(cp $cp:expr,)?
                $(effect $effect:expr)?
            )
        ),+ $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Action {
            $($action_name,)*
        }


        impl Action {
            pub const ACTIONS: &'static [Action] = &[
                $(Action::$action_name,)*
            ];

            pub fn attributes(&self) -> ActionAttributes {
                match *self {
                    $(
                        Action::$action_name => ActionAttributes {
                            progress_efficiency: optional!($( $progress )?),
                            quality_efficiency: optional!($( $quality )?),
                            durability_cost: optional!($( $durability )?),
                            cp_cost: optional!($( $cp )?),
                            effect: optional!($( $effect )?),
                        },
                    )*
                }
            }
        }
    };
}

// https://na.finalfantasyxiv.com/crafting_gathering_guide/carpenter/
create_actions!(
    BasicSynthesis(progress 1.2, durability 10,),
    BasicTouch(quality 1.0, durability 10, cp 18, effect |state| {
        state.next_combo = Some(Action::StandardTouch);
    }),
    MastersMend(cp 88, effect |state| {
        state.durability = cmp::min(state.durability + 30, state.durability_max);
    }),
    // HastyTouch
    // RapidSynthesis
    Observe(cp 7, effect |state| {
        state.observe = true;
    }),
    // TricksOfTheTrade
    WasteNot(cp 56, effect |state| {
        state.buffs.waste_not = 4;
    }),
    Veneration(cp 18, effect |state| {
        state.buffs.veneration = 4;
    }),
    StandardTouch(quality 1.25, durability 10, cp 32, effect |state| {
        if state.next_combo == Some(Action::StandardTouch) {
            state.next_combo = Some(Action::AdvancedTouch);
        }
    }),
    GreatStrides(cp 32, effect |state| {
        state.buffs.great_strides = 3;
    }),
    Innovation(cp 18, effect |state| {
        state.buffs.innovation = 4;
    }),
    // FinalAppraisal
    WasteNotII(cp 98, effect |state| {
        state.buffs.waste_not_ii = 8;
    }),
    ByregotsBlessing(quality 0.0, durability 10, cp 24,),
    // PreciseTouch
    MuscleMemory(progress 3.0, durability 10, cp 6, effect |state| {
        state.buffs.muscle_memory = 5;
    }),
    CarefulSynthesis(progress 1.8, durability 10, cp 7,),
    Manipulation(cp 96, effect |state| {
        state.buffs.manipulation = 8;
    }),
    PrudentTouch(quality 1.0, durability 5, cp 25,),
    FocusedSynthesis(progress 2.0, durability 10, cp 5,),
    FocusedTouch(quality 1.5, durability 10, cp 18,),
    Reflect(quality 1.0, durability 10, cp 6, effect |state| {
        state.buffs.inner_quiet += 1;
    }),
    PreparatoryTouch(quality 2.0, durability 20, cp 40, effect |state| {
        state.buffs.inner_quiet = cmp::min(state.buffs.inner_quiet + 1, 10);
    }),
    Groundwork(progress 3.6, durability 20, cp 18,),
    DelicateSynthesis(progress 1.0, quality 1.0, durability 10, cp 32,),
    // Intensive Synthesis
    // TrainedEye
    AdvancedTouch(quality 1.5, durability 10, cp 46, effect |state| {
        state.next_combo = None;
    }),
    PrudentSynthesis(progress 1.8, durability 10, cp 18,),
    TrainedFinesse(quality 1.0, cp 32,),
);

impl Action {
    pub fn calc_progress_increase(state: &CraftState, efficiency: f32) -> u32 {
        let base = state.progress_factor;

        let mut multiplier = 1.0;
        if state.buffs.veneration > 0 {
            multiplier += 0.5;
        }
        if state.buffs.muscle_memory > 0 {
            multiplier += 1.0;
        }

        (base * efficiency * multiplier).floor() as u32
    }

    pub fn calc_quality_increase(state: &CraftState, efficiency: f32) -> u32 {
        let base = state.quality_factor;

        let mut efficiency = efficiency;

        if state.action == Some(Action::ByregotsBlessing) {
            efficiency = 1.0 + state.buffs.inner_quiet as f32 * 0.2;
        }

        let mut modifier = 1.0 + state.buffs.inner_quiet as f32 / 10.0;

        let mut multiplier = 1.0;
        if state.buffs.innovation > 0 {
            multiplier += 0.5;
        }
        if state.buffs.great_strides > 0 {
            multiplier += 1.0;
        }

        modifier *= multiplier;

        (base * efficiency * modifier).floor() as u32
    }

    pub fn calc_durability_cost(state: &CraftState, base_cost: u32) -> u32 {
        if state.buffs.waste_not > 0 || state.buffs.waste_not_ii > 0 {
            return base_cost / 2;
        }
        base_cost
    }

    pub fn calc_cp_cost(state: &CraftState, base_cost: u32) -> u32 {
        // test for basic touch combo
        if state.action.is_some() && state.action == state.next_combo {
            return 18;
        }
        base_cost
    }

    pub fn execute(self, prev_state: &CraftState) -> CraftState {
        let mut state = CraftState {
            step: prev_state.step + 1,
            buffs: prev_state.buffs.clone(),
            action: Some(self),
            prior: 1.0,
            score_sum: 0.0,
            max_score: 0.0,
            visits: 0.0,
            available_moves: vec![],
            ..*prev_state
        };

        let action = self.attributes();

        if let Some(efficiency) = action.progress_efficiency {
            state.progress += Action::calc_progress_increase(&state, efficiency);
            state.buffs.muscle_memory = 0;
        }

        if let Some(efficiency) = action.quality_efficiency {
            state.quality += Action::calc_quality_increase(&state, efficiency);
            if self == Action::ByregotsBlessing {
                state.buffs.inner_quiet = 0;
            } else {
                state.buffs.inner_quiet = cmp::min(state.buffs.inner_quiet + 1, 10);
            }
            state.buffs.great_strides = 0;
        }

        if let Some(base_cost) = action.durability_cost {
            state.durability = state
                .durability
                .saturating_sub(Action::calc_durability_cost(&state, base_cost));
        }

        if state.buffs.manipulation > 0 && state.durability > 0 {
            state.durability = cmp::min(state.durability + 5, state.durability_max);
        }

        if let Some(base_cost) = action.cp_cost {
            state.cp -= Action::calc_cp_cost(&state, base_cost);
        }

        state.observe = false;

        if state.next_combo != Some(self) {
            state.next_combo = None;
        }

        state.buffs.decrement_timers();

        // Always apply effects last
        if let Some(apply_effect) = action.effect {
            apply_effect(&mut state);
        }

        state.determine_possible_moves();

        state
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
