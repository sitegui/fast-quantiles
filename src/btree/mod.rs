mod node;
mod tree;

pub use tree::BTree;

// This implementation assumes the capacity is odd
const CAPACITY: usize = 11;

pub struct InsertionPoint<'a, T> {
    pub left_endpoint: Option<&'a mut T>,
    pub right_endpoint: Option<&'a mut T>,
}

enum TryInsertResult<T: Ord + Clone> {
    NothingInserted,
    Inserted(InsertResult<T>),
}

enum InsertResult<T: Ord + Clone> {
    Inserted,
    PendingSplit(T, node::Node<T>),
}
