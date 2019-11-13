use super::node::{InsertResult, PushResult, SamplesNode};
use std::mem;

pub struct SamplesTree<T: Ord> {
	root: SamplesNode<T>,
	len: usize,
}

impl<T: Ord> SamplesTree<T> {
	/// Create a new empty tree
	pub fn new() -> Self {
		SamplesTree {
			root: SamplesNode::new(),
			len: 0,
		}
	}

	/// Insert a new value into the tree.
	/// This can happen by actually adding it to the tree or by updating
	/// neighbouring data (micro-compression)
	pub fn push_value(&mut self, value: T, cap: u64) {
		if let PushResult::Inserted(insert_result) = self.root.push_value(value, cap, false, None) {
			self.len += 1;
			if let InsertResult::PendingSplit(med_element, right_child) = insert_result {
				// Splitting reached root tree: build new root node
				let old_root = mem::replace(&mut self.root, SamplesNode::new());
				self.root =
					SamplesNode::with_samples(vec![med_element], Some(vec![old_root, right_child]));
			}
		}
	}

	/// Return the number of stored samples in the whole tree
	pub fn len(&self) -> usize {
		self.len
	}
}
