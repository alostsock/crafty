use ordered_float::OrderedFloat;
use std::collections::BTreeMap;
use std::ops::Bound;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParetoItem(usize, [OrderedFloat<f32>; 3]);

impl ParetoItem {
    pub fn new(index: usize, floats: [f32; 3]) -> Self {
        Self(
            index,
            [
                OrderedFloat(floats[0]),
                OrderedFloat(floats[1]),
                OrderedFloat(floats[2]),
            ],
        )
    }

    pub fn index(&self) -> usize {
        self.0
    }

    pub fn x(&self) -> OrderedFloat<f32> {
        self.1[0]
    }

    pub fn y(&self) -> OrderedFloat<f32> {
        self.1[1]
    }

    pub fn z(&self) -> OrderedFloat<f32> {
        self.1[2]
    }
}

#[derive(Debug)]
pub struct TriObjectiveParetoSet {
    /// Balanced tree with key `y`
    inner: BTreeMap<OrderedFloat<f32>, Vec<ParetoItem>>,
}

impl TriObjectiveParetoSet {
    pub fn items(&self) -> Vec<&ParetoItem> {
        self.inner.values().flatten().collect()
    }
}

impl From<Vec<ParetoItem>> for TriObjectiveParetoSet {
    fn from(mut items: Vec<ParetoItem>) -> Self {
        // This obtains a Pareto optimal set for a collection of items in 3 dimensions,
        // in O(n log n) time
        //
        // From "Computational Geometry: An Introduction",
        // Section 4.1.3: The problem of the maxima of a point set
        //
        // 1) Sort items by X descending
        // 2) Maintain a balanced binary tree keyed on Y. For each `item` by X descending, decide
        //    whether we should add it to the tree:
        //   3) Starting at `item.y()` inclusive, traverse the tree in order of Y descending
        //   4) Remove any existing items dominated by `item`.
        //   5) Insert `item` if it is not dominated by existing items.

        items.sort_unstable_by(|a, b| b.x().cmp(&a.x()));

        // A map keyed on Y, where values are `ParetoItem`s with equivalent Z values
        let mut map: BTreeMap<OrderedFloat<f32>, Vec<ParetoItem>> = BTreeMap::new();

        for item in items.into_iter() {
            let mut cursor = map.lower_bound_mut(Bound::Included(&item.y()));
            cursor.next();

            if cursor.peek_prev().is_none() {
                map.insert(item.y(), vec![item]);
                continue;
            }

            let mut item_dominated = false;

            while let Some((_, prev_items)) = cursor.peek_prev() {
                prev_items.retain(|prev| {
                    if item.1 == prev.1 {
                        true
                    } else if item.x() >= prev.x() && item.y() >= prev.y() && item.z() >= prev.z() {
                        false
                    } else if prev.x() >= item.x() && prev.y() >= item.y() && prev.z() >= item.z() {
                        item_dominated = true;
                        true
                    } else {
                        true
                    }
                });

                if prev_items.is_empty() {
                    cursor.remove_prev();
                } else {
                    cursor.prev();
                }
            }

            if !item_dominated {
                map.entry(item.y())
                    .and_modify(|existing_items| existing_items.push(item))
                    .or_insert(vec![item]);
            }
        }

        Self { inner: map }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn two_dimensions_xy() {
        let items: Vec<ParetoItem> = vec![
            ParetoItem::new(0, [0.2, 0.8, 0.0]),
            ParetoItem::new(1, [0.4, 0.6, 0.0]),
            ParetoItem::new(2, [0.6, 0.4, 0.0]),
            ParetoItem::new(3, [0.8, 0.2, 0.0]),
            ParetoItem::new(4, [1.0, 0.0, 0.0]),
            ParetoItem::new(5, [0.1, 0.1, 0.0]),
            ParetoItem::new(6, [0.2, 0.4, 0.0]),
            ParetoItem::new(7, [0.5, 0.4, 0.0]),
            ParetoItem::new(8, [0.9, 0.1, 0.0]),
            ParetoItem::new(9, [0.1, 0.8, 0.0]),
            ParetoItem::new(10, [0.6, 0.3, 0.0]),
        ];

        let set = TriObjectiveParetoSet::from(items);
        let mut set_items = set.items();
        set_items.sort_unstable_by_key(|i| i.index());
        assert_eq!(
            set_items.iter().map(|i| i.index()).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 8]
        )
    }

    #[test]
    fn two_dimensions_yz() {
        let items: Vec<ParetoItem> = vec![
            ParetoItem::new(0, [0.0, 0.2, 0.8]),
            ParetoItem::new(1, [0.0, 0.4, 0.6]),
            ParetoItem::new(2, [0.0, 0.6, 0.4]),
            ParetoItem::new(3, [0.0, 0.8, 0.2]),
            ParetoItem::new(4, [0.0, 1.0, 0.0]),
            ParetoItem::new(5, [0.0, 0.1, 0.1]),
            ParetoItem::new(6, [0.0, 0.2, 0.4]),
            ParetoItem::new(7, [0.0, 0.5, 0.4]),
            ParetoItem::new(8, [0.0, 0.9, 0.1]),
            ParetoItem::new(9, [0.0, 0.1, 0.8]),
            ParetoItem::new(10, [0.0, 0.6, 0.3]),
        ];

        let set = TriObjectiveParetoSet::from(items);
        let mut set_items = set.items();
        set_items.sort_unstable_by_key(|i| i.index());
        assert_eq!(
            set_items.iter().map(|i| i.index()).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 8]
        )
    }

    #[test]
    fn two_dimensions_xz() {
        let items: Vec<ParetoItem> = vec![
            ParetoItem::new(0, [0.2, 0.0, 0.8]),
            ParetoItem::new(1, [0.4, 0.0, 0.6]),
            ParetoItem::new(2, [0.6, 0.0, 0.4]),
            ParetoItem::new(3, [0.8, 0.0, 0.2]),
            ParetoItem::new(4, [1.0, 0.0, 0.0]),
            ParetoItem::new(5, [0.1, 0.0, 0.1]),
            ParetoItem::new(6, [0.2, 0.0, 0.4]),
            ParetoItem::new(7, [0.5, 0.0, 0.4]),
            ParetoItem::new(8, [0.9, 0.0, 0.1]),
            ParetoItem::new(9, [0.1, 0.0, 0.8]),
            ParetoItem::new(10, [0.6, 0.0, 0.3]),
        ];

        let set = TriObjectiveParetoSet::from(items);
        let mut set_items = set.items();
        set_items.sort_unstable_by_key(|i| i.index());
        assert_eq!(
            set_items.iter().map(|i| i.index()).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 8]
        )
    }
}
