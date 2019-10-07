use super::node::Node;
use super::*;
use std::mem::MaybeUninit;
use std::ptr;

#[derive(Clone)]
pub struct BTree<T: Ord + Clone> {
    root: Node<T>,
    len: usize,
}

impl<T: Ord + Clone> BTree<T> {
    pub fn new() -> Self {
        unsafe {
            BTree {
                root: Node::with_elements_and_children(&[], None),
                len: 0,
            }
        }
    }

    /// Possibly insert a new value into this tree, by following these steps:
    /// 1. find where a given `search_value` would be inserted
    /// 2. execute the closure, passing the elements to right and left of the insertion point (if any).
    ///    The closure is free to mutate the received elements. It can return a new value to be
    ///    inserted at the insertion point
    /// 3. If a concrete insertion element is given, insert it
    ///
    /// The closure must guarantee that:
    /// 1. it won't mutate the passed elements in a way that changes their ordering
    /// 2. the concrete element to insert compares as equal to the effemeral `search_value`
    pub fn try_insert<F>(&mut self, search_value: T, get_insert_value: F)
    where
        F: FnOnce(InsertionPoint<T>) -> Option<T>,
    {
        // Delegate logic to root node
        if let TryInsertResult::Inserted(insert_result) =
            self.root
                .try_insert(search_value, get_insert_value, None, None)
        {
            self.len += 1;

            if let InsertResult::PendingSplit(median, right) = insert_result {
                // Splitting reached root tree: build new root node
                unsafe {
                    // Safe since the old root reference will be replaced without dropping it
                    let prev_root = ptr::read(&self.root as *const _);
                    let new_root = Node::with_elements_and_children(
                        &[MaybeUninit::new(median)],
                        Some(&[
                            MaybeUninit::new(Box::new(prev_root)),
                            MaybeUninit::new(Box::new(right)),
                        ]),
                    );
                    ptr::write(&mut self.root, new_root);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
