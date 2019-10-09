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
    pub(super) fn try_insert<'a, F>(
        &'a mut self,
        search_value: &T,
        get_insert_value: F,
        left: Option<&'a mut T>,
        right: Option<&'a mut T>,
    ) -> TryInsertResult<T>
    where
        F: FnOnce(InsertionPoint<T>) -> Option<T>,
    {
        // Find first index such that element > search_value
        let mut index = self.len;
        let mut new_right = None;
        for i in 0..self.len {
            // Safe since the element is inside the initialized zone
            let element = unsafe { self.get_mut_element_unchecked(i) };
            if *element > *search_value {
                index = i;
                new_right = Some(element);
                break;
            }
        }

        let new_left = if index > 0 {
            Some(unsafe { self.get_mut_element_unchecked(index - 1) })
        } else {
            None
        };

        match &self.children {
            // Non-leaf node
            Some(_) => {
                let child = unsafe { self.get_mut_child_unchecked(index) };

                // Recursively look into the child
                match child.try_insert(
                    search_value,
                    get_insert_value,
                    new_left.or(left),
                    new_right.or(right),
                ) {
                    // Insertion bubbled a split up
                    TryInsertResult::Inserted(InsertResult::PendingSplit(
                        median,
                        new_right_node,
                    )) => TryInsertResult::Inserted(self.insert_and_split(
                        median,
                        Some(new_right_node),
                        index,
                    )),
                    x => x,
                }
            }
            // Leaf
            None => {
                // Build the final insertion point structure
                let insertion_point = unsafe {
                    if index == 0 && self.len == 0 {
                        // Tree is empty
                        InsertionPoint::Empty
                    } else if index == 0 && left.is_none() {
                        // Minimum all the way
                        InsertionPoint::Minimum(
                            new_right.unwrap(),
                            if self.len > 1 {
                                Some(self.get_mut_element_unchecked(1))
                            } else {
                                right
                            },
                        )
                    } else if index == self.len && right.is_none() {
                        // Maximum all the way
                        InsertionPoint::Maximum(new_left.unwrap())
                    } else {
                        // Right is always present at this point, otherwise the `else if`
                        // above would catch it
                        InsertionPoint::Intermediate(new_right.or(right).unwrap())
                    }
                };

                // Insertion point found: call closure and check if the insertion should proceed
                match get_insert_value(insertion_point) {
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

    /// Insert a new value larger or equal to the current maximum value.
    /// This is a logical error to violate the above requirement.
    pub(super) fn insert_max(&mut self, value: T) -> InsertResult<T> {
        match self.children {
            // Recursively look into its children
            Some(_) => {
                let child = unsafe { self.get_mut_child_unchecked(self.len) };
                match child.insert_max(value) {
                    InsertResult::PendingSplit(median, right) => {
                        self.insert_and_split(median, Some(right), self.len)
                    }
                    x => x,
                }
            }
            // Insertion point found
            None => self.insert_and_split(value, None, self.len),
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

    /// Return the total number of elements in this node
    pub(super) fn len(&self) -> usize {
        self.len
    }

    /// Return the element at the given index.
    /// Panics if out-of-bounds
    pub(super) fn get_element(&self, index: usize) -> &T {
        assert!(index < self.len);
        unsafe { &*self.elements.get_unchecked(index).as_ptr() }
    }

    /// Return the element at the given index.
    unsafe fn get_mut_element_unchecked<'a, 'b>(&'a mut self, index: usize) -> &'b mut T {
        &mut *self.elements.get_unchecked_mut(index).as_mut_ptr()
    }

    /// Return whether the node is a leaf
    pub(super) fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    /// Return the child at the given index.
    /// Panics if it is a leaf node or out-of-bounds access
    pub(super) fn get_child(&self, index: usize) -> &Node<T> {
        assert!(index < self.len + 1);
        unsafe {
            &*self
                .children
                .as_ref()
                .unwrap()
                .get_unchecked(index)
                .as_ptr()
        }
    }

    /// Return the child at the given index.
    unsafe fn get_mut_child_unchecked(&mut self, index: usize) -> &mut Node<T> {
        &mut *self
            .children
            .as_mut()
            .unwrap()
            .get_unchecked_mut(index)
            .as_mut_ptr()
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
    use std::sync::Mutex;

    // Wrap a value and
    // - double it every clone operation
    // - count the number of drop calls
    lazy_static! {
        static ref NUM_DROPPED: Mutex<u32> = Mutex::new(0);
        static ref DROP_MUTEX: Mutex<()> = Mutex::new(());
    }
    #[derive(Ord, Eq, PartialOrd, PartialEq, Debug)]
    struct Element(i32);
    impl Clone for Element {
        fn clone(&self) -> Self {
            Element(2 * self.0)
        }
    }
    impl Drop for Element {
        fn drop(&mut self) {
            *NUM_DROPPED.lock().unwrap() += 1;
        }
    }

    fn helper_assert_drop_count<T>(x: T, num: u32) {
        let lock = DROP_MUTEX.lock().unwrap();
        let before = *NUM_DROPPED.lock().unwrap();
        drop(x);
        let after = *NUM_DROPPED.lock().unwrap();
        *NUM_DROPPED.lock().unwrap() = 0;
        drop(lock);
        assert_eq!(before, 0);
        assert_eq!(after, num);
    }

    /// Create node from owning data structures
    fn helper_new_node<T: Ord + Clone>(
        elements: Vec<T>,
        children: Option<Vec<Node<T>>>,
    ) -> Node<T> {
        let init_elements = elements
            .into_iter()
            .map(|x| MaybeUninit::new(x))
            .collect::<Vec<_>>();
        let init_children = children.map(|children| {
            children
                .into_iter()
                .map(|x| MaybeUninit::new(Box::new(x)))
                .collect::<Vec<_>>()
        });
        let ref_init_children = init_children.as_ref().map(|x| &x[..]);
        let node = unsafe { Node::with_elements_and_children(&init_elements, ref_init_children) };
        std::mem::forget(init_elements);
        std::mem::forget(init_children);
        node
    }

    fn helper_assert_elements(node: &Node<Element>, values: Vec<i32>) {
        assert_eq!(node.len, values.len());
        for (i, v) in values.iter().enumerate() {
            assert_eq!(node.get_element(i).0, *v);
        }
    }

    fn helper_assert_children_first_element(node: &Node<Element>, values: Vec<i32>) {
        assert_eq!(node.len + 1, values.len());
        for (i, v) in values.iter().enumerate() {
            assert_eq!(node.get_child(i).get_element(0).0, *v);
        }
    }

    fn helper_assert_eq_insertion_point(
        insertion_point: InsertionPoint<Element>,
        expected_insertion_point: InsertionPoint<i32>,
    ) {
        assert!(match (insertion_point, expected_insertion_point) {
            (InsertionPoint::Empty, InsertionPoint::Empty) => true,
            (InsertionPoint::Minimum(a, b), InsertionPoint::Minimum(a2, b2)) => {
                assert_eq!(a.0, *a2);
                assert_eq!(b.map(|x| x.0), b2.map(|x| *x));
                true
            }
            (InsertionPoint::Maximum(a), InsertionPoint::Maximum(a2)) => {
                assert_eq!(a.0, *a2);
                true
            }
            (InsertionPoint::Intermediate(a), InsertionPoint::Intermediate(a2)) => {
                assert_eq!(a.0, *a2);
                true
            }
            _ => false,
        });
    }

    #[test]
    fn create_node() {
        let leaf = helper_new_node(vec![Element(1), Element(2)], None);
        assert_eq!(leaf.len, 2);
        assert_eq!(leaf.get_element(0).0, 1);
        assert_eq!(leaf.get_element(1).0, 2);

        let non_leaf = helper_new_node(
            vec![Element(3), Element(4)],
            Some(vec![leaf.clone(), leaf.clone(), leaf]),
        );
        assert_eq!(non_leaf.len, 2);
        assert_eq!(non_leaf.get_element(0).0, 3);
        assert_eq!(non_leaf.get_element(1).0, 4);
        assert_eq!(non_leaf.get_child(0).get_element(0).0, 2);
        assert_eq!(non_leaf.get_child(0).get_element(1).0, 4);
        assert_eq!(non_leaf.get_child(1).get_element(0).0, 2);
        assert_eq!(non_leaf.get_child(1).get_element(1).0, 4);
        assert_eq!(non_leaf.get_child(2).get_element(0).0, 1);
        assert_eq!(non_leaf.get_child(2).get_element(1).0, 2);

        helper_assert_drop_count(non_leaf, 8);
    }

    #[test]
    #[should_panic]
    fn create_node_too_big() {
        helper_new_node((0..CAPACITY + 1).collect::<Vec<_>>(), None);
    }

    #[test]
    #[should_panic]
    fn create_node_wrong_number_of_children() {
        helper_new_node(vec![3, 14], Some(vec![]));
    }

    #[test]
    fn clone_node() {
        // Create node topology
        let a = helper_new_node(vec![Element(1), Element(2)], None);
        let b = helper_new_node(vec![Element(4), Element(5)], None);
        let c = helper_new_node(vec![Element(3)], Some(vec![a, b]));

        assert_eq!(c.get_element(0).0, 3);
        assert_eq!(c.get_child(0).get_element(0).0, 1);
        assert_eq!(c.get_child(1).get_element(0).0, 4);

        // Cloned explicitly
        let d = c.clone();
        assert_eq!(d.get_element(0).0, 6);
        assert_eq!(d.get_child(0).get_element(0).0, 2);
        assert_eq!(d.get_child(1).get_element(0).0, 8);

        // Drop calls
        helper_assert_drop_count(c, 5);
        helper_assert_drop_count(d, 5);
    }

    #[test]
    fn insert() {
        let mut leaf_left = helper_new_node(vec![], None);
        let mut leaf_right = helper_new_node(vec![], None);
        leaf_left.insert(Element(17), None, 0);
        leaf_left.insert(Element(15), None, 0);
        leaf_right.insert(Element(25), None, 0);
        leaf_right.insert(Element(27), None, 1);
        assert_eq!(leaf_left.len, 2);
        assert_eq!(leaf_left.get_element(0).0, 15);
        assert_eq!(leaf_left.get_element(1).0, 17);
        assert_eq!(leaf_right.len, 2);
        assert_eq!(leaf_right.get_element(0).0, 25);
        assert_eq!(leaf_right.get_element(1).0, 27);

        let mut non_leaf = helper_new_node(vec![Element(20)], Some(vec![leaf_left, leaf_right]));

        let new_leaf = helper_new_node(vec![Element(35), Element(37)], None);
        non_leaf.insert(Element(30), Some(new_leaf), 1);
        assert_eq!(non_leaf.len, 2);
        assert_eq!(non_leaf.get_element(0).0, 20);
        assert_eq!(non_leaf.get_element(1).0, 30);
        assert_eq!(non_leaf.get_child(0).get_element(0).0, 15);
        assert_eq!(non_leaf.get_child(1).get_element(0).0, 25);
        assert_eq!(non_leaf.get_child(2).get_element(0).0, 35);

        helper_assert_drop_count(non_leaf, 8);
    }

    #[test]
    fn insert_and_split_leaf() {
        // Fill node
        let mut node = helper_new_node(vec![], None);
        for i in 0..11 {
            assert!(match node.insert_and_split(Element(i as i32), None, i) {
                InsertResult::Inserted => true,
                _ => false,
            });
        }

        let mut node2 = node.clone();

        // Split and add to right
        assert!(match node.insert_and_split(Element(-1), None, 2) {
            InsertResult::PendingSplit(el, right_node) => {
                assert_eq!(el.0, 5);
                helper_assert_drop_count(el, 1);
                helper_assert_elements(&node, vec![0, 1, -1, 2, 3, 4]);
                helper_assert_drop_count(node, 6);
                helper_assert_elements(&right_node, vec![6, 7, 8, 9, 10]);
                helper_assert_drop_count(right_node, 5);
                true
            }
            _ => false,
        });

        // Split and add to left
        assert!(match node2.insert_and_split(Element(-1), None, 7) {
            InsertResult::PendingSplit(el, right_node) => {
                assert_eq!(el.0, 10);
                helper_assert_drop_count(el, 1);
                helper_assert_elements(&node2, vec![0, 2, 4, 6, 8]);
                helper_assert_drop_count(node2, 5);
                helper_assert_elements(&right_node, vec![12, -1, 14, 16, 18, 20]);
                helper_assert_drop_count(right_node, 6);
                true
            }
            _ => false,
        });
    }

    #[test]
    fn insert_and_split_non_leaf() {
        let elements = (0..11).map(|n| Element(n)).collect();
        let children = (20..32)
            .map(|n| helper_new_node(vec![Element(n)], None))
            .collect();
        let mut node = helper_new_node(elements, Some(children));

        let new_value = Element(100);
        let new_node = helper_new_node(vec![Element(101)], None);
        assert!(match node.insert_and_split(new_value, Some(new_node), 3) {
            InsertResult::PendingSplit(el, right_node) => {
                assert_eq!(el.0, 5);
                helper_assert_drop_count(el, 1);

                helper_assert_elements(&node, vec![0, 1, 2, 100, 3, 4]);
                helper_assert_children_first_element(&node, vec![20, 21, 22, 23, 101, 24, 25]);
                helper_assert_drop_count(node, 13);

                helper_assert_elements(&right_node, vec![6, 7, 8, 9, 10]);
                helper_assert_children_first_element(&right_node, vec![26, 27, 28, 29, 30, 31]);
                helper_assert_drop_count(right_node, 11);
                true
            }
            _ => false,
        })
    }

    #[test]
    fn try_insert_leaf() {
        // First insertion
        let mut node = helper_new_node(vec![], None);
        let mut search_el = Element(11);
        assert!(match node.try_insert(
            &search_el,
            |p| {
                helper_assert_eq_insertion_point(p, InsertionPoint::Empty);
                Some(Element(10))
            },
            None,
            None,
        ) {
            TryInsertResult::Inserted(InsertResult::Inserted) => true,
            _ => false,
        });
        helper_assert_elements(&node, vec![10]);

        // Min insertion point with no double right
        search_el.0 = 9;
        assert!(match node.try_insert(
            &search_el,
            |p| {
                helper_assert_eq_insertion_point(p, InsertionPoint::Minimum(&mut 10, None));
                None
            },
            None,
            None,
        ) {
            TryInsertResult::NothingInserted => true,
            _ => false,
        });
        helper_assert_elements(&node, vec![10]);

        // Max insertion
        search_el.0 = 21;
        assert!(match node.try_insert(
            &search_el,
            |p| {
                helper_assert_eq_insertion_point(p, InsertionPoint::Maximum(&mut 10));
                Some(Element(20))
            },
            None,
            None,
        ) {
            TryInsertResult::Inserted(InsertResult::Inserted) => true,
            _ => false,
        });
        helper_assert_elements(&node, vec![10, 20]);

        // Min insertion
        search_el.0 = 9;
        assert!(match node.try_insert(
            &search_el,
            |p| {
                helper_assert_eq_insertion_point(
                    p,
                    InsertionPoint::Minimum(&mut 10, Some(&mut 20)),
                );
                Some(Element(8))
            },
            None,
            None,
        ) {
            TryInsertResult::Inserted(InsertResult::Inserted) => true,
            _ => false,
        });
        helper_assert_elements(&node, vec![8, 10, 20]);

        // Non-extreme insertion
        search_el.0 = 12;
        assert!(match node.try_insert(
            &search_el,
            |p| {
                helper_assert_eq_insertion_point(p, InsertionPoint::Intermediate(&mut 20));
                Some(Element(13))
            },
            None,
            None,
        ) {
            TryInsertResult::Inserted(InsertResult::Inserted) => true,
            _ => false,
        });
        helper_assert_elements(&node, vec![8, 10, 13, 20]);

        // No insertion
        assert!(match node.try_insert(
            &search_el,
            |p| {
                helper_assert_eq_insertion_point(p, InsertionPoint::Intermediate(&mut 13));
                None
            },
            None,
            None,
        ) {
            TryInsertResult::NothingInserted => true,
            _ => false,
        });
        helper_assert_elements(&node, vec![8, 10, 13, 20]);

        helper_assert_drop_count(search_el, 1);
        helper_assert_drop_count(node, 4);
    }

    #[test]
    fn try_insert_non_leaf() {
        // Create 3 full leaf nodes and a non-full non-leaf one
        let leaf1 = helper_new_node((0..11).map(|x| Element(x)).collect(), None);
        let leaf2 = helper_new_node((100..111).map(|x| Element(x)).collect(), None);
        let leaf3 = helper_new_node((200..211).map(|x| Element(x)).collect(), None);
        let mut node = helper_new_node(
            vec![Element(50), Element(150)],
            Some(vec![leaf1, leaf2, leaf3]),
        );

        let mut check_query = |search_value: i32,
                               expected_insertion_point: InsertionPoint<i32>,
                               insert_value: Option<i32>| {
            let search_el = Element(search_value);
            node.try_insert(
                &search_el,
                |insertion_point| {
                    helper_assert_eq_insertion_point(insertion_point, expected_insertion_point);
                    insert_value.map(|x| Element(x))
                },
                None,
                None,
            );
            helper_assert_drop_count(search_el, 1);
        };

        // Min at parent
        check_query(-2, InsertionPoint::Minimum(&mut 0, Some(&mut 1)), None);
        check_query(20, InsertionPoint::Intermediate(&mut 50), None);
        check_query(5, InsertionPoint::Intermediate(&mut 6), None);
        // Max at parent
        check_query(160, InsertionPoint::Intermediate(&mut 200), None);
        check_query(205, InsertionPoint::Intermediate(&mut 206), None);
        check_query(300, InsertionPoint::Maximum(&mut 210), None);
        // Intermediate at parent
        check_query(60, InsertionPoint::Intermediate(&mut 100), None);
        check_query(140, InsertionPoint::Intermediate(&mut 150), None);
        check_query(105, InsertionPoint::Intermediate(&mut 106), None);

        // Split and move into parent
        check_query(7, InsertionPoint::Intermediate(&mut 8), Some(7));
        helper_assert_elements(&node, vec![5, 50, 150]);
        helper_assert_children_first_element(&node, vec![0, 6, 100, 200]);

        helper_assert_drop_count(node, 36);
    }
}
