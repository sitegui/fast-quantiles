use super::node::{InsertResult, PushResult, SamplesNode};
use super::{IntoIter, Iter, Sample};
use std::mem;

pub struct SamplesTree<T: Ord> {
	root: SamplesNode<T>,
	len: usize,
	depth: usize,
}

impl<T: Ord> SamplesTree<T> {
	/// Create a new empty tree
	pub fn new() -> Self {
		SamplesTree {
			root: SamplesNode::new(),
			len: 0,
			depth: 0,
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
				self.depth += 1;
			}
		}
	}

	/// Return the number of stored samples in the whole tree
	pub fn len(&self) -> usize {
		self.len
	}

	/// Create a iterator over a reference to all the samples in sorted order
	pub fn iter(&self) -> Iter<T> {
		self.root.iter(self.depth)
	}
}

impl<T: Ord> IntoIterator for SamplesTree<T> {
	type Item = Sample<T>;
	type IntoIter = IntoIter<T>;

	/// Create a iterator over all the samples in sorted order
	fn into_iter(self) -> Self::IntoIter {
		self.root.into_iter()
	}
}

#[cfg(test)]
mod test {
	use super::super::NodeCapacity;
	use super::*;
	use typenum::marker_traits::Unsigned;

	#[test]
	fn iterators() {
		fn check<T: Ord + Clone + std::fmt::Debug>(mut values: Vec<T>) {
			// Build tree from exact samples (use cap = 0 to keep all them)
			let mut tree: SamplesTree<T> = SamplesTree::new();
			for i in values.iter().cloned() {
				tree.push_value(i, 0);
			}
			assert_eq!(tree.len(), values.len());

			// Collect from by-ref and by-value iterators
			let collected_by_ref: Vec<T> = tree.iter().map(|sample| sample.value.clone()).collect();
			let collected_by_value: Vec<T> = tree.into_iter().map(|sample| sample.value).collect();

			values.sort();
			assert_eq!(values, collected_by_ref);
			assert_eq!(values, collected_by_value);
		}

		let capacity = NodeCapacity::to_u64() as i32;

		// Empty tree
		check::<i32>(vec![]);

		// Leaf tree
		check((0..capacity).collect::<Vec<_>>());

		// Tree with two levels
		check((0..capacity * capacity / 2).collect::<Vec<_>>());

		// Tree with three levels
		check((0..capacity * capacity).collect::<Vec<_>>());

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
}
