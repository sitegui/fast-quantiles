/// Represent each saved sample
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Sample<T: Ord> {
    pub value: T,
    pub g: u64,
    pub delta: u64,
}