use crate::{Action, CraftState};
use enum_indexing::EnumIndexing;

type BuffScores = [f32; 9];

#[derive(Debug, Default, Clone)]
struct ActionValue {
    buff_scores: BuffScores,
    visits: f32,
}

impl ActionValue {
    fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug)]
pub struct ActionValues(Vec<ActionValue>);

impl ActionValues {
    pub fn new() -> Self {
        let action_count = Action::count();
        Self(vec![ActionValue::new(); action_count])
    }

    /// Record which buffs were active for a state, scaled by the score of the craft.
    pub fn record(&mut self, state: &CraftState, score: f32) {
        let action_index = if let Some(action) = state.action {
            action.index()
        } else {
            return;
        };

        let action_value = self.0.get_mut(action_index).unwrap();

        let active_buffs = state.buffs.as_mask();

        for (buff_score, is_active) in action_value.buff_scores.iter_mut().zip(active_buffs) {
            if is_active {
                *buff_score += score;
            }
        }
        action_value.visits += 1.0;
    }

    /// Generate weights for each action. An action is weighted with a higher
    /// value if the buff scores for that action correlate closely to the buffs
    /// in the craft state.
    ///
    /// Weight calculation:
    /// `1 - distance(current, recorded) / max_distance`
    pub fn generate_weights(&self, state: &CraftState) -> Vec<f32> {
        let current_buffs = state.buffs.as_mask();

        let mut distances: Vec<f32> = vec![];
        let mut max_distance: f32 = 0.0;

        for action in state.available_moves.iter() {
            let ActionValue {
                buff_scores,
                visits,
            } = self.0[action.index()];

            let avg_buff_scores: Vec<f32> = buff_scores
                .iter()
                .map(|score| if visits > 0.0 { score / visits } else { 0.0 })
                .collect();
            let distance = buff_distance(&current_buffs, &avg_buff_scores);
            distances.push(distance);

            if distance > max_distance {
                max_distance = distance;
            }
        }

        if max_distance == 0.0 {
            return distances.iter().map(|_| 1.0).collect();
        } else {
            return distances
                .iter()
                .map(|&d| 1.0 - 0.99 * d / max_distance)
                .collect();
        }
    }
}

/// Compare currently active buffs to a recorded action's buff scores (i.e. the action's success
/// rate against each buff). Distance should be lower if the buff score closely matches active buffs.
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
