use crate::action::{calc_cp_cost, calc_durability_cost, Action, ACTIONS};
use rand::Rng;
use std::fmt;

#[derive(Debug)]
pub enum CraftResult {
    /// Reached 100% progress. Include a score based on quality and steps
    Finished(f64),
    /// Failed either because of durability loss or the step limit was reached
    Failed,
}

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

    pub fn decrement_timers(&mut self) {
        self.waste_not = self.waste_not.saturating_sub(1);
        self.waste_not_ii = self.waste_not_ii.saturating_sub(1);
        self.manipulation = self.manipulation.saturating_sub(1);
        self.great_strides = self.great_strides.saturating_sub(1);
        self.innovation = self.innovation.saturating_sub(1);
        self.veneration = self.veneration.saturating_sub(1);
        self.makers_mark = self.makers_mark.saturating_sub(1);
        self.muscle_memory = self.muscle_memory.saturating_sub(1);
    }
}

#[derive(Debug, Clone)]
pub struct CraftState {
    /// Multiply by synthesis action efficiency for increase in progress
    pub progress_factor: f32,
    /// Multiply by touch action efficiency for increase in quality
    pub quality_factor: f32,
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

    pub observe: bool,
    pub next_combo: Option<Action>,
    pub buffs: Buffs,

    /// The action that led to this state
    pub action: Option<Action>,
    /// The probability that this state will occur (action chance * condition chance)
    pub prior: f64,
    /// Sum of scores from this node onward
    pub score_sum: f64,
    /// Number of times this node has been visited
    pub visits: f64,
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
        durability: u32,
        cp: u32,
    ) -> Self {
        let mut state = Self {
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
            observe: false,
            next_combo: None,
            buffs: Buffs::new(),
            action: None,
            prior: 1.0,
            score_sum: 0.0,
            visits: 0.0,
            available_moves: vec![],
        };

        state.determine_possible_moves();
        state
    }

    /// Prune as many moves as possible to reduce the search space
    pub fn determine_possible_moves(&mut self) {
        let mut available_moves = ACTIONS.to_vec();
        available_moves.retain(|action| {
            let attrs = action.attributes();

            if let Some(base_cost) = attrs.cp_cost {
                if calc_cp_cost(self, base_cost) > self.cp {
                    return false;
                }
            }

            use Action::*;
            match action {
                MuscleMemory | Reflect => self.step == 1,
                ByregotsBlessing => self.buffs.inner_quiet > 0,
                TrainedFinesse => self.buffs.inner_quiet == 10,
                PrudentSynthesis | PrudentTouch => {
                    self.buffs.waste_not == 0 && self.buffs.waste_not_ii == 0
                }
                // don't allow observe if observing
                Observe => !self.observe,
                // only allow focused skills if observing
                FocusedSynthesis | FocusedTouch => self.observe,
                // don't allow downgraded groundwork
                Groundwork => {
                    let cost = calc_durability_cost(self, attrs.durability_cost.unwrap());
                    self.durability >= cost
                }
                _ => true,
            }
        });
        self.available_moves = available_moves;
    }

    pub fn score(&self) -> f64 {
        // cap quality at 1.0
        let quality_ratio = 1.0_f64.min((self.quality / self.quality_target) as f64);
        // the lower the step count, the better
        quality_ratio - (self.step as f64 / 10.0)
    }

    pub fn check_result(&self) -> Option<CraftResult> {
        if self.progress >= self.progress_target {
            Some(CraftResult::Finished(self.score()))
        } else if self.durability == 0 {
            Some(CraftResult::Failed)
        } else {
            None
        }
    }

    pub fn execute_action(&mut self, action: Action) -> Self {
        let picked_index = self
            .available_moves
            .iter()
            .position(|&m| m == action)
            .unwrap();
        let action = self.available_moves.swap_remove(picked_index);
        action.execute(self)
    }

    pub fn execute_random_action(&mut self) -> Self {
        let mut rng = rand::thread_rng();
        let random_index = rng.gen_range(0..self.available_moves.len());
        let random_action = self.available_moves.swap_remove(random_index);
        random_action.execute(self)
    }
}
