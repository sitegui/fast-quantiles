mod iter;
mod node;
mod sample;
mod tree;

pub use iter::{IntoIter, Iter};
pub use sample::Sample;
pub use tree::SamplesTree;
use typenum;

// Max number of elements per node (MUST be odd)
type NodeCapacity = typenum::U15;

type ChildrenCapacity = typenum::Sum<NodeCapacity, typenum::U1>;
