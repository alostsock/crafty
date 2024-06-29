use crate::CraftState;
use enum_indexing::EnumIndexing;
use serde::Serialize;
use std::{cmp, fmt};
use ts_type::{wasm_bindgen, TsType};

pub struct Attributes {
    pub level: u32,
    pub progress_efficiency: Option<u32>,
    pub quality_efficiency: Option<u32>,
    pub durability_cost: Option<i8>,
    pub cp_cost: Option<u32>,
    pub effect: Option<fn(&mut CraftState)>,
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
            [$action_name:ident, $label:expr]
                level $level:expr,
                $(progress $progress:expr,)?
                $(quality $quality:expr,)?
                $(durability $durability:expr,)?
                $(cp $cp:expr,)?
                $(effect $effect:expr,)?
        )+ $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, EnumIndexing, TsType)]
        pub enum Action {
            $($action_name,)*
        }

        impl Action {
            pub const ACTIONS: &'static [Action] = &[
                $(Action::$action_name,)*
            ];

            pub fn attributes(&self) -> Attributes {
                match *self {
                    $(
                        Action::$action_name => Attributes {
                            level: $level,
                            progress_efficiency: optional!($( $progress )?),
                            quality_efficiency: optional!($( $quality )?),
                            durability_cost: optional!($( $durability )?),
                            cp_cost: optional!($( $cp )?),
                            effect: optional!($( $effect )?),
                        },
                    )*
                }
            }

            pub fn name(&self) -> &'static str {
                match *self {
                    $(Action::$action_name => stringify!($action_name),)*
                }
            }

            pub fn label(&self) -> &'static str {
                match *self {
                    $(Action::$action_name => $label,)*
                }
            }
        }

        #[derive(Debug)]
        pub struct ActionParseError;

        impl std::str::FromStr for Action {
            type Err = ActionParseError;

            fn from_str(s: &str) -> Result<Action, ActionParseError> {
                match s {
                    $(stringify!($action_name) => Ok(Action::$action_name),)*
                    _ => Err(ActionParseError)
                }
            }
        }
    };
}

// https://na.finalfantasyxiv.com/crafting_gathering_guide/carpenter/
create_actions!(
    [BasicSynthesis, "Basic Synthesis"]
        level 1,
        progress 100,
        durability 10,
    [BasicTouch, "Basic Touch"]
        level 5,
        quality 100,
        durability 10,
        cp 18,
    [MastersMend, "Master's Mend"]
        level 7,
        durability 0,  // indicates that this move is not a buff
        cp 88,
        effect |state| {
            state.durability = cmp::min(state.durability + 30, state.context.durability_max);
        },
    // HastyTouch
    // RapidSynthesis
    [Observe, "Observe"]
        level 13,
        durability 0,  // indicates that this move is not a buff
        cp 7,
    // TricksOfTheTrade
    [WasteNot, "Waste Not"]
        level 15,
        cp 56,
        effect |state| {
            state.buffs.waste_not = 4;
            state.buffs.waste_not_ii = 0;
        },
    [Veneration, "Veneration"]
        level 15,
        cp 18,
        effect |state| {
            state.buffs.veneration = 4;
        },
    [StandardTouch, "Standard Touch"]
        level 18,
        quality 125,
        durability 10,
        cp 32,
    [GreatStrides, "Great Strides"]
        level 21,
        cp 32,
        effect |state| {
            state.buffs.great_strides = 3;
        },
    [Innovation, "Innovation"]
        level 26,
        cp 18,
        effect |state| {
            state.buffs.innovation = 4;
        },
    [BasicSynthesisTraited, "Basic Synthesis"]
        level 31,
        progress 120,
        durability 10,
    // FinalAppraisal
    [WasteNotII, "Waste Not II"]
        level 47,
        cp 98,
        effect |state| {
            state.buffs.waste_not = 0;
            state.buffs.waste_not_ii = 8;
        },
    [ByregotsBlessing, "Byregot's Blessing"]
        level 50,
        quality 0,  // a placeholder to indicate this action *does* affect quality
        durability 10,
        cp 24,
    // PreciseTouch
    [MuscleMemory, "Muscle Memory"]
        level 54,
        progress 300,
        durability 10,
        cp 6,
        effect |state| {
            state.buffs.muscle_memory = 5;
        },
    [CarefulSynthesis, "Careful Synthesis"]
        level 62,
        progress 150,
        durability 10,
        cp 7,
    [Manipulation, "Manipulation"]
        level 65,
        cp 96,
        effect |state| {
            state.buffs.manipulation = 8;
        },
    [PrudentTouch, "Prudent Touch"]
        level 66,
        quality 100,
        durability 5,
        cp 25,
    [AdvancedTouch, "Advanced Touch"]
        level 68,
        quality 150,
        durability 10,
        cp 46,
    [Reflect, "Reflect"]
        level 69,
        quality 300,
        durability 10,
        cp 6,
    [PreparatoryTouch, "Preparatory Touch"]
        level 71,
        quality 200,
        durability 20,
        cp 40,
    [Groundwork, "Groundwork"]
        level 72,
        progress 300,
        durability 20,
        cp 18,
    [DelicateSynthesis, "Delicate Synthesis"]
        level 76,
        progress 100,
        quality 100,
        durability 10,
        cp 32,
    // IntensiveSynthesis
    [TrainedEye, "Trained Eye"]
        level 80,
        quality 0, // a placeholder to indicate this action *does* affect quality
        durability 0,
        cp 250,
    [CarefulSynthesisTraited, "Careful Synthesis"]
        level 82,
        progress 180,
        durability 10,
        cp 7,
    [GroundworkTraited, "Groundwork"]
        level 86,
        progress 360,
        durability 20,
        cp 18,
    [PrudentSynthesis, "Prudent Synthesis"]
        level 88,
        progress 180,
        durability 5,
        cp 18,
    [TrainedFinesse, "Trained Finesse"]
        level 90,
        quality 100,
        cp 32,
    [RefinedTouch, "Refined Touch"]
        level 92,
        quality 100,
        cp 24,
    [DelicateSynthesisTraited, "Delicate Synthesis"]
        level 94,
        progress 150,
        quality 100,
        durability 10,
        cp 32,
    // DaringTouch
    [QuickInnovation, "Quick Innovation"]
        level 96,
        effect |state| {
            state.buffs.innovation = 1;
            state.quick_innovation_available = false;
        },
    [ImmaculateMend, "Immaculate Mend"]
        level 98,
        durability 0,  // indicates that this move is not a buff
        cp 112,
        effect |state| {
            state.durability = state.context.durability_max;
        },
    [TrainedPerfection, "Trained Perfection"]
        level 100,
        durability 0,  // indicates that this move is not a buff
        effect |state| {
            state.trained_perfection_available = false;
        },

);

impl Action {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_precision_loss)]
    pub fn calc_progress_increase(state: &CraftState, efficiency: u32) -> u32 {
        let base = state.context.base_progress_factor;

        let mut multiplier = 1.0;
        if state.buffs.veneration > 0 {
            multiplier += 0.5;
        }
        if state.buffs.muscle_memory > 0 {
            multiplier += 1.0;
        }

        (base * efficiency as f32 * multiplier / 100.0) as u32
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_precision_loss)]
    pub fn calc_quality_increase(state: &CraftState, efficiency: u32) -> u32 {
        if state.action == Some(Action::TrainedEye) {
            return state.context.quality_target - state.quality;
        }

        let base = state.context.base_quality_factor;

        let efficiency = if state.action == Some(Action::ByregotsBlessing) {
            100 + u32::from(state.buffs.inner_quiet) * 20
        } else {
            efficiency
        };

        let mut modifier = 1.0 + f32::from(state.buffs.inner_quiet) / 10.0;

        let mut multiplier = 1.0;
        if state.buffs.innovation > 0 {
            multiplier += 0.5;
        }
        if state.buffs.great_strides > 0 {
            multiplier += 1.0;
        }

        modifier *= multiplier;

        (base * efficiency as f32 * modifier / 100.0) as u32
    }

    pub fn calc_durability_cost(state: &CraftState, base_cost: i8) -> i8 {
        if state.previous_combo_action == Some(Action::TrainedPerfection) {
            return 0;
        }
        if state.buffs.waste_not > 0 || state.buffs.waste_not_ii > 0 {
            return base_cost / 2;
        }
        base_cost
    }

    pub fn calc_cp_cost(state: &CraftState, base_cost: u32) -> u32 {
        use Action::*;

        match (state.previous_combo_action, state.action) {
            (Some(BasicTouch), Some(StandardTouch))
            | (Some(StandardTouch) | Some(Observe), Some(AdvancedTouch)) => 18,
            _ => base_cost,
        }
    }

    pub fn macro_text(&self) -> String {
        let mut label = self.label().to_string();
        if label.contains(' ') {
            label = format!("\"{label}\"");
        }

        let attrs = self.attributes();
        let is_buff = attrs.progress_efficiency.is_none()
            && attrs.quality_efficiency.is_none()
            && attrs.durability_cost.is_none();
        let wait_time = if is_buff { 2 } else { 3 };

        format!("/ac {label} <wait.{wait_time}>")
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}
