use super::sample::Sample;

/// Implement a modified version of the algorithm by Greenwald and Khanna in
/// Space-Efficient Online Computation of Quantile Summaries
/// TODO: describe the diferences and explain why
#[derive(Clone)]
pub struct Summary<T: Ord> {
    samples: Vec<Sample<T>>,
    /// Maximum error
    max_expected_error: f64,
    /// Number of samples already seen
    num: u64,
}

impl<T: Ord> Summary<T> {
    /// Create a new empty Summary
    pub fn new(max_expected_error: f64) -> Summary<T> {
        Summary {
            samples: Vec::new(),
            max_expected_error,
            num: 0,
        }
    }

    /// Insert a single new value into the Summary
    /// If you want to insert many values with a higher performance, use
    /// SummaryWriter
    pub fn insert_one(&mut self, value: T) {
        // Special case: new minimum
        if self.samples.len() == 0 || value < self.samples[0].value {
            self.samples.insert(0, Sample::new(value, 1, 0));
            return;
        }

    }

    /// Merge another Summary into this one
    pub fn merge(&mut self, other: &Summary<T>) {
        unimplemented!();
    }

    /// Query to a desired quantile
    pub fn query(&self, q: f64) -> Option<T> {
        unimplemented!();
    }

    /// Query to a desired quantile and return the query maximum error
    pub fn query_with_error(&self, q: f64) -> Option<(T, f64)> {
        unimplemented!();
    }

    /// Get the stored samples
    pub fn samples(&self) -> &Vec<Sample<T>> {
        &self.samples
    }

    /// Get the maximum desired error
    pub fn max_expected_error(&self) -> f64 {
        self.max_expected_error
    }

    /// Get the maximum possible error in the current state
    pub fn max_current_error(&self) -> f64 {
        unimplemented!();
    }

    /// Get the number of inserted values
    pub fn num(&self) -> u64 {
        self.num
    }
}
