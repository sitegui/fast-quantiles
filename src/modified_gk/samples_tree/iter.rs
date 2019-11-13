use super::node::SamplesNode;
use super::{ChildrenCapacity, NodeCapacity, Sample};
use sized_chunks::sized_chunk::Iter;

type SamplesIter<T> = Box<Iter<Sample<T>, NodeCapacity>>;
type ChildrenIter<T> = Box<Iter<Box<SamplesNode<T>>, ChildrenCapacity>>;

/// Iterate over all samples in this node and its descendents
pub struct NodeIter<T: Ord> {
	samples_iter: SamplesIter<T>,
	children_iter: Option<ChildrenIter<T>>,
	state: State<T>,
}

enum State<T: Ord> {
	Sample,
	Child(Box<NodeIter<T>>),
}

impl<T: Ord> NodeIter<T> {
	pub fn new(samples_iter: SamplesIter<T>, children_iter: Option<ChildrenIter<T>>) -> Self {
		NodeIter {
			samples_iter,
			children_iter,
			state: State::Sample,
		}
	}
}

impl<T: Ord> Iterator for NodeIter<T> {
	type Item = Sample<T>;
	fn next(&mut self) -> Option<Self::Item> {
		match &mut self.state {
			State::Sample => {
				// Return next sample
				let sample = self.samples_iter.next();
				if let Some(children_iter) = &mut self.children_iter {
					// Iterate over a child
					let child_iter = children_iter.next().unwrap().into_iter();
					self.state = State::Child(Box::new(child_iter));
				}
				sample
			}
			State::Child(child_iter) => match child_iter.next() {
				None => {
					self.state = State::Sample;
					self.next()
				}
				v => v,
			},
		}
	}
}
