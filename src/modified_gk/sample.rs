use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

/// Represent each saved sample
/// For ordering, only the field `value` is considered
#[derive(Debug, Copy, Clone)]
pub struct Sample<T: Ord> {
    pub value: T,
    pub g: u64,
    pub delta: u64
}

impl<T: Ord> Sample<T> {
    pub fn new(value: T, g: u64, delta: u64) -> Sample<T> {
        Sample {
            value,
            g,
            delta
        }
    }
}

impl<T: Ord> PartialEq for Sample<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<T: Ord> Eq for Sample<T> {}

impl<T: Ord> PartialOrd for Sample<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: Ord> Ord for Sample<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}
