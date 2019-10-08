use super::node::Node;
use super::BTree;

#[derive(Copy, Clone)]
struct TreeIterState<'a, T: Ord + Clone> {
    node: &'a Node<T>,
    pos: usize,
}

pub struct TreeIter<'a, T: Ord + Clone> {
    /// List of parent nodes and current child position in them
    tail_states: Vec<TreeIterState<'a, T>>,
    /// The current node and the next element position to return
    head_state: TreeIterState<'a, T>,
    len: usize,
}

impl<'a, T: Ord + Clone> TreeIter<'a, T> {
    pub(super) fn new(tree: &'a BTree<T>) -> Self {
        // Create initial state, by recursing into child at the bottom
        let mut iter = TreeIter {
            tail_states: vec![],
            head_state: TreeIterState {
                node: &tree.root,
                pos: 0,
            },
            len: tree.len(),
        };
        iter.prepare_state_from(&tree.root);
        iter
    }

    fn prepare_state_from(&mut self, mut node: &'a Node<T>) {
        self.head_state = TreeIterState { node, pos: 0 };
        while !node.is_leaf() {
            node = node.get_child(0);
            let next_state = TreeIterState { node, pos: 0 };
            self.tail_states
                .push(std::mem::replace(&mut self.head_state, next_state));
        }
    }
}

impl<'a, T: Ord + Clone> Iterator for TreeIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        let TreeIterState { node, pos } = self.head_state;
        if pos < node.len() {
            // Iterate in node
            let res = node.get_element(pos);
            self.head_state.pos += 1;
            if !node.is_leaf() {
                self.tail_states.push(self.head_state.clone());
                self.prepare_state_from(node.get_child(pos + 1));
            }
            self.len -= 1;
            Some(res)
        } else {
            // Walk over the chain
            match self.tail_states.pop() {
                None => None,
                Some(parent_state) => {
                    self.head_state = parent_state;
                    self.next()
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T: Ord + Clone> ExactSizeIterator for TreeIter<'a, T> {}
impl<'a, T: Ord + Clone> std::iter::FusedIterator for TreeIter<'a, T> {}
