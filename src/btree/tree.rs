use super::node::Node;
use super::*;
use std::mem::MaybeUninit;
use std::ptr;

#[derive(Clone)]
pub struct BTree<T: Ord + Clone> {
    pub(super) root: Node<T>,
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
    pub fn try_insert<F>(&mut self, search_value: &T, get_insert_value: F)
    where
        F: FnOnce(InsertionPoint<T>) -> Option<T>,
    {
        // Delegate logic to root node
        if let TryInsertResult::Inserted(insert_result) =
            self.root
                .try_insert(search_value, get_insert_value, None, None)
        {
            self.handle_insert_result(insert_result);
        }
    }

    /// Insert a new value into the tree
    pub fn insert(&mut self, value: T) {
        self.try_insert(&value.clone(), |_| Some(value));
    }

    /// Return the total number of values actually present in the tree
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return a sorted iterator over references to elements in the tree
    pub fn iter(&self) -> TreeIter<T> {
        TreeIter::new(self)
    }

    /// Insert a new value larger or equal to the current maximum value.
    /// This is a logical error to violate the above requirement.
    fn insert_max(&mut self, value: T) {
        let insert_result = self.root.insert_max(value);
        self.handle_insert_result(insert_result);
    }

    /// Handle the result of an insertion
    fn handle_insert_result(&mut self, insert_result: InsertResult<T>) {
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

impl<T: Ord + Clone> std::iter::FromIterator<T> for BTree<T> {
    /// Create a tree from an interator. If the iterator returns elements in ascending order
    /// an optimization will kick in and speed up each insertion
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut tree = BTree::new();

        // Load first data
        let mut iter = iter.into_iter();
        if let Some(first) = iter.next() {
            let mut ascending = Some(first.clone());
            tree.insert(first);

            // Insert other data
            for value in iter {
                match &ascending {
                    Some(max) if value >= *max => {
                        // Fast path
                        ascending = Some(value.clone());
                        tree.insert_max(value);
                    }
                    _ => {
                        tree.insert(value);
                        ascending = None;
                    }
                }
            }
        }

        tree
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_new_root() {
        // Fill tree
        let mut tree = BTree::new();
        for i in 0..CAPACITY {
            tree.try_insert(&i, |_| Some(i));
        }
        assert_eq!(tree.len(), CAPACITY);
        assert_eq!(tree.root.len(), CAPACITY);

        // Split at insert
        tree.try_insert(&0, |_| Some(0));
        assert_eq!(tree.len(), CAPACITY + 1);
        assert_eq!(tree.root.len(), 1);
    }

    #[test]
    fn iter() {
        fn check<T: Ord + Clone + std::fmt::Debug>(mut values: Vec<T>) {
            let mut tree: BTree<T> = BTree::new();
            for i in values.iter() {
                tree.try_insert(i, |_| Some(i.clone()));
            }

            values.sort();
            let tree_collected = tree.iter().cloned().collect::<Vec<_>>();

            assert_eq!(values, tree_collected);
        }

        // Leaf tree
        check((0..CAPACITY).collect::<Vec<_>>());

        // Tree with two levels
        check((0..CAPACITY * CAPACITY / 2).collect::<Vec<_>>());

        // Tree with three levels
        check((0..CAPACITY * CAPACITY).collect::<Vec<_>>());

        // Pi
        check(vec![
            31, 41, 59, 26, 53, 58, 97, 93, 23, 84, 62, 64, 33, 83, 27, 95, 2, 88, 41, 97, 16, 93,
            99, 37, 51, 5, 82, 9, 74, 94, 45, 92, 30, 78, 16, 40, 62, 86, 20, 89, 98, 62, 80, 34,
            82, 53, 42, 11, 70, 67, 98, 21, 48, 8, 65, 13, 28, 23, 6, 64, 70, 93, 84, 46, 9, 55, 5,
            82, 23, 17, 25, 35, 94, 8, 12, 84, 81, 11, 74, 50, 28, 41, 2, 70, 19, 38, 52, 11, 5,
            55, 96, 44, 62, 29, 48, 95, 49, 30, 38, 19, 64, 42, 88, 10, 97, 56, 65, 93, 34, 46, 12,
            84, 75, 64, 82, 33, 78, 67, 83, 16, 52, 71, 20, 19, 9, 14, 56, 48, 56, 69, 23, 46, 3,
            48, 61, 4, 54, 32, 66, 48, 21, 33, 93, 60, 72, 60, 24, 91, 41, 27, 37, 24, 58, 70, 6,
            60, 63, 15, 58, 81, 74, 88, 15, 20, 92, 9, 62, 82, 92, 54, 9, 17, 15, 36, 43, 67, 89,
            25, 90, 36, 0, 11, 33, 5, 30, 54, 88, 20, 46, 65, 21, 38, 41, 46, 95, 19, 41, 51, 16,
            9, 43, 30, 57, 27, 3, 65, 75, 95, 91, 95, 30, 92, 18, 61, 17, 38, 19, 32, 61, 17, 93,
            10, 51, 18, 54, 80, 74, 46, 23, 79, 96, 27, 49, 56, 73, 51, 88, 57, 52, 72, 48, 91, 22,
            79, 38, 18, 30, 11, 94, 91, 29, 83, 36, 73, 36, 24, 40, 65, 66, 43, 8, 60, 21, 39, 49,
            46, 39, 52, 24, 73, 71, 90, 70, 21, 79, 86, 9, 43, 70, 27, 70, 53, 92, 17, 17, 62, 93,
            17, 67, 52, 38, 46, 74, 81, 84, 67, 66, 94, 5, 13, 20, 0, 56, 81, 27, 14, 52, 63, 56,
            8, 27, 78, 57, 71, 34, 27, 57, 78, 96, 9, 17, 36, 37, 17, 87, 21, 46, 84, 40, 90, 12,
            24, 95, 34, 30, 14, 65, 49, 58, 53, 71, 5, 7, 92, 27, 96, 89, 25, 89, 23, 54, 20, 19,
            95, 61, 12, 12, 90, 21, 96, 8, 64, 3, 44, 18, 15, 98, 13, 62, 97, 74, 77, 13, 9, 96, 5,
            18, 70, 72, 11, 34, 99, 99, 99, 83, 72, 97, 80, 49, 95, 10, 59, 73, 17, 32, 81, 60, 96,
            31, 85, 95, 2, 44, 59, 45, 53, 46, 90, 83, 2, 64, 25, 22, 30, 82, 53, 34, 46, 85, 3,
            52, 61, 93, 11, 88, 17, 10, 10, 0, 31, 37, 83, 87, 52, 88, 65, 87, 53, 32, 8, 38, 14,
            20, 61, 71, 77, 66, 91, 47, 30, 35, 98, 25, 34, 90, 42, 87, 55, 46, 87, 31, 15, 95, 62,
            86, 38, 82, 35, 37, 87, 59, 37, 51, 95, 77, 81, 85, 77, 80, 53, 21, 71, 22, 68, 6, 61,
            30, 1, 92, 78, 76, 61, 11, 95, 90, 92, 16, 42, 1, 98,
        ]);
    }

    #[test]
    fn from_iter() {
        fn check<T: Ord + Clone + std::fmt::Debug>(mut values: Vec<T>) {
            let tree: BTree<T> = values.iter().cloned().collect();
            values.sort();
            let tree_collected = tree.iter().cloned().collect::<Vec<_>>();
            assert_eq!(values, tree_collected);
        }

        check::<i32>(vec![]);
        check(vec![1, 2, 3, 1, 2, 3]);
        check((0..1000).collect::<Vec<_>>());
        check((0..1000).chain(20..30).collect::<Vec<_>>());
    }
}
