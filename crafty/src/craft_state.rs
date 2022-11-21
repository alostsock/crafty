use crate::Action;
use serde::Serialize;
use std::fmt;
use ts_type::{wasm_bindgen, TsType};

#[derive(Debug)]
pub enum CraftResult {
    /// Reached 100% progress. Include a score based on quality and steps
    Finished(f32),
    /// Failed either because of durability loss or the step limit was reached
    DurabilityFailure,
    MaxStepsFailure,
    NoMovesFailure,
}

#[derive(Default, Debug, Clone, Serialize, TsType)]
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

    /// Decrements all buff timers by 1 step
    pub fn decrement_timers(&mut self) {
        // don't decrement inner quiet
        self.waste_not = self.waste_not.saturating_sub(1);
        self.waste_not_ii = self.waste_not_ii.saturating_sub(1);
        self.manipulation = self.manipulation.saturating_sub(1);
        self.great_strides = self.great_strides.saturating_sub(1);
        self.innovation = self.innovation.saturating_sub(1);
        self.veneration = self.veneration.saturating_sub(1);
        self.makers_mark = self.makers_mark.saturating_sub(1);
        self.muscle_memory = self.muscle_memory.saturating_sub(1);
    }

    /// An array indicating which buffs are active
    pub fn as_mask(&self) -> [bool; 9] {
        [
            self.inner_quiet > 0,
            self.waste_not > 0,
            self.waste_not_ii > 0,
            self.manipulation > 0,
            self.great_strides > 0,
            self.innovation > 0,
            self.veneration > 0,
            self.makers_mark > 0,
            self.muscle_memory > 0,
        ]
    }
}

#[derive(Debug, Clone, Serialize, TsType)]
pub struct CraftState {
    /// Multiply by synthesis action efficiency for increase in progress
    pub progress_factor: f32,
    /// Multiply by touch action efficiency for increase in quality
    pub quality_factor: f32,
    /// Current step number, starting from 1
    pub step: u8,
    pub step_max: u8,
    pub progress: u32,
    pub progress_target: u32,
    pub quality: u32,
    pub quality_target: u32,
    pub durability: i8,
    pub durability_max: i8,
    pub cp: u32,
    pub cp_max: u32,

    pub observe: bool,
    pub next_combo_action: Option<Action>,
    pub buffs: Buffs,

    /// The action that led to this state
    pub action: Option<Action>,
    // The probability that this state will occur (action chance * condition chance)
    // pub prior: f32,
    /// Sum of scores from this node onward
    pub score_sum: f32,
    /// Maximum score that can be obtained by following this node
    pub max_score: f32,
    /// Number of times this node has been visited
    pub visits: f32,
    pub available_moves: Vec<Action>,
}

impl fmt::Display for CraftState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:>5}/{:>5} progress | {:>5}/{:>5} quality | {:>2}/{:>2} durability | {:>3}/{:>3} cp",
            self.progress,
            self.progress_target,
            self.quality,
            self.quality_target,
            self.durability,
            self.durability_max,
            self.cp,
            self.cp_max
        )
    }
}

impl CraftState {
    pub fn new(
        progress_factor: f32,
        quality_factor: f32,
        progress_target: u32,
        quality_target: u32,
        step_max: u8,
        durability: i8,
        cp: u32,
    ) -> Self {
        let mut state = Self {
            progress_factor,
            quality_factor,
            step: 1,
            step_max,
            progress: 0,
            progress_target,
            quality: 0,
            quality_target,
            durability,
            durability_max: durability,
            cp,
            cp_max: cp,
            observe: false,
            next_combo_action: None,
            buffs: Buffs::new(),
            action: None,
            score_sum: 0.0,
            max_score: 0.0,
            visits: 0.0,
            available_moves: vec![],
        };

        state.set_available_moves(true);
        state
    }

    /// Examine the current craft state and populate `available_moves`.
    /// Enabling `strict` will add more rules that aim to prune as many
    /// suboptimal moves as possible.
    pub fn set_available_moves(&mut self, strict: bool) -> &mut Self {
        if self.progress >= self.progress_target
            || self.step >= self.step_max
            || self.durability == 0
        {
            return self;
        }

        let mut available_moves = Action::ACTIONS.to_vec();
        available_moves.retain(|action| {
            use Action::*;
            let attrs = action.attributes();

            if let Some(base_cost) = attrs.cp_cost {
                if Action::calc_cp_cost(self, base_cost) > self.cp {
                    return false;
                }
            }

            // only allow Focused moves after Observe
            if strict && self.observe && action != &FocusedSynthesis && action != &FocusedTouch {
                return false;
            }

            // don't allow quality moves under Muscle Memory
            if strict && self.buffs.muscle_memory > 0 && attrs.quality_efficiency.is_some() {
                return false;
            }

            // don't allow pure quality moves under Veneration
            if strict
                && self.buffs.veneration > 0
                && attrs.progress_efficiency.is_none()
                && attrs.quality_efficiency.is_some()
            {
                return false;
            }

            // don't allow finishing the craft if there is significant progress remaining
            if strict && self.quality < self.quality_target / 3 {
                if let Some(progress_eff) = attrs.progress_efficiency {
                    let progress_increase = Action::calc_progress_increase(self, progress_eff);
                    if self.progress + progress_increase >= self.progress_target {
                        return false;
                    }
                }
            }

            // don't allow quality moves at max quality
            if self.quality >= self.quality_target && attrs.quality_efficiency.is_some() {
                return false;
            }

            match action {
                MuscleMemory | Reflect => self.step == 1,
                ByregotsBlessing => self.buffs.inner_quiet > 1,
                TrainedFinesse => self.buffs.inner_quiet == 10,
                // use of Waste Not should be efficient
                PrudentSynthesis | PrudentTouch | WasteNot | WasteNotII if strict => {
                    self.buffs.waste_not == 0 && self.buffs.waste_not_ii == 0
                }
                PrudentSynthesis | PrudentTouch => {
                    self.buffs.waste_not == 0 && self.buffs.waste_not_ii == 0
                }
                // don't allow Observe if observing; should also have enough CP to follow up
                Observe if strict => !self.observe && self.cp >= 5,
                Observe => !self.observe,
                // only allow focused skills if observing
                FocusedSynthesis | FocusedTouch => self.observe,
                // don't allow Groundwork if
                //  1) waste not isn't active, or
                //  2) it's downgraded
                Groundwork if strict => {
                    if self.buffs.waste_not == 0 && self.buffs.waste_not_ii == 0 {
                        return false;
                    }
                    let cost = Action::calc_durability_cost(self, attrs.durability_cost.unwrap());
                    self.durability >= cost
                }
                // don't allow buffs too early
                MastersMend if strict => self.durability_max - self.durability <= 10,
                Manipulation if strict => self.buffs.manipulation == 0,
                GreatStrides if strict => {
                    self.buffs.veneration == 0 && self.buffs.great_strides == 0
                }
                Veneration | Innovation if strict => {
                    self.buffs.veneration == 0 && self.buffs.innovation == 0
                }
                // make sure we've exhaustively handled every action; don't use a wildcard here
                AdvancedTouch | BasicSynthesis | BasicTouch | CarefulSynthesis
                | DelicateSynthesis | GreatStrides | Groundwork | Innovation | Manipulation
                | MastersMend | PreparatoryTouch | StandardTouch | Veneration | WasteNot
                | WasteNotII => true,
            }
        });
        self.available_moves = available_moves;

        self
    }

    // An evaluation of the craft from 0 to 1
    pub fn score(&self) -> f32 {
        // bonuses should add up to 1.0
        let quality_bonus: f32 = 0.995;
        let fewer_steps_bonus: f32 = 0.005;

        if self.progress >= self.progress_target {
            let quality_score =
                quality_bonus.min(quality_bonus * self.quality as f32 / self.quality_target as f32);

            let fewer_steps_score =
                fewer_steps_bonus * (1.0_f32 - self.step as f32 / self.step_max as f32);

            quality_score + fewer_steps_score
        } else {
            0.0
        }
    }

    pub fn check_result(&self) -> Option<CraftResult> {
        if self.progress >= self.progress_target {
            Some(CraftResult::Finished(self.score()))
        } else if self.durability <= 0 {
            Some(CraftResult::DurabilityFailure)
        } else if self.step >= self.step_max {
            Some(CraftResult::MaxStepsFailure)
        } else if self.available_moves.is_empty() {
            Some(CraftResult::NoMovesFailure)
        } else {
            None
        }
    }
}
