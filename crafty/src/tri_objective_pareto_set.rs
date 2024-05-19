use std::collections::BTreeMap;

#[derive(Debug)]
struct ParetoItem(usize, [f32; 3]);

impl ParetoItem {
    fn index(&self) -> usize {
        self.0
    }

    fn x(&self) -> f32 {
        self.1[0]
    }

    fn y(&self) -> f32 {
        self.1[1]
    }

    fn z(&self) -> f32 {
        self.1[2]
    }
}

#[derive(Debug)]
pub struct TriObjectiveParetoSet {
    /// Balanced tree with key `y`
    inner: BTreeMap<f32, Vec<ParetoItem>>,
}

impl TriObjectiveParetoSet {
    fn update(&mut self, item: ParetoItem) -> bool {
        // 1) start at key `item.y()`
        // 2) move in direction `item.y()` descending
        // 3) remove items until `existing_item.z()` > `item.z()`
        // 4) if any items were removed, insert `item`

        let items_to_remove: _ = self.inner.iter().rev().filter_map(|(key, existing_items)| {
            let indices_to_remove: Vec<usize> = existing_items
                .iter()
                .enumerate()
                .flat_map(|(i, existing_item)| {
                    if existing_item.z() <= item.z() {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();

            if indices_to_remove.len() == existing_items.len() {
                (key, None)
            } else {
                (key, Some(indices_to_remove))
            }
        });
    }
}

impl From<Vec<ParetoItem>> for TriObjectiveParetoSet {
    fn from(vec: Vec<ParetoItem>) -> Self {
        vec.sort_unstable_by(|a, b| b.x().partial_cmp(&a.x()).unwrap());

        let mut set = Self {
            inner: BTreeMap::new(),
        };
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn it_works() {}
}
