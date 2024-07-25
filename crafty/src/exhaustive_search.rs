use std::{cmp::Ordering, collections::BinaryHeap};

use crate::{backtracker::Backtracker, Action, CraftResult, CraftState};
use ahash::AHashMap;
use pareto_front::{Dominate, ParetoFront};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct FinishableState {
    progress: u32,
    durability: i8,
    cp: u16,
    trained_perfection_active: Option<bool>,
    waste_not: u8,
    manipulation: u8,
    veneration: u8,
    muscle_memory: u8,
}

impl FinishableState {
    fn from_state(state: &CraftState) -> Self {
        Self {
            progress: state.progress,
            durability: state.durability,
            cp: state.cp,
            trained_perfection_active: state.trained_perfection_active,
            waste_not: state.buffs.waste_not.max(state.buffs.waste_not_ii),
            manipulation: state.buffs.manipulation,
            veneration: state.buffs.veneration,
            muscle_memory: state.buffs.muscle_memory,
        }
    }
}

impl Dominate for FinishableState {
    /// Used for determining the lower bound for finishing.
    /// `self` only dominates `x` if it has less resources.
    fn dominate(&self, x: &Self) -> bool {
        self.progress <= x.progress
            && self.durability <= x.durability
            && self.cp <= x.cp
            && self.muscle_memory <= x.muscle_memory
            && self.manipulation <= x.manipulation
            && self.veneration <= x.veneration
            && self.waste_not <= x.waste_not
            && (
                // states match
                self.trained_perfection_active == x.trained_perfection_active
                // `self` dominates if it's already used Trained Perfection
                || self.trained_perfection_active.is_some() && x.trained_perfection_active.is_none()
                // `self` dominates if it's used Trained Perfection, and the buff is inactive
                || self.trained_perfection_active == Some(false) && x.trained_perfection_active == Some(true)
            )
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct HqableState {
    quality: u32,
    durability: i8,
    cp: u16,
    previous_combo_action: Option<Action>,
    quick_innovation_available: bool,
    trained_perfection_active: Option<bool>,
    inner_quiet: u8,
    waste_not: u8,
    manipulation: u8,
    great_strides: u8,
    innovation: u8,
}

impl HqableState {
    fn from_state(state: &CraftState) -> Self {
        Self {
            quality: state.quality,
            durability: state.durability,
            cp: state.cp,
            previous_combo_action: state.previous_combo_action,
            quick_innovation_available: state.quick_innovation_available,
            trained_perfection_active: state.trained_perfection_active,
            inner_quiet: state.buffs.inner_quiet,
            waste_not: state.buffs.waste_not.max(state.buffs.waste_not_ii),
            manipulation: state.buffs.manipulation,
            great_strides: state.buffs.great_strides,
            innovation: state.buffs.innovation,
        }
    }
}

impl Dominate for HqableState {
    /// Used for determining the lower bound for max quality.
    /// `self` only dominates `x` if it has less resources available.
    fn dominate(&self, x: &Self) -> bool {
        self.quality <= x.quality
            && self.durability <= x.durability
            && self.cp <= x.cp
            && self.inner_quiet <= x.inner_quiet
            && self.waste_not <= x.waste_not
            && self.manipulation <= x.manipulation
            && self.great_strides <= x.great_strides
            && self.innovation <= x.innovation
            && (self.previous_combo_action == x.previous_combo_action
                // `self` is better if it doesn't have a combo action ready
                || self.previous_combo_action.is_none() && x.previous_combo_action.is_some())
            && (self.trained_perfection_active == x.trained_perfection_active
                // `self` is better if Trained Perfection has already been used
                || self.trained_perfection_active.is_some() && x.trained_perfection_active.is_none()
                || self.trained_perfection_active == Some(false) && x.trained_perfection_active == Some(true))
            && (self.quick_innovation_available == x.quick_innovation_available
                // `self` is better if Quick Innovation has already been used
                || self.quick_innovation_available == false && x.quick_innovation_available == true)
    }
}

#[derive(Debug, Hash)]
pub struct ReducedState {
    progress: u32,
    quality: u32,
    durability: i8,
    cp: u16,
    previous_combo_action: Option<Action>,
    quick_innovation_available: bool,
    trained_perfection_active: Option<bool>,
    inner_quiet: u8,
    waste_not: u8,
    manipulation: u8,
    great_strides: u8,
    innovation: u8,
    veneration: u8,
    muscle_memory: u8,
}

impl ReducedState {
    fn from_state(state: &CraftState) -> Self {
        Self {
            progress: state.progress,
            quality: state.quality,
            durability: state.durability,
            cp: state.cp,
            previous_combo_action: state.previous_combo_action,
            quick_innovation_available: state.quick_innovation_available,
            trained_perfection_active: state.trained_perfection_active,
            inner_quiet: state.buffs.inner_quiet,
            waste_not: state.buffs.waste_not.max(state.buffs.waste_not_ii),
            manipulation: state.buffs.manipulation,
            great_strides: state.buffs.great_strides,
            innovation: state.buffs.innovation,
            veneration: state.buffs.veneration,
            muscle_memory: state.buffs.muscle_memory,
        }
    }
}

impl Dominate for ReducedState {
    /// Used for keeping track of optimal states.
    fn dominate(&self, x: &Self) -> bool {
        self.progress >= x.progress
            && self.quality >= x.quality
            && self.durability >= x.durability
            && self.cp >= x.cp
            && self.inner_quiet >= x.inner_quiet
            && self.waste_not >= x.waste_not
            && self.manipulation >= x.manipulation
            && self.great_strides >= x.great_strides
            && self.innovation >= x.innovation
            && self.veneration >= x.veneration
            && self.muscle_memory >= x.muscle_memory
            && (self.previous_combo_action == x.previous_combo_action
                || self.previous_combo_action.is_some() && x.previous_combo_action.is_none())
            && (self.trained_perfection_active == x.trained_perfection_active
                // trained perfection hasn't been used yet
                || self.trained_perfection_active.is_none() && x.trained_perfection_active.is_some()
                // trained perfection was used, and is still active
                || self.trained_perfection_active == Some(true) && x.trained_perfection_active == Some(false))
            && (self.quick_innovation_available == x.quick_innovation_available
                || self.quick_innovation_available == true && x.quick_innovation_available == false)
    }
}

struct QueuedState<'a> {
    state: CraftState<'a>,
    parent_index: Option<usize>,
}

impl Ord for QueuedState<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        let quality_target = self.state.context.quality_target;
        let self_quality = self.state.quality.min(quality_target);
        let other_quality = other.state.quality.min(quality_target);
        self_quality.cmp(&other_quality)
    }
}

impl PartialOrd for QueuedState<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for QueuedState<'_> {}

impl PartialEq for QueuedState<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.state.quality.min(self.state.context.quality_target)
            == other.state.quality.min(self.state.context.quality_target)
    }
}

pub struct Solution {
    score: f32,
    backtracker_index: Option<usize>,
}

#[derive(Default, Debug)]
struct Stats {
    queued_states_visited: usize,
    finishable_lower_bound_count: usize,
    finishable_lower_bound_count_max: usize,
    finishable_states_count: usize,
    finishable_states_hits: usize,
    finishable_states_misses: usize,
    finishable_rejections: usize,
    finishable_inner_rejections: usize,
    hqable_lower_bound_count: usize,
    hqable_lower_bound_count_max: usize,
    hqable_states_count: usize,
    hqable_states_hits: usize,
    hqable_states_misses: usize,
    hqable_rejections: usize,
    visited_upper_bound_count: usize,
    visited_upper_bound_count_max: usize,
    visited_upper_bound_rejections: usize,
    dead_ends: usize,
}

pub struct ExhaustiveSearch<'a> {
    backtracker: Backtracker<Action>,
    best_solution: Solution,
    queue: BinaryHeap<QueuedState<'a>>,
    finishable_lower_bound: ParetoFront<FinishableState>,
    checked_finishable_states: AHashMap<FinishableState, bool>,
    hqable_lower_bound: ParetoFront<HqableState>,
    checked_hqable_states: AHashMap<HqableState, bool>,
    visited_upper_bound: ParetoFront<ReducedState>,
    stats: Stats,
}

impl<'a> ExhaustiveSearch<'a> {
    pub fn new(initial_state: CraftState<'a>) -> Self {
        let mut queue = BinaryHeap::new();
        queue.push(QueuedState {
            state: initial_state,
            parent_index: None,
        });

        let best_solution = Solution {
            score: 0.0,
            backtracker_index: None,
        };

        Self {
            backtracker: Backtracker::new(),
            queue,
            best_solution,
            finishable_lower_bound: ParetoFront::new(),
            checked_finishable_states: AHashMap::new(),
            hqable_lower_bound: ParetoFront::new(),
            checked_hqable_states: AHashMap::new(),
            visited_upper_bound: ParetoFront::new(),
            stats: Stats::default(),
        }
    }

    pub fn search(&mut self) -> Option<Vec<Action>> {
        while let Some(QueuedState {
            state,
            parent_index,
        }) = self.queue.pop()
        {
            self.stats.queued_states_visited += 1;

            if !self.check_finishable_lower_bound(&state) {
                self.stats.finishable_rejections += 1;
                continue;
            }

            if !self.check_hqable_lower_bound(&state) {
                self.stats.hqable_rejections += 1;
                continue;
            }

            if !self.check_is_upper_bound(&state) {
                continue;
            }

            for action in state.available_moves.iter() {
                let backtracker_index = Some(self.backtracker.push(parent_index, action));
                let child_state = state.execute_semistrict(&action);
                match child_state.check_result_simple() {
                    Some(CraftResult::Finished(score)) => {
                        if score > self.best_solution.score {
                            self.best_solution = Solution {
                                score,
                                backtracker_index,
                            };
                        }
                    }
                    Some(_) => {
                        self.stats.dead_ends += 1;
                    }
                    _ => {
                        self.queue.push(QueuedState {
                            state: child_state,
                            parent_index: backtracker_index,
                        });
                    }
                }
            }
        }

        self.stats.finishable_lower_bound_count = self.finishable_lower_bound.len();
        self.stats.finishable_states_count = self.checked_finishable_states.len();
        self.stats.hqable_lower_bound_count = self.hqable_lower_bound.len();
        self.stats.hqable_states_count = self.checked_finishable_states.len();
        self.stats.visited_upper_bound_count = self.visited_upper_bound.len();

        dbg!(&self.stats);

        self.get_solution()
    }

    fn check_finishable_lower_bound(&mut self, state: &CraftState) -> bool {
        match state.check_result_partial(true) {
            Some(CraftResult::Finished(_)) => return true,
            Some(_) => return false,
            _ => (),
        }

        let candidate = FinishableState::from_state(state);

        if let Some(&finishable) = self.checked_finishable_states.get(&candidate) {
            self.stats.finishable_states_hits += 1;
            return finishable;
        }

        self.stats.finishable_states_misses += 1;

        let finishable = {
            if self
                .finishable_lower_bound
                .iter()
                .any(|state| state.dominate(&candidate))
            {
                true
            } else if state.get_progress_moves().iter().any(|action| {
                let next_state = state.execute_semistrict(&action);
                self.check_finishable_lower_bound(&next_state)
            }) {
                self.finishable_lower_bound.push(candidate.clone());
                self.stats.finishable_lower_bound_count_max = self
                    .finishable_lower_bound
                    .len()
                    .max(self.stats.finishable_lower_bound_count_max);
                true
            } else {
                false
            }
        };

        self.checked_finishable_states.insert(candidate, finishable);
        finishable
    }

    fn check_hqable_lower_bound(&mut self, state: &CraftState) -> bool {
        match state.check_result_partial(false) {
            Some(CraftResult::Finished(_)) => return true,
            Some(_) => return false,
            _ => (),
        }

        if !self.check_finishable_lower_bound(state) {
            self.stats.finishable_inner_rejections += 1;
            return false;
        }

        let candidate = HqableState::from_state(state);

        if let Some(&hqable) = self.checked_hqable_states.get(&candidate) {
            self.stats.hqable_states_hits += 1;
            return hqable;
        }

        self.stats.hqable_states_misses += 1;

        let hqable = {
            if self
                .hqable_lower_bound
                .iter()
                .any(|state| state.dominate(&candidate))
            {
                true
            } else if state.get_quality_moves().iter().any(|action| {
                let next_state = state.execute_semistrict(&action);
                self.check_hqable_lower_bound(&next_state)
            }) {
                self.hqable_lower_bound.push(candidate.clone());
                self.stats.hqable_lower_bound_count_max = self
                    .hqable_lower_bound
                    .len()
                    .max(self.stats.hqable_lower_bound_count_max);
                true
            } else {
                false
            }
        };

        self.checked_hqable_states.insert(candidate, hqable);
        hqable
    }

    fn check_is_upper_bound(&mut self, state: &CraftState) -> bool {
        let candidate = ReducedState::from_state(state);
        if self.visited_upper_bound.push(candidate) {
            self.stats.visited_upper_bound_count_max = self
                .visited_upper_bound
                .len()
                .max(self.stats.visited_upper_bound_count_max);
            true
        } else {
            self.stats.visited_upper_bound_rejections += 1;
            false
        }
    }

    fn get_solution(&self) -> Option<Vec<Action>> {
        if let Some(index) = self.best_solution.backtracker_index {
            Some(self.backtracker.backtrack(index))
        } else {
            None
        }
    }
}
