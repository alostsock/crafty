#[derive(Debug)]
pub struct Backtracker<T> {
    nodes: Vec<Node<T>>,
}

impl<T: Copy> Backtracker<T> {
    pub fn new() -> Self {
        Self { nodes: vec![] }
    }

    pub fn push(&mut self, parent: Option<usize>, item: T) -> usize {
        let index = self.nodes.len();
        self.nodes.push(Node { parent, item });
        index
    }

    pub fn backtrack(&self, index: usize) -> Vec<T> {
        let mut items = vec![];
        let mut next_index = Some(index);
        while let Some(node) = next_index.and_then(|i| self.nodes.get(i)) {
            items.push(node.item);
            next_index = node.parent;
        }

        items.reverse();
        items
    }
}

#[derive(Debug)]
pub struct Node<T> {
    pub parent: Option<usize>,
    pub item: T,
}
