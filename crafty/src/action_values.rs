use crate::{Action, CraftState};
use enum_indexing::EnumIndexing;

type BuffScores = [f32; 9];

#[derive(Debug)]
pub struct ActionValues {
    /// Buff scores for each action
    pub buff_scores_by_action: Vec<BuffScores>,
    /// Number of times each action was used
    pub visits_by_action: Vec<f32>,
}

impl ActionValues {
    pub fn new() -> Self {
        let action_count = Action::count();
        Self {
            buff_scores_by_action: vec![[0f32; 9]; action_count],
            visits_by_action: vec![0.0; action_count],
        }
    }

    /// Record which buffs were active for a state, scaled by the score of the craft.
    pub fn record(&mut self, state: &CraftState, score: f32) {
        let active_buffs = state.buffs.as_mask();
        let action_index = if let Some(action) = state.action {
            action.index()
        } else {
            return;
        };

        let buff_scores = self.buff_scores_by_action.get_mut(action_index).unwrap();
        for (buff_score, is_active) in buff_scores.iter_mut().zip(active_buffs) {
            if is_active {
                *buff_score += score;
            }
        }

        self.visits_by_action[action_index] += 1.0;
    }

    /// Average buff score per visit for each action
    fn average_scores(&self) -> Vec<Vec<f32>> {
        self.buff_scores_by_action
            .iter()
            .zip(&self.visits_by_action)
            .map(|(buff_scores, visits)| {
                buff_scores
                    .iter()
                    .map(|score| if *visits > 0.0 { score / visits } else { 0.0 })
                    .collect::<Vec<f32>>()
            })
            .collect::<Vec<Vec<f32>>>()
    }

    /// Generate weights for each action based on which buffs are active.
    /// Weight calculation: `1 - distance(current, recorded) / max_distance`
    pub fn generate_weights(&self, state: &CraftState) -> Vec<f32> {
        let current_buffs: Vec<f32> = state
            .buffs
            .as_mask()
            .iter()
            .map(|b| if *b { 1.0 } else { 0.0 })
            .collect();
        let all_scores = self.average_scores();
        // retain only available actions
        let available_scores: Vec<Vec<f32>> = state
            .available_moves
            .iter()
            .map(|action| all_scores[action.index()].clone())
            .collect();

        let mut max_distance: f32 = 0.01;
        let distances: Vec<f32> = available_scores
            .iter()
            .map(|buff_scores| {
                let d = distance(buff_scores, &current_buffs);
                if d > max_distance {
                    max_distance = d;
                }
                d
            })
            .collect();

        distances
            .iter()
            .map(|d| 1.0 - 0.99 * (d / max_distance))
            .collect::<Vec<f32>>()
    }
}

/// Manhattan distance
fn distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| (*a - b).abs()).sum()
}
