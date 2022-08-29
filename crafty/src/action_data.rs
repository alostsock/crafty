use crate::{Action, CraftState};
use enum_indexing::EnumIndexing;

type BuffScores = [f32; 9];

#[derive(Debug, Default, Clone)]
struct ActionValue {
    pub buff_scores: BuffScores,
    pub visits: f32,
}

impl ActionValue {
    fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug)]
pub struct ActionData {
    inner: Vec<ActionValue>,
}

impl Default for ActionData {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionData {
    pub fn new() -> Self {
        let action_count = Action::count();
        Self {
            inner: vec![ActionValue::new(); action_count],
        }
    }

    /// For each action, track 1) sum of craft scores when using it and 2) the amount
    /// of times the action was used, in an ActionValue. The craft scores are tracked
    /// for each buff active in the state the action was used.
    ///
    /// When buff_scores is averaged for each buff, this should give a general indicator
    /// for the value of the action when specific buffs are active.
    pub fn record(&mut self, action: &Action, state: &CraftState, score: f32) {
        let action_index = action.index();

        let action_value = self.inner.get_mut(action_index).unwrap();

        let active_buffs = state.buffs.as_mask();

        for (buff_score, is_active) in action_value.buff_scores.iter_mut().zip(active_buffs) {
            if is_active {
                *buff_score += score;
            }
        }
        action_value.visits += 1.0;
    }

    /// Get a score for an `Action` given a `CraftState`. An action is weighted
    /// with a higher value if the buff scores for that action correlate closely
    /// with the buffs in the craft state.
    ///
    /// Using a sigmoid function `2 / (1 + 0.01^(-d))`, we can roughly convert
    /// `d` (a value from 0 to 1), to a `score` (a value from 1 to 0).
    pub fn score(&self, action: &Action, state: &CraftState) -> f32 {
        let active_buffs = state.buffs.as_mask();

        let ActionValue {
            buff_scores,
            visits,
        } = self.inner[action.index()];

        let avg_buff_scores: Vec<f32> = buff_scores
            .iter()
            .map(|score| if visits > 0.0 { score / visits } else { 0.0 })
            .collect();

        let distance = buff_distance(&active_buffs, &avg_buff_scores);

        2.0 / (1.0 + (0.01_f32.powf(-distance)))
    }
}

/// Compare currently active buffs to a recorded action's buff scores (i.e. the action's success
/// rate against each buff). Distance should be smaller if the buff score closely matches active buffs.
fn buff_distance(active_buffs: &[bool], buff_scores: &[f32]) -> f32 {
    active_buffs
        .iter()
        .zip(buff_scores)
        .map(|(&is_active, &score)| {
            if is_active {
                (1.0 - score).abs()
            } else {
                score
            }
        })
        .sum()
}
