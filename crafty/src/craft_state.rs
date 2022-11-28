use crate::{action::Attributes, data, Action, Player, Recipe};
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
    pub makers_mark: u8,
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
    #[allow(clippy::cast_precision_loss)]
    fn factors(player: &Player, recipe: &Recipe) -> (f32, f32) {
        // https://github.com/ffxiv-teamcraft/simulator/blob/72f4a6037baa3cd7cd78dfe34207283b824881a2/src/model/actions/crafting-action.ts#L176

        let progress_div = recipe.progress_div as f32;
        let mut progress_factor: f32 = (player.craftsmanship * 10) as f32 / progress_div + 2.0;

        let quality_div = recipe.quality_div as f32;
        let mut quality_factor: f32 = (player.control * 10) as f32 / quality_div + 35.0;

        if let Some(&base_recipe_level) = data::base_recipe_level(player.job_level) {
            if base_recipe_level <= recipe.recipe_level {
                progress_factor *= recipe.progress_mod as f32 / 100.0;
                quality_factor *= recipe.quality_mod as f32 / 100.0;
            }
        }

        (progress_factor.floor(), quality_factor.floor())
    }

    fn _new(player: &Player, recipe: &Recipe, max_steps: u8) -> Self {
        let (progress_factor, quality_factor) = Self::factors(player, recipe);
        Self {
            progress_factor,
            quality_factor,
            step: 1,
            step_max: max_steps,
            progress: 0,
            progress_target: recipe.progress,
            quality: 0,
            quality_target: recipe.quality,
            durability: recipe.durability,
            durability_max: recipe.durability,
            cp: player.cp,
            cp_max: player.cp,
            observe: false,
            next_combo_action: None,
            buffs: Buffs::new(),
            action: None,
            score_sum: 0.0,
            max_score: 0.0,
            visits: 0.0,
            available_moves: vec![],
        }
    }

    pub fn new(player: &Player, recipe: &Recipe, max_steps: u8) -> Self {
        let mut state = Self::_new(player, recipe, max_steps);
        state.set_available_moves(false);
        state
    }

    pub fn new_strict(player: &Player, recipe: &Recipe, max_steps: u8) -> Self {
        let mut state = Self::_new(player, recipe, max_steps);
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
    fn set_available_moves(&mut self, strict: bool) -> &mut Self {
        if self.progress >= self.progress_target
            || self.step >= self.step_max
            || self.durability <= 0
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

    // interesting lint, but passing by value apparently results in a 2-3% performance regression?
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn _execute(&self, action: &Action) -> Self {
        let mut state = Self {
            step: self.step + 1,
            buffs: self.buffs.clone(),
            action: Some(*action),
            score_sum: 0.0,
            max_score: 0.0,
            visits: 0.0,
            available_moves: vec![],
            ..*self
        };

        let Attributes {
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
            if action == &Action::ByregotsBlessing {
                state.buffs.inner_quiet = 0;
            } else {
                state.buffs.inner_quiet = cmp::min(state.buffs.inner_quiet + 1, 10);
            }
            state.buffs.great_strides = 0;
        }

        if let Some(base_cost) = durability_cost {
            state.durability -= Action::calc_durability_cost(&state, base_cost);
        }

        if state.buffs.manipulation > 0 && state.durability > 0 {
            state.durability = cmp::min(state.durability + 5, state.durability_max);
        }

        if let Some(base_cost) = cp_cost {
            state.cp -= Action::calc_cp_cost(&state, base_cost);
        }

        state.observe = false;

        if state.next_combo_action != Some(*action) {
            state.next_combo_action = None;
        }

        state.buffs.decrement_timers();

        // Always apply effects last
        if let Some(apply_effect) = effect {
            apply_effect(&mut state);
        }

        state
    }

    /// Executes the action against a `CraftState`, and returns a `CraftState` with
    /// all available moves
    pub fn execute(&self, action: &Action) -> CraftState {
        let mut state = self._execute(action);
        state.set_available_moves(false);
        state
    }

    /// Executes the action against a `CraftState`, and returns a `CraftState` with
    /// a strict, pruned moveset
    pub fn execute_strict(&self, action: &Action) -> CraftState {
        let mut state = self._execute(action);
        state.set_available_moves(true);
        state
    }

    /// An evaluation of the craft. Returns a value from 0 to 1.
    #[allow(clippy::cast_precision_loss)]
    pub fn score(&self) -> f32 {
        // bonuses should add up to 1.0
        let quality_bonus: f32 = 0.995;
        let fewer_steps_bonus: f32 = 0.005;

        if self.progress >= self.progress_target {
            let quality_score =
                quality_bonus.min(quality_bonus * self.quality as f32 / self.quality_target as f32);

            let fewer_steps_score =
                fewer_steps_bonus * (1.0_f32 - f32::from(self.step) / f32::from(self.step_max));

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
            Some(CraftResult::InvalidActionFailure)
        } else {
            None
        }
    }
}
