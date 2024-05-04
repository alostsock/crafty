use enum_indexing::EnumIndexing;
use rand::{rngs::SmallRng, Rng};

use crate::Action;

#[derive(Debug, Default, Clone)]
pub struct ActionSet(u32);

impl ActionSet {
    #[allow(clippy::cast_possible_truncation)]
    fn bit_from_action(action: Action) -> u32 {
        1u32 << action.index()
    }

    fn set_bit(&mut self, bit: u32) {
        self.0 |= bit;
    }

    fn unset_bit(&mut self, bit: u32) {
        self.0 &= !bit;
    }

    pub fn set(&mut self, action: Action) {
        self.set_bit(Self::bit_from_action(action));
    }

    pub fn unset(&mut self, action: Action) {
        self.unset_bit(Self::bit_from_action(action));
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_vec(actions: &Vec<Action>) -> Self {
        let mut instance = Self::new();

        for action in actions {
            instance.set(*action);
        }

        instance
    }

    pub fn contains(&self, action: Action) -> bool {
        (self.0 & Self::bit_from_action(action)).count_ones() == 1
    }

    /// Iterates through Actions in the set and keeps or removes them based on
    /// the closure `f` provided.
    ///
    /// Similar to Vec's retain method.
    pub fn keep<F>(&mut self, mut f: F)
    where
        F: FnMut(&Action) -> bool,
    {
        let mut remaining_bits = self.0;

        while remaining_bits != 0 {
            let index = (32 - remaining_bits.leading_zeros() - 1) as usize;
            let action = Action::from_index(index).unwrap();
            let action_bit = 1u32 << index;

            if !f(&action) {
                self.unset_bit(action_bit);
            }

            remaining_bits &= !action_bit;
        }
    }

    fn random_index(&self, rng: &mut SmallRng) -> usize {
        // inspired by https://stackoverflow.com/a/37460774
        let mut nth = rng.gen_range(0..self.len());
        let mut remaining_bits = self.0;

        while remaining_bits != 0 {
            let index = (32 - remaining_bits.leading_zeros() - 1) as usize;

            if nth == 0 {
                return index;
            }

            let bit = 1u32 << index;

            nth -= 1;
            remaining_bits &= !bit;
        }

        panic!("called `random` on empty ActionSet");
    }

    /// Returns a random Action from the set
    pub fn sample(&self, rng: &mut SmallRng) -> Action {
        let random_index = self.random_index(rng);
        Action::from_index(random_index).unwrap()
    }

    /// Removes and returns a random Action from the set
    pub fn pick(&mut self, rng: &mut SmallRng) -> Action {
        let random_index = self.random_index(rng);
        self.unset_bit(1u32 << random_index);
        Action::from_index(random_index).unwrap()
    }

    pub fn len(&self) -> u32 {
        self.0.count_ones()
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn to_vec(&self) -> Vec<Action> {
        let mut actions = vec![];

        for action in Action::ACTIONS.iter() {
            if self.contains(*action) {
                actions.push(*action);
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use Action::*;

    #[test]
    fn set_and_unset_works() {
        let mut set = ActionSet::new();

        set.set(BasicTouch);
        set.set(BasicSynthesis);
        assert_eq!(set.len(), 2);

        set.unset(BasicTouch);
        set.unset(BasicSynthesis);
        assert!(set.is_empty());
    }

    #[test]
    fn keep_works() {
        let mut set = ActionSet::new();
        set.set(BasicTouch);
        set.set(BasicSynthesis);
        set.set(GreatStrides);
        set.set(MuscleMemory);

        set.keep(|action| *action != BasicTouch && *action != GreatStrides);
        assert_eq!(set.len(), 2);
        assert!(set.contains(BasicSynthesis));
        assert!(set.contains(MuscleMemory));
    }

    #[test]
    fn random_index_works() {
        let mut set = ActionSet::new();
        set.set(BasicTouch);
        set.set(BasicSynthesis);
        set.set(GreatStrides);
        set.set(TrainedFinesse);

        let mut counts = vec![0; Action::ACTIONS.len()];
        let mut rng = SmallRng::seed_from_u64(1);
        for _ in 0..100 {
            let random_index = set.random_index(&mut rng);

            assert!([
                BasicTouch.index(),
                BasicSynthesis.index(),
                GreatStrides.index(),
                TrainedFinesse.index(),
            ]
            .contains(&random_index));

            counts[random_index] += 1;
        }

        assert!(counts[BasicTouch.index()] > 0);
        assert!(counts[BasicSynthesis.index()] > 0);
        assert!(counts[GreatStrides.index()] > 0);
        assert!(counts[TrainedFinesse.index()] > 0);
    }
}
