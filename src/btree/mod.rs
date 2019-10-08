mod node;
mod tree;
mod tree_iter;

pub use tree::BTree;
pub use tree_iter::TreeIter;

// This implementation assumes the capacity is odd
const CAPACITY: usize = 11;

pub struct InsertionPoint<'a, T> {
    pub left: Option<&'a mut T>,
    pub right: Option<&'a mut T>,
}

enum TryInsertResult<T: Ord + Clone> {
    NothingInserted,
    Inserted(InsertResult<T>),
}

enum InsertResult<T: Ord + Clone> {
    Inserted,
    PendingSplit(T, node::Node<T>),
}
