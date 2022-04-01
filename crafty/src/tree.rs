#[derive(Debug)]
pub struct Arena<T> {
    pub nodes: Vec<Node<T>>,
}

impl<T> Arena<T> {
    pub fn new(initial_state: T) -> Self {
        let initial_node = Node {
            parent: None,
            index: 0,
            children: vec![],
            state: initial_state,
        };
        Arena {
            nodes: vec![initial_node],
        }
    }

    pub fn insert(&mut self, parent_index: usize, state: T) -> usize {
        let index = self.nodes.len();
        let node = Node {
            parent: Some(parent_index),
            index,
            children: vec![],
            state,
        };
        self.get_mut(parent_index).unwrap().children.push(index);
        self.nodes.push(node);
        index
    }

    pub fn get(&self, index: usize) -> Option<&Node<T>> {
        self.nodes.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Node<T>> {
        self.nodes.get_mut(index)
    }
}

#[derive(Debug)]
pub struct Node<T> {
    pub parent: Option<usize>,
    pub index: usize,
    pub children: Vec<usize>,
    pub state: T,
}

#[cfg(test)]
mod tests {
    use super::Arena;

    #[test]
    fn starts_with_initial_node() {
        let arena = Arena::new("a");

        assert_eq!(arena.nodes.len(), 1);
        assert_eq!(arena.get(0).unwrap().state, "a");
    }

    #[test]
    fn inserts_into_arena_and_parent() {
        let mut arena = Arena::new("a");

        assert_eq!(arena.get(0).unwrap().children.len(), 0);

        let index_b = arena.insert(0, "b");

        assert_eq!(arena.nodes.len(), 2);
        assert_eq!(arena.get(index_b).unwrap().state, "b");
        assert_eq!(arena.get(0).unwrap().children.len(), 1);
    }
}
