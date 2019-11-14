use super::node::SamplesNode;
use super::{ChildrenCapacity, NodeCapacity, Sample};
use sized_chunks::Chunk;

type SamplesChunk<T> = Chunk<Sample<T>, NodeCapacity>;
type ChildrenChunk<T> = Option<Chunk<Box<SamplesNode<T>>, ChildrenCapacity>>;

pub struct IntoIter<T: Ord> {
	samples: SamplesChunk<T>,
	children: ChildrenChunk<T>,
	next_source: NextSource<T>,
}

enum NextSource<T: Ord> {
	Sample,
	Child(Box<IntoIter<T>>),
}

impl<T: Ord> IntoIter<T> {
	pub fn new(samples: SamplesChunk<T>, mut children: ChildrenChunk<T>) -> Self {
		let next_source = IntoIter::prepare_next_child(&mut children);

		IntoIter {
			samples,
			children,
			next_source,
		}
	}

	fn prepare_next_child(children: &mut ChildrenChunk<T>) -> NextSource<T> {
		match children {
			Some(some_children) => {
				let next_source =
					NextSource::Child(Box::new(some_children.pop_front().into_iter()));
				if some_children.is_empty() {
					// No more child nodes
					*children = None;
				}
				next_source
			}
			None => NextSource::Sample,
		}
	}
}

impl<T: Ord> Iterator for IntoIter<T> {
	type Item = Sample<T>;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match &mut self.next_source {
				NextSource::Child(child_iter) => {
					match child_iter.next() {
						next @ Some(_) => return next,
						None => {
							// This child finished
							self.next_source = NextSource::Sample;
						}
					}
				}
				NextSource::Sample => {
					if self.samples.is_empty() {
						return None;
					}
					let next = self.samples.pop_front();
					self.next_source = IntoIter::prepare_next_child(&mut self.children);
					return Some(next);
				}
			}
		}
	}
}

pub struct Iter<'a, T: Ord> {
	// The stack point to the next value to return.
	// Once empty, iteration is over
	stack: Vec<(&'a SamplesNode<T>, usize)>,
}

impl<'a, T: Ord> Iter<'a, T> {
	pub fn new(node: &'a SamplesNode<T>, tree_depth: usize) -> Self {
		let stack = Vec::with_capacity(tree_depth);
		let mut it = Iter { stack };
		it.descend(node);
		it
	}

	fn descend(&mut self, mut node: &'a SamplesNode<T>) {
		loop {
			self.stack.push((node, 0));
			match &node.children {
				None => break,
				Some(children) => node = &children[0],
			}
		}
	}
}

impl<'a, T: Ord> Iterator for Iter<'a, T> {
	type Item = &'a Sample<T>;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let (last_node, last_pos) = self.stack.last_mut().unwrap();
			let next = last_node.samples.get(*last_pos);

			match next {
				Some(_) => {
					*last_pos += 1;
					if let Some(children) = &last_node.children {
						// Walk to next sample of the deepest child
						let child = &children[*last_pos];
						self.descend(child);
					}
					return next;
				}
				None => {
					// Reached end of the node at the end of the stack
					self.stack.pop();
					if self.stack.len() == 0 {
						return None;
					}
				}
			}
		}
	}
}
