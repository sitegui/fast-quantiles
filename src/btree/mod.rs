mod node;
mod tree;
mod tree_iter;

pub use tree::BTree;
pub use tree_iter::TreeIter;

// This implementation assumes the capacity is odd
const CAPACITY: usize = 11;

pub enum InsertionPoint<'a, T> {
    /// The tree is empty
    Empty,
    /// This is a new minimum.
    /// Get references to the current minimum and the value after that (if it exists)
    Minimum(&'a mut T, Option<&'a mut T>),
    /// This is a new maximum. Get reference to the current maximum
    Maximum(&'a mut T),
    /// This is an intermediate value. Get reference to the next value, "to the right"
    Intermediate(&'a mut T),
}

enum TryInsertResult<T: Ord + Clone> {
    NothingInserted,
    Inserted(InsertResult<T>),
}

enum InsertResult<T: Ord + Clone> {
    Inserted,
    PendingSplit(T, node::Node<T>),
}
