use super::*;
use std::mem::MaybeUninit;
use std::ptr;

pub(super) struct Node<T: Ord + Clone> {
    len: usize,
    elements: [MaybeUninit<T>; CAPACITY],
    children: Option<[MaybeUninit<Box<Node<T>>>; CAPACITY + 1]>,
}

impl<T: Ord + Clone> Node<T> {
    /// Recursive implementation of `BTree::try_insert`.
    /// When this node splits, it will return the median and new right node
    pub(super) fn try_insert<F>(
        &mut self,
        search_value: T,
        get_insert_value: F,
        mut left_endpoint: Option<&mut T>,
        mut right_endpoint: Option<&mut T>,
    ) -> TryInsertResult<T>
    where
        F: FnOnce(InsertionPoint<T>) -> Option<T>,
    {
        // Find first index such that element > search_value
        let mut index = self.len;
        for i in 0..self.len {
            // Safe since the element is inside the initialized zone
            let element = unsafe { &mut *self.elements.get_unchecked_mut(i).as_mut_ptr() };
            if *element > search_value {
                index = i;
                right_endpoint = Some(element);
                break;
            }
        }

        if index > 0 {
            left_endpoint =
                Some(unsafe { &mut *self.elements.get_unchecked_mut(index - 1).as_mut_ptr() });
        }

        match &mut self.children {
            Some(children) => {
                // Recursively look into its children
                let child = unsafe { &mut *children.get_unchecked_mut(index).as_mut_ptr() };
                match child.try_insert(
                    search_value,
                    get_insert_value,
                    left_endpoint,
                    right_endpoint,
                ) {
                    TryInsertResult::Inserted(InsertResult::PendingSplit(median, right)) => {
                        TryInsertResult::Inserted(self.insert_and_split(median, Some(right), index))
                    }
                    x => x,
                }
            }
            None => {
                // Insertion point found: call closure and check if the insertion should proceed
                match get_insert_value(InsertionPoint {
                    left_endpoint,
                    right_endpoint,
                }) {
                    None => TryInsertResult::NothingInserted,
                    Some(insertion_value) => TryInsertResult::Inserted(self.insert_and_split(
                        insertion_value,
                        None,
                        index,
                    )),
                }
            }
        }
    }

    /// Build a new node (leaf or root).
    /// It is marked unsafe because the node will copy the values from the arguments.
    /// It up to the caller to make sure no other valid reference to these items exist
    /// after the call
    pub(super) unsafe fn with_elements_and_children(
        elements: &[MaybeUninit<T>],
        children: Option<&[MaybeUninit<Box<Node<T>>>]>,
    ) -> Self {
        // Copy elements
        assert!(elements.len() <= CAPACITY);
        let mut copied_elements: [MaybeUninit<T>; CAPACITY] = MaybeUninit::uninit().assume_init();
        ptr::copy_nonoverlapping(
            elements.as_ptr(),
            copied_elements.as_mut_ptr(),
            elements.len(),
        );

        // Copy children
        let copied_children = children.map(|children| {
            assert_eq!(children.len(), elements.len() + 1);
            let mut copied: [MaybeUninit<Box<Node<T>>>; CAPACITY + 1] =
                MaybeUninit::uninit().assume_init();
            ptr::copy_nonoverlapping(children.as_ptr(), copied.as_mut_ptr(), children.len());
            copied
        });

        Node {
            len: elements.len(),
            elements: copied_elements,
            children: copied_children,
        }
    }

    /// Insert `value` (and optional right child) into this node.
    /// If the node is full, it will be split it into (left, median, right).
    /// Self will become left and the other two values will be returned
    fn insert_and_split(
        &mut self,
        value: T,
        right_child: Option<Node<T>>,
        index: usize,
    ) -> InsertResult<T> {
        if self.len < CAPACITY {
            // Simply insert at this node
            self.insert(value, right_child, index);
            return InsertResult::Inserted;
        }

        // Node is full: split into two and return median and new node to insert at the parent
        // This is safe since each element will placed in exactly one of the left, median or right
        // (copied bits will be left in the uninitialized region, but that should not be accessed
        // directly afterwards either way)
        let med = self.len / 2;
        let (median, mut right) = unsafe {
            self.len = med;
            (
                ptr::read(self.elements.get_unchecked(med)).assume_init(),
                Node::with_elements_and_children(
                    &self.elements[med + 1..],
                    self.children.as_ref().map(|children| &children[med + 1..]),
                ),
            )
        };

        // Insert left or right
        // This part of the code depends on the fact that `CAPACITY` is odd,
        // so the `median` can be chosen before inserting the new value
        if index <= med {
            self.insert(value, right_child, index);
        } else {
            right.insert(value, right_child, index - med - 1);
        }

        InsertResult::PendingSplit(median, right)
    }

    /// Insert `value` (and optional right child) into this non-full node
    fn insert(&mut self, value: T, right_child: Option<Node<T>>, index: usize) {
        // Sanity checks
        assert!(self.len < CAPACITY);
        assert!(index <= self.len);

        // If this is a leaf node no child can be inserted.
        // Conversely, in an internal node a child must be inserted
        assert_eq!(self.children.is_none(), right_child.is_none());

        unsafe {
            // Insert element
            let p = self.elements.as_mut_ptr().add(index);
            // Shift everything over to make space.
            // (Duplicating the `index`th element into two consecutive places.)
            ptr::copy(p, p.offset(1), self.len - index);
            // Write it in, overwriting the first copy of the `index`th element.
            ptr::write(p, MaybeUninit::new(value));

            if let Some(child) = right_child {
                // Insert child
                let p = self.children.as_mut().unwrap().as_mut_ptr().add(index + 1);
                ptr::copy(p, p.offset(1), self.len - index);
                ptr::write(p, MaybeUninit::new(Box::new(child)));
            }

            self.len += 1
        }
    }

    #[cfg(test)]
    fn spec(self) {}
}

impl<T: Ord + Clone> Drop for Node<T> {
    /// Since MaybeUninit won't drop the wrapped values by itself, each Node is
    /// responsible for dropping the initialized spots
    fn drop(&mut self) {
        unsafe {
            let len = self.len;
            // Avoid problems if the recursive drops panic
            self.len = 0;
            for element in &mut self.elements[0..len] {
                ptr::drop_in_place(element.as_mut_ptr());
            }
            if let Some(children) = &mut self.children {
                for element in &mut children[0..len + 1] {
                    ptr::drop_in_place(element.as_mut_ptr());
                }
            }
        }
    }
}

impl<T: Ord + Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        unsafe {
            // Clone elements
            let mut cloned_elements: [MaybeUninit<T>; CAPACITY] =
                MaybeUninit::uninit().assume_init();
            for (i, el) in self.elements[0..self.len].iter().enumerate() {
                *cloned_elements.get_unchecked_mut(i) = MaybeUninit::new((*el.as_ptr()).clone());
            }

            // Recursively clone children
            let cloned_children = self.children.as_ref().map(|children| {
                let mut cloned_children: [MaybeUninit<Box<Node<T>>>; CAPACITY + 1] =
                    MaybeUninit::uninit().assume_init();
                for (i, el) in children[0..self.len + 1].iter().enumerate() {
                    *cloned_children.get_unchecked_mut(i) =
                        MaybeUninit::new((*el.as_ptr()).clone());
                }
                cloned_children
            });

            Node {
                len: self.len,
                elements: cloned_elements,
                children: cloned_children,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_node() {
        let leaf = unsafe {
            Node::with_elements_and_children(&[MaybeUninit::new(3), MaybeUninit::new(14)], None)
        };

        assert_eq!(leaf.len, 2);

        let non_leaf = unsafe {
            Node::with_elements_and_children(
                &[MaybeUninit::new(3), MaybeUninit::new(14)],
                Some(&[
                    MaybeUninit::new(Box::new(leaf.clone())),
                    MaybeUninit::new(Box::new(leaf.clone())),
                    MaybeUninit::new(Box::new(leaf)),
                ]),
            )
        };

        assert_eq!(non_leaf.len, 2);
    }

    #[test]
    #[should_panic]
    fn create_node_too_big() {
        unsafe {
            let mut elements: [MaybeUninit<_>; CAPACITY + 1] = MaybeUninit::uninit().assume_init();
            for i in 0..CAPACITY + 1 {
                elements[i] = MaybeUninit::new(i);
            }
            Node::with_elements_and_children(&elements, None);
        }
    }

    #[test]
    #[should_panic]
    fn create_node_wrong_number_of_children() {
        unsafe {
            Node::with_elements_and_children(
                &[MaybeUninit::new(3), MaybeUninit::new(14)],
                Some(&[]),
            );
        }
    }
}
