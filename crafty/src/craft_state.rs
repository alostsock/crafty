use crate::{action::Attributes, Action, ActionSet, CraftContext};
use serde::Serialize;
use std::{cmp, fmt};
use ts_type::{wasm_bindgen, TsType};

#[derive(Debug)]
pub enum CraftResult {
    /// The craft reached 100% progress. Includes the score of the `CraftState`.
    Finished(f32),
    /// No durability remains.
    DurabilityFailure,
    /// The step limit was reached.
    MaxStepsFailure,
    /// No actions are available, or an invalid action was used.
    InvalidActionFailure,
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
    pub muscle_memory: u8,
}

impl Buffs {
    pub fn new() -> Self {
        Self::default()
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
        self.muscle_memory = self.muscle_memory.saturating_sub(1);
    }
}

#[derive(Debug, Clone)]
pub struct CraftState<'a> {
    /// This is intended to be a readonly field that contains important values
    /// that won't change while a craft is in progress. This reduces the amount
    /// of data we need to store in each node, and reduces memory usage.
    pub context: &'a CraftContext,

    pub step: u8,
    pub progress: u32,
    pub quality: u32,
    pub durability: i8,
    pub cp: u32,

    pub previous_combo_action: Option<Action>,
    pub quick_innovation_available: bool,
    pub trained_perfection_active: Option<bool>,
    pub buffs: Buffs,

    /// The action that led to this state
    pub action: Option<Action>,
    /// Sum of scores from this node onward
    pub score_sum: f32,
    /// Maximum score that can be obtained by following this node
    pub max_score: f32,
    /// Number of times this node has been visited
    pub visits: f32,
    pub available_moves: ActionSet,
}

impl<'a> fmt::Display for CraftState<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:>5}/{:>5} progress | {:>5}/{:>5} quality | {:>2}/{:>2} durability | {:>3}/{:>3} cp",
            self.progress,
            self.context.progress_target,
            self.quality,
            self.context.quality_target,
            self.durability,
            self.context.durability_max,
            self.cp,
            self.context.cp_max
        )
    }
}

impl<'a> CraftState<'a> {
    pub fn _new(context: &'a CraftContext) -> Self {
        Self {
            context,
            step: 1,
            progress: 0,
            quality: context.starting_quality,
            durability: context.durability_max,
            cp: context.cp_max,
            previous_combo_action: None,
            quick_innovation_available: context.use_delineation,
            trained_perfection_active: None,
            buffs: Buffs::new(),
            action: None,
            score_sum: 0.0,
            max_score: 0.0,
            visits: 0.0,
            available_moves: ActionSet::new(),
        }
    }

    pub fn new(context: &'a CraftContext) -> Self {
        let mut state = Self::_new(context);
        state.set_available_moves(false);
        state
    }

    pub fn new_strict(context: &'a CraftContext) -> Self {
        let mut state = Self::_new(context);
        state.set_available_moves(true);
        state
    }

    pub fn clone_strict(&self) -> Self {
        let mut state = self.clone();
        state.set_available_moves(true);
        state
    }

    /// Examine the current craft state and populate `available_moves`.
    /// Enabling `strict` will add more rules that aim to prune as many
    /// suboptimal moves as possible.
    #[allow(clippy::too_many_lines)]
    fn set_available_moves(&mut self, strict: bool) -> &mut Self {
        if self.progress >= self.context.progress_target
            || self.step >= self.context.step_max
            || self.durability <= 0
        {
            return self;
        }

        let mut available_moves = self.context.action_pool.clone();
        available_moves.keep(|action| {
            use Action::*;
            let attrs = action.attributes();

            if let Some(base_cost) = attrs.cp_cost {
                if Action::calc_cp_cost(self, base_cost) > self.cp {
                    return false;
                }
            }

            // don't allow quality moves at max quality
            if self.quality >= self.context.quality_target && attrs.quality_efficiency.is_some() {
                return false;
            }

            if strict {
                // always used Trained Eye if it's available
                if self.step == 1
                    && self.context.quality_target > 0
                    && !self.context.is_expert
                    && self.context.action_pool.contains(TrainedEye)
                {
                    return action == &TrainedEye;
                }

                // don't allow quality moves under Muscle Memory for difficult crafts
                if self.context.recipe_job_level == self.context.player_job_level
                    && self.buffs.muscle_memory > 0
                    && attrs.quality_efficiency.is_some()
                {
                    return false;
                }

                // don't allow pure quality moves under Veneration
                if self.buffs.veneration > 0
                    && attrs.progress_efficiency.is_none()
                    && attrs.quality_efficiency.is_some()
                {
                    return false;
                }

                // only allow Advanced Touch when Observing
                if self.previous_combo_action == Some(Observe) && action != &AdvancedTouch {
                    return false;
                }

                if let Some(progress_eff) = attrs.progress_efficiency {
                    let progress_increase = Action::calc_progress_increase(self, progress_eff);
                    let would_finish =
                        self.progress + progress_increase >= self.context.progress_target;

                    if would_finish {
                        // don't allow finishing the craft if there is significant quality remaining
                        if self.quality < self.context.quality_target / 5 {
                            return false;
                        }
                    } else {
                        // don't allow pure progress moves under Innovation, if it wouldn't finish the craft
                        if self.buffs.innovation > 0
                            && attrs.quality_efficiency.is_none()
                            && attrs.progress_efficiency.is_some()
                        {
                            return false;
                        }
                    }
                }
            }

            match action {
                MuscleMemory | Reflect => self.step == 1,
                TrainedEye => self.step == 1 && !self.context.is_expert,
                ByregotsBlessing if strict => self.buffs.inner_quiet > 1,
                ByregotsBlessing => self.buffs.inner_quiet > 0,
                TrainedFinesse => self.buffs.inner_quiet == 10,
                TrainedPerfection => self.trained_perfection_active.is_none(),
                // use of Waste Not should be efficient
                PrudentSynthesis | PrudentTouch | WasteNot | WasteNotII if strict => {
                    self.buffs.waste_not == 0 && self.buffs.waste_not_ii == 0
                }
                PrudentSynthesis | PrudentTouch => {
                    self.buffs.waste_not == 0 && self.buffs.waste_not_ii == 0
                }
                // don't allow Observe if observing
                // should also have enough CP to follow up with Advanced Touch (7 + 18 CP)
                Observe if strict => self.previous_combo_action != Some(Observe) && self.cp >= 25,
                Observe => self.previous_combo_action != Some(Observe),
                // don't allow Groundwork if it's downgraded
                Groundwork | GroundworkTraited => {
                    let cost = Action::calc_durability_cost(self, attrs.durability_cost.unwrap());
                    self.durability >= cost
                }
                // don't allow Refined Touch without a combo
                RefinedTouch => self.previous_combo_action == Some(BasicTouch),
                // don't allow Immaculate Mends that are too inefficient
                ImmaculateMend if strict => {
                    self.context.durability_max - self.durability > 45
                        && self.buffs.manipulation == 0
                }
                // don't allow buffs too early
                MastersMend if strict => self.context.durability_max - self.durability >= 25,
                Manipulation if strict => {
                    self.context.use_manipulation && self.buffs.manipulation == 0
                }
                Manipulation => self.context.use_manipulation,
                GreatStrides if strict => self.buffs.great_strides == 0,
                Veneration | Innovation if strict => {
                    self.buffs.veneration <= 1 && self.buffs.innovation <= 1
                }
                QuickInnovation if strict => {
                    self.quick_innovation_available
                        && self.buffs.innovation == 0
                        && self.quality > self.context.quality_target / 3
                }
                QuickInnovation => self.quick_innovation_available && self.buffs.innovation == 0,
                // make sure we've exhaustively handled every action; don't use a wildcard here
                AdvancedTouch
                | BasicSynthesis
                | BasicSynthesisTraited
                | BasicTouch
                | CarefulSynthesis
                | CarefulSynthesisTraited
                | DelicateSynthesis
                | DelicateSynthesisTraited
                | GreatStrides
                | Innovation
                | ImmaculateMend
                | MastersMend
                | PreparatoryTouch
                | StandardTouch
                | Veneration
                | WasteNot
                | WasteNotII => true,
            }
        });
        self.available_moves = available_moves;

        self
    }

    // interesting lint, but passing by value apparently results in a 2-3% performance regression?
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn _execute(&self, &action: &Action) -> Self {
        let mut state = Self {
            step: if action == Action::QuickInnovation {
                self.step
            } else {
                self.step + 1
            },
            buffs: self.buffs.clone(),
            action: Some(action),
            score_sum: 0.0,
            max_score: 0.0,
            visits: 0.0,
            available_moves: ActionSet::new(),
            ..*self
        };

        let Attributes {
            level: _,
            progress_efficiency,
            quality_efficiency,
            durability_cost,
            cp_cost,
            effect,
        } = action.attributes();

        if let Some(efficiency) = progress_efficiency {
            state.progress += Action::calc_progress_increase(&state, efficiency);
            state.buffs.muscle_memory = 0;
        }

        if let Some(efficiency) = quality_efficiency {
            state.quality += Action::calc_quality_increase(&state, efficiency);

            if state.context.player_job_level >= 11 {
                state.buffs.inner_quiet = match (state.previous_combo_action, action) {
                    (Some(Action::BasicTouch), Action::RefinedTouch)
                    | (_, Action::Reflect | Action::PreparatoryTouch) => {
                        cmp::min(state.buffs.inner_quiet + 2, 10)
                    }
                    (_, Action::ByregotsBlessing) => 0,
                    _ => cmp::min(state.buffs.inner_quiet + 1, 10),
                };
            }

            state.buffs.great_strides = 0;
        }

        if let Some(base_cost) = durability_cost {
            state.durability -= Action::calc_durability_cost(&state, base_cost);

            if base_cost > 0 && state.trained_perfection_active == Some(true) {
                state.trained_perfection_active = Some(false);
            }
        }

        if state.buffs.manipulation > 0 && state.durability > 0 {
            state.durability = cmp::min(state.durability + 5, state.context.durability_max);
        }

        if let Some(base_cost) = cp_cost {
            state.cp -= Action::calc_cp_cost(&state, base_cost);
        }

        state.previous_combo_action = match (state.previous_combo_action, action) {
            (Some(Action::BasicTouch), Action::StandardTouch)
            | (Some(Action::BasicTouch), Action::RefinedTouch)
            | (_, Action::BasicTouch | Action::Observe) => Some(action),
            _ => None,
        };

        if action != Action::QuickInnovation {
            state.buffs.decrement_timers();
        }

        // Always apply buffs last
        if let Some(apply_effect) = effect {
            apply_effect(&mut state);
        }

        state
    }

    /// Executes the action against a `CraftState`, and returns a `CraftState` with
    /// all available moves
    pub fn execute(&self, action: &Action) -> Self {
        let mut state = self._execute(action);
        state.set_available_moves(false);
        state
    }

    /// Executes the action against a `CraftState`, and returns a `CraftState` with
    /// a strict, pruned moveset
    pub fn execute_strict(&self, action: &Action) -> Self {
        let mut state = self._execute(action);
        state.set_available_moves(true);
        state
    }

    /// An evaluation of the craft. Returns a value from 0 to 1.
    #[allow(clippy::cast_precision_loss)]
    pub fn score(&self) -> f32 {
        fn apply(bonus: f32, value: f32, target: f32) -> f32 {
            bonus * 1f32.min(value / target)
        }

        // bonuses should add up to 1.0

        // The search only expands on finished states (100% progress) so you may
        // be thinking, "Why do we need to reward progress if we don't score
        // unfinished craft states at all?". Two reasons:
        // 1) Conceptually, I think the progress bonus is still useful as a
        //    weight against the other bonuses
        // 2) Practically, it ensures the score of a state is sufficiently above
        //    zero without having to rely solely on durability, cp, and step
        //    metrics, which by themselves could provide a bad signal.
        let progress_bonus = 0.20;
        let quality_bonus = 0.65;
        let durability_bonus = 0.05;
        let cp_bonus = 0.05;
        let fewer_steps_bonus = 0.05;

        let progress_score = apply(
            progress_bonus,
            self.progress as f32,
            self.context.progress_target as f32,
        );

        let quality_score = apply(
            quality_bonus,
            self.quality as f32,
            self.context.quality_target as f32,
        );

        let durability_score = apply(
            durability_bonus,
            f32::from(self.durability),
            f32::from(self.context.durability_max),
        );

        let cp_score = apply(cp_bonus, self.cp as f32, self.context.cp_max as f32);

        let fewer_steps_score =
            fewer_steps_bonus * (1.0_f32 - f32::from(self.step) / f32::from(self.context.step_max));

        progress_score + quality_score + durability_score + cp_score + fewer_steps_score
    }

    /// Evaluates the craft based on step count since quality doesn't matter.
    /// Returns a value from 0 to 1.
    #[allow(clippy::cast_precision_loss)]
    pub fn score_no_quality(&self) -> f32 {
        1.0_f32 - f32::from(self.step) / f32::from(self.context.step_max)
    }

    pub fn check_result(&self) -> Option<CraftResult> {
        if self.progress >= self.context.progress_target {
            let score = if self.context.quality_target > 0 {
                self.score()
            } else {
                self.score_no_quality()
            };
            Some(CraftResult::Finished(score))
        } else if self.durability <= 0 {
            Some(CraftResult::DurabilityFailure)
        } else if self.step >= self.context.step_max {
            Some(CraftResult::MaxStepsFailure)
        } else if self.available_moves.is_empty() {
            Some(CraftResult::InvalidActionFailure)
        } else {
            None
        }
    }
}
