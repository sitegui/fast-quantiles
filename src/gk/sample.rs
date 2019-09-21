use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd};

/// Represent each saved sample
/// For ordering, only the field `value` is considered
#[derive(Debug, Copy, Clone)]
pub struct Sample {
    pub value: f64,
    pub g: usize,
    pub delta: usize,
    // This is a cached result, that is NOT guaranteed to be up to date
    pub band: usize,
}

impl Sample {
    pub fn new(value: f64, delta: usize) -> Sample {
        Sample {
            value,
            g: 1,
            delta,
            band: 0,
        }
    }
}

impl PartialEq for Sample {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for Sample {}

impl PartialOrd for Sample {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Sample {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.partial_cmp(&other.value).unwrap()
    }
}
