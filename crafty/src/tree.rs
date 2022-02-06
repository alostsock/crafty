use anyhow::{Context, Result};

pub struct Arena<T> {
    nodes: Vec<Node<T>>,
}

#[allow(dead_code)]
impl<T> Arena<T> {
    pub fn new(initial_value: T) -> Self {
        let initial_node = Node {
            parent: None,
            index: 0,
            children: vec![],
            value: initial_value,
        };
        Arena {
            nodes: vec![initial_node],
        }
    }

    pub fn insert(&mut self, parent_index: usize, value: T) -> Result<usize> {
        let index = self.nodes.len();
        let node = Node {
            parent: Some(parent_index),
            index,
            children: vec![],
            value,
        };
        self.get_mut(parent_index)?.children.push(index);
        self.nodes.push(node);
        Ok(index)
    }

    pub fn get(&self, index: usize) -> Result<&Node<T>> {
        self.nodes
            .get(index)
            .context(format!("no node with index {}", index))
    }

    pub fn get_mut(&mut self, index: usize) -> Result<&mut Node<T>> {
        self.nodes
            .get_mut(index)
            .context(format!("no node with index {}", index))
    }
}

pub struct Node<T> {
    pub parent: Option<usize>,
    pub index: usize,
    pub children: Vec<usize>,
    pub value: T,
}

#[cfg(test)]
mod tests {
    use super::Arena;

    #[test]
    fn starts_with_initial_node() {
        let arena = Arena::new("a");

        assert_eq!(arena.nodes.len(), 1);
        assert_eq!(arena.get(0).unwrap().value, "a");
    }

    #[test]
    fn inserts_into_arena_and_parent() {
        let mut arena = Arena::new("a");

        assert_eq!(arena.get(0).unwrap().children.len(), 0);

        let index_b = arena.insert(0, "b").unwrap();

        assert_eq!(arena.nodes.len(), 2);
        assert_eq!(arena.get(index_b).unwrap().value, "b");
        assert_eq!(arena.get(0).unwrap().children.len(), 1);
    }
}
