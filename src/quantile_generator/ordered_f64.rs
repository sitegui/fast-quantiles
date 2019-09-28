use std::cmp::Ordering;

/// A f64 wrapped with a defined ordering
/// This assumes the underlying value is not NaN, otherwise methods may panic
#[derive(Debug, Copy, Clone)]
pub struct OrderedF64(f64);

impl PartialEq for OrderedF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for OrderedF64 {}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}

impl From<f64> for OrderedF64 {
    fn from(value: f64) -> OrderedF64 {
        OrderedF64(value)
    }
}

impl OrderedF64 {
    pub fn into_inner(self) -> f64 {
        self.0
    }
}