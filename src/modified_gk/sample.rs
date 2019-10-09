use std::cmp::Ordering;

/// Represent each saved sample
#[derive(Debug, Copy, Clone)]
pub struct Sample<T: Ord> {
    pub value: T,
    pub g: u64,
    pub delta: u64,
}

impl<T: Ord> Sample<T> {
    /// A sample with the exact knowledge of the value's rank
    pub fn exact(value: T) -> Self {
        Sample {
            value,
            g: 1,
            delta: 0,
        }
    }
}

impl<T: Ord> Ord for Sample<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: Ord> PartialOrd for Sample<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Ord> PartialEq for Sample<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Ord> Eq for Sample<T> {}
