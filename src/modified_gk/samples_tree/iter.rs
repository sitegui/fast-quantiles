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
