use super::{ChildrenCapacity, NodeCapacity};
use super::{NodeIter, Sample};
use sized_chunks::Chunk;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SamplesNode<T: Ord> {
	// An array with capacity for all elements
	samples: Chunk<Sample<T>, NodeCapacity>,
	// Either None for a leaf node or an array with capacity for an owned
	// reference to all nodes
	children: Option<Chunk<Box<Self>, ChildrenCapacity>>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum PushResult<T: Ord> {
	UpdatedInPlace,
	Inserted(InsertResult<T>),
}

#[derive(PartialEq, Eq, Debug)]
pub enum InsertResult<T: Ord> {
	Inserted,
	PendingSplit(Sample<T>, SamplesNode<T>),
}

impl<T: Ord> SamplesNode<T> {
	pub fn new() -> Self {
		SamplesNode {
			samples: Chunk::new(),
			children: None,
		}
	}

	pub fn with_samples(samples: Vec<Sample<T>>, children: Option<Vec<SamplesNode<T>>>) -> Self {
		if let Some(children) = &children {
			assert_eq!(children.len(), samples.len() + 1);
		}
		SamplesNode {
			samples: samples.into_iter().collect(),
			children: children.map(|children| children.into_iter().map(|c| Box::new(c)).collect()),
		}
	}

	/// Insert a new value into the node or one of its children.
	/// This can happen by actually adding it to the tree or by updating
	/// neighbouring data (micro-compression)
	pub fn push_value(
		&mut self,
		value: T,
		cap: u64,
		has_parent_left: bool,
		parent_right: Option<&mut Sample<T>>,
	) -> PushResult<T> {
		// Find first index such that element > sample.value
		let pos = self
			.samples
			.iter()
			.position(|element| element.value > value)
			.unwrap_or(self.samples.len());

		match &mut self.children {
			Some(children) => {
				// Non-leaf node: recursively look into the child

				// Update context values
				let has_parent_left = has_parent_left || pos > 0;
				let parent_right = self.samples.get_mut(pos).or(parent_right);

				match children[pos].push_value(value, cap, has_parent_left, parent_right) {
					// Insertion bubbled a split up
					PushResult::Inserted(InsertResult::PendingSplit(med_element, right_child)) => {
						PushResult::Inserted(self.insert_sample(
							med_element,
							Some(right_child),
							pos,
						))
					}
					// Done
					x => x,
				}
			}
			None => {
				// Leaf node: check if we should insert this new sample or just
				// update a neighbour one
				self.push_value_leaf(value, pos, cap, has_parent_left, parent_right)
			}
		}
	}

	/// Insert a new value to this leaf node, detecting a possible micro-compression
	/// oportunity
	fn push_value_leaf(
		&mut self,
		value: T,
		pos: usize,
		cap: u64,
		has_parent_left: bool,
		parent_right: Option<&mut Sample<T>>,
	) -> PushResult<T> {
		let possible_sample = if pos == 0 && self.samples.len() == 0 {
			// Tree is empty
			Some(Sample::exact(value))
		} else if pos == 0 && !has_parent_left {
			// Minimum all the way
			let min = &self.samples[0];
			debug_assert_eq!(min.g, 1);
			debug_assert_eq!(min.delta, 0);
			let after_min = self.samples.get(1);
			match after_min {
				Some(after_min) if after_min.delta + after_min.g + 1 <= cap => {
					// Merge previous `min` into `after_min` and replace it
					self.samples[1].g += 1;
					self.samples[0].value = value;
					None
				}
				_ => {
					// Insert
					Some(Sample::exact(value))
				}
			}
		} else if pos == self.samples.len() && parent_right.is_none() {
			// Maximum all the way
			let max = self.samples.last_mut().unwrap();
			debug_assert_eq!(max.delta, 0);
			if max.g + 1 <= cap {
				// Merge previous `max` into this new one
				max.g += 1;
				max.value = value;
				None
			} else {
				// Insert
				Some(Sample::exact(value))
			}
		} else {
			// Should not panic, otherwise we would stop on the `else if` above
			let right = self.samples.get_mut(pos).or(parent_right).unwrap();
			if right.delta + right.g + 1 <= cap {
				// Drop
				right.g += 1;
				None
			} else {
				// Insert
				let delta = right.g + right.delta - 1;
				Some(Sample { value, g: 1, delta })
			}
		};

		match possible_sample {
			None => PushResult::UpdatedInPlace,
			Some(sample) => PushResult::Inserted(self.insert_sample(sample, None, pos)),
		}
	}

	/// Actually insert a `sample` (and optional right child) into this node.
	/// If the node is full, it will be split it into (left, median, right).
	/// Self will become left and the other two values will be returned
	fn insert_sample(
		&mut self,
		sample: Sample<T>,
		right_child: Option<SamplesNode<T>>,
		pos: usize,
	) -> InsertResult<T> {
		if !self.samples.is_full() {
			// Simply insert at this node
			self.insert_sample_non_full(sample, right_child, pos);
			return InsertResult::Inserted;
		}

		// Node is full: split into two and return median and new node to insert at the parent
		let med_pos = self.samples.len() / 2;
		let right_samples = self.samples.split_off(med_pos + 1);
		let med_element = self.samples.pop_back();
		let right_children = self
			.children
			.as_mut()
			.map(|children| children.split_off(med_pos + 1));
		let mut new_right_child = SamplesNode {
			samples: right_samples,
			children: right_children,
		};

		// Insert left or right
		// This part of the code depends on the fact that `CAPACITY` is odd,
		// so the `median` can be chosen before inserting the new value
		if pos <= med_pos {
			self.insert_sample_non_full(sample, right_child, pos);
		} else {
			new_right_child.insert_sample_non_full(sample, right_child, pos - med_pos - 1);
		}

		InsertResult::PendingSplit(med_element, new_right_child)
	}

	/// Insert a sample into this non-full node
	fn insert_sample_non_full(
		&mut self,
		sample: Sample<T>,
		right_child: Option<SamplesNode<T>>,
		pos: usize,
	) {
		self.samples.insert(pos, sample);
		if let Some(children) = &mut self.children {
			children.insert(pos + 1, Box::new(right_child.unwrap()));
		}
	}
}

impl<T: Ord> IntoIterator for SamplesNode<T> {
	type Item = Sample<T>;
	type IntoIter = NodeIter<T>;

	fn into_iter(self) -> Self::IntoIter {
		let samples_iter = Box::new(self.samples.into_iter());
		let children_iter = self.children.map(|children| Box::new(children.into_iter()));
		NodeIter::new(samples_iter, children_iter)
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::fmt::Debug;
	use typenum::marker_traits::Unsigned;

	fn helper_new_node<T: Ord>(
		values: Vec<T>,
		children: Option<Vec<SamplesNode<T>>>,
	) -> SamplesNode<T> {
		SamplesNode::with_samples(
			values.into_iter().map(|v| Sample::exact(v)).collect(),
			children,
		)
	}

	fn helper_assert_values<T: Ord + Debug + Clone>(node: &SamplesNode<T>, values: Vec<T>) {
		let node_values: Vec<T> = node
			.samples
			.iter()
			.map(|sample| sample.value.clone())
			.collect();
		assert_eq!(node_values, values);
	}

	fn helper_assert_children_first_values<T: Ord + Debug + Clone>(
		node: &SamplesNode<T>,
		values: Vec<T>,
	) {
		let child_first_values: Vec<T> = node
			.children
			.as_ref()
			.unwrap()
			.iter()
			.map(|child| child.samples[0].value.clone())
			.collect();
		assert_eq!(child_first_values, values);
	}

	#[test]
	fn create_node() {
		let leaf = helper_new_node(vec![1, 2], None);
		helper_assert_values(&leaf, vec![1, 2]);

		let right = helper_new_node(vec![10, 20], None);
		let left = helper_new_node(vec![100, 200], None);
		let non_leaf = helper_new_node(vec![3], Some(vec![right, left]));
		helper_assert_values(&non_leaf, vec![3]);
		helper_assert_values(&non_leaf.children.as_ref().unwrap()[0], vec![10, 20]);
		helper_assert_values(&non_leaf.children.as_ref().unwrap()[1], vec![100, 200]);
	}

	#[test]
	#[should_panic]
	fn create_node_too_big() {
		helper_new_node((0..NodeCapacity::to_u64() + 1).collect::<Vec<_>>(), None);
	}

	#[test]
	#[should_panic]
	fn create_node_wrong_number_of_children() {
		helper_new_node(vec![3, 14], Some(vec![]));
	}

	#[test]
	fn insert_sample_non_full() {
		let mut leaf_left = helper_new_node(vec![], None);
		let mut leaf_right = helper_new_node(vec![], None);
		leaf_left.insert_sample_non_full(Sample::exact(17), None, 0);
		leaf_left.insert_sample_non_full(Sample::exact(15), None, 0);
		leaf_right.insert_sample_non_full(Sample::exact(25), None, 0);
		leaf_right.insert_sample_non_full(Sample::exact(27), None, 1);
		helper_assert_values(&leaf_left, vec![15, 17]);
		helper_assert_values(&leaf_right, vec![25, 27]);

		let mut non_leaf = helper_new_node(vec![20], Some(vec![leaf_left, leaf_right]));

		let new_leaf = helper_new_node(vec![35, 37], None);
		non_leaf.insert_sample_non_full(Sample::exact(30), Some(new_leaf), 1);
		helper_assert_values(&non_leaf, vec![20, 30]);
		helper_assert_children_first_values(&non_leaf, vec![15, 25, 35]);
	}

	#[test]
	fn insert_sample() {
		// Fill node
		let capacity = NodeCapacity::to_u64() as i32;
		let med = capacity / 2;
		let mut node = helper_new_node(vec![], None);
		for i in 0..capacity {
			assert_eq!(
				node.insert_sample(Sample::exact(i as i32), None, i as usize),
				InsertResult::Inserted
			);
		}

		let mut node2 = node.clone();

		// Split and add to right
		assert_eq!(
			node.insert_sample(Sample::exact(-1), None, 1),
			InsertResult::PendingSplit(
				Sample::exact(med),
				helper_new_node((med + 1..capacity).collect(), None)
			)
		);
		helper_assert_values(&node, vec![0, -1].into_iter().chain(1..med).collect());

		// Split and add to left
		assert_eq!(
			node2.insert_sample(Sample::exact(-1), None, (capacity - 1) as usize),
			InsertResult::PendingSplit(
				Sample::exact(med),
				helper_new_node(
					(med + 1..capacity - 1)
						.chain(vec![-1, capacity - 1].into_iter())
						.collect(),
					None
				)
			)
		);
		helper_assert_values(&node2, (0..med).collect());
	}

	#[test]
	fn insert_sample_non_leaf() {
		let capacity = NodeCapacity::to_u64() as i32;
		let med = capacity / 2;
		let elements = (0..capacity).collect();
		let children: Vec<SamplesNode<_>> = (capacity..2 * capacity + 1)
			.map(|n| helper_new_node(vec![n], None))
			.collect();
		let mut node = helper_new_node(elements, Some(children.clone()));

		let new_value = -1;
		let new_node = helper_new_node(vec![-2], None);
		assert_eq!(
			node.insert_sample(Sample::exact(new_value), Some(new_node), 1),
			InsertResult::PendingSplit(
				Sample::exact(med),
				helper_new_node(
					(med + 1..capacity).collect(),
					Some(children[(med + 1) as usize..].to_vec())
				)
			)
		);
		helper_assert_values(&node, vec![0, -1].into_iter().chain(1..med).collect());
		helper_assert_children_first_values(
			&node,
			vec![capacity, capacity + 1, -2]
				.into_iter()
				.chain(capacity + 2..capacity + med + 1)
				.collect(),
		);
	}
}
