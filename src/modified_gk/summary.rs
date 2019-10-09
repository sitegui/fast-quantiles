use super::incoming_merge_state::IncomingMergeState;
use super::sample::Sample;
use super::samples_compressor::SamplesCompressor;
use crate::btree::{BTree, InsertionPoint};
use crate::quantile_to_rank;

/// Implement a modified version of the algorithm by Greenwald and Khanna in
/// Space-Efficient Online Computation of Quantile Summaries
/// TODO: describe the diferences and explain why
pub struct Summary<T: Ord + Clone> {
    samples: BTree<Sample<T>>,
    /// Maximum number of samples to keep
    max_samples: u64,
    /// Maximum error
    max_expected_error: f64,
    /// Number of samples already seen
    len: u64,
}

impl<T: Ord + Clone> Summary<T> {
    /// Create a new empty Summary
    pub fn new(max_expected_error: f64) -> Summary<T> {
        let expected_least_compressed_samples = (1. / max_expected_error).ceil() as u64;
        Summary {
            samples: BTree::new(),
            // This encodes a tradeoff between using more memory and compressing more frequently.
            // However, with the implemented micro-compression at every insert, in the worst case
            // (sorted stream of values), the structure will accumulate all of the `F=1/eps` first
            // elements, then half of the next `F/2`, then a third of the next `F/2`, and so on.
            // This means that in the worst case we'll reach:
            // | saved samples | saw samples |
            // |        1.00 F |           F |
            // |        2.01 F |         6 F |
            // |        3.00 F |        42 F |
            // |        4.00 F |       309 F |
            // |        5.00 F |      2276 F |
            // Eventhough this sum is unbounded, it grows very slowly, so full compression will
            // rarely be called
            max_samples: 5 * expected_least_compressed_samples,
            max_expected_error,
            len: 0,
        }
    }

    /// Insert a single new value into the Summary
    pub fn insert_one(&mut self, value: T) {
        // General case
        let search_sample = Sample::exact(value.clone());

        self.len += 1;
        let cap = self.max_g_delta();

        self.samples.try_insert(&search_sample, |insertion_point| {
            match insertion_point {
                // First value
                InsertionPoint::Empty => Some(Sample::exact(value)),
                // New minimum
                InsertionPoint::Minimum(min, mut after_min) => {
                    debug_assert_eq!(min.g, 1);
                    debug_assert_eq!(min.delta, 0);
                    match &mut after_min {
                        Some(after_min) if after_min.delta + after_min.g + 1 <= cap => {
                            // Merge previous `min` into `after_min` and replace it
                            after_min.g += 1;
                            min.value = value;
                            None
                        }
                        _ => {
                            // Insert
                            Some(Sample::exact(value))
                        }
                    }
                }
                // New maximum
                InsertionPoint::Maximum(max) => {
                    debug_assert_eq!(max.delta, 0);
                    if max.g + 1 <= cap {
                        // Merge previous `max` into this new one
                        max.g += 1;
                        max.value = value;
                        None
                    } else {
                        // Insert
                        Some(Sample::exact(value))
                    }
                }
                // Somewhere in the middle
                InsertionPoint::Intermediate(right) => {
                    if right.delta + right.g + 1 <= cap {
                        // Drop
                        right.g += 1;
                        None
                    } else {
                        // Insert
                        let delta = right.g + right.delta - 1;
                        Some(Sample { value, g: 1, delta })
                    }
                }
            }
        });

        // Keep the number of saved samples bounded
        if self.samples.len() > self.max_samples as usize {
            println!("============== compress =================");
            self.compress()
        }
    }

    /// Merge another Summary into this one
    pub fn merge(&mut self, other: Summary<T>) {
        assert!(
            other.max_expected_error <= self.max_expected_error,
            "The incoming Summary must have an equal or smaller max_expected_error"
        );
        self.merge_sorted_samples(other.samples.iter().cloned(), other.len);
    }

    /// Query for a desired quantile
    /// Return None if and only if the summary is empty
    pub fn query(&self, q: f64) -> Option<&T> {
        self.query_with_error(q).map(|(value, _error)| value)
    }

    /// Query for a desired quantile and return the query maximum error
    /// Return None if and only if the summary is empty
    pub fn query_with_error(&self, quantile: f64) -> Option<(&T, f64)> {
        // Find the sample with the smallest maximum rank error

        let target_rank = quantile_to_rank(quantile, self.len);
        let mut min_rank = 0;

        self.samples
            .iter()
            // For each sample, calculate the maximum rank error if we choose it as the answer
            .map(|sample| {
                // This sample's rank is in [min_rank, max_rank] (inclusive in both sides)
                min_rank += sample.g;
                let max_rank = min_rank + sample.delta;
                let mid_rank = (min_rank + max_rank) / 2;

                // In the worst case, the correct sample's rank is at the opposite extremity
                let max_rank_error = if target_rank > mid_rank {
                    target_rank - min_rank
                } else {
                    max_rank - target_rank
                };

                (sample, max_rank_error)
            })
            // Grab the best answer
            .min_by_key(|&(_sample, max_rank_error)| max_rank_error)
            // Output values consistent with the public API (the value and quantile error)
            .map(|(sample, rank_error)| (&sample.value, rank_error as f64 / self.len as f64))
    }

    /// Get the maximum desired error
    pub fn max_expected_error(&self) -> f64 {
        self.max_expected_error
    }

    /// Get the number of inserted values
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Get the current limit on g+delta
    /// An invariant of this structure is that:
    /// max(sample.g + sample.delta) <= max_g_delta, for all intermediate samples
    fn max_g_delta(&self) -> u64 {
        return (2. * self.max_expected_error * self.len as f64).floor() as u64;
    }

    /// Compress the samples: search for samples to "forget"
    fn compress(&mut self) {
        let mut compressor = SamplesCompressor::new(self.max_g_delta());

        // Consume the samples (since T may not implement Copy, we temporally place a zero tree)
        for sample in self.samples.iter().cloned() {
            compressor.push(sample);
        }

        self.samples = compressor.into_samples();
    }

    /// Merge a source of sorted samples into this Summary
    /// `other_len` is the number of values represented by the samples, that is, the sum of all its `g` values
    /// `other_capacity` is the minimum capacity for the final merged samples vector
    pub(super) fn merge_sorted_samples<I>(&mut self, other_samples: I, other_len: u64)
    where
        I: Iterator<Item = Sample<T>>,
    {
        // Create a streaming compressor
        // Note the use of the largest capacity to avoid reallocs in final vector
        self.len += other_len;
        let max_g_delta = self.max_g_delta();
        let mut compressor = SamplesCompressor::new(max_g_delta);

        // Get current samples as iterator
        // Note the use of replace() since T may not implement Copy
        // Besides, a zero-capacity vector does not call alloc(), that's cool
        let self_samples = self.samples.iter().cloned();

        // Prepare state for merge
        let mut other_input = IncomingMergeState::new(other_samples);
        let mut self_input = IncomingMergeState::new(self_samples);

        // Bring the least from each iterator until one of them ends
        loop {
            match (self_input.peek(), other_input.peek()) {
                // Nothing to merge from one of the sides: move remaining values
                (None, _) => {
                    other_input.push_remaining_to(&mut compressor);
                    self.samples = compressor.into_samples();
                    break;
                }
                (_, None) => {
                    self_input.push_remaining_to(&mut compressor);
                    self.samples = compressor.into_samples();
                    break;
                }
                (Some(self_peeked), Some(other_peeked)) => {
                    // Detect from which input to consume next and prepare the next sample
                    let mut new_sample;
                    if self_peeked.value < other_peeked.value {
                        new_sample = self_input.pop_front();
                        new_sample.delta += other_input.aditional_delta();
                    } else {
                        new_sample = other_input.pop_front();
                        new_sample.delta += self_input.aditional_delta();
                    };

                    compressor.push(new_sample);
                }
            }
        }
    }

    #[cfg(test)]
    pub(super) fn samples_spec(&self) -> Vec<(T, u64, u64)>
    where
        T: Copy,
    {
        self.samples
            .iter()
            .map(|&sample| (sample.value, sample.g, sample.delta))
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn insert_one_by_one_and_query() {
        // insert [8, 6, 0, 4, 3, 9, 2, 5, 1, 7] one by one
        let mut summary = Summary::new(0.2);

        // First
        summary.insert_one(8);
        assert_eq!(summary.samples_spec(), vec![(8, 1, 0)]);

        // New minimum
        summary.insert_one(6);
        assert_eq!(summary.samples_spec(), vec![(6, 1, 0), (8, 1, 0)]);

        // New minimum
        summary.insert_one(0);
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (6, 1, 0), (8, 1, 0)],
        );

        //
        summary.insert_one(4);
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (4, 1, 0), (6, 1, 0), (8, 1, 0)],
        );

        // Local compression (cap=2)
        summary.insert_one(3);
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (4, 2, 0), (6, 1, 0), (8, 1, 0)],
        );

        // New maximum + local compression (cap=2)
        summary.insert_one(9);
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (4, 2, 0), (6, 1, 0), (9, 2, 0)],
        );

        //
        summary.insert_one(2);
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (2, 1, 1), (4, 2, 0), (6, 1, 0), (9, 2, 0)],
        );

        // Local compression (cap=3)
        summary.insert_one(5);
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (2, 1, 1), (4, 2, 0), (6, 2, 0), (9, 2, 0)],
        );

        // Local compression (cap=3)
        summary.insert_one(1);
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (2, 2, 1), (4, 2, 0), (6, 2, 0), (9, 2, 0)],
        );

        // Local compression (cap=4)
        summary.insert_one(7);
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (2, 2, 1), (4, 2, 0), (6, 2, 0), (9, 3, 0)],
        );

        // Compression (cap=4)
        summary.compress();
        assert_eq!(
            summary.samples_spec(),
            vec![(0, 1, 0), (4, 4, 0), (6, 2, 0), (9, 3, 0)],
        );

        // Query all ranks
        let check_rank = |rank, expected_value, rank_error| {
            let q = crate::rank_to_quantile(rank, summary.len());
            let (&value, error) = summary.query_with_error(q).unwrap();
            assert_eq!(expected_value, value);
            assert_eq!(rank_error as f64 / summary.len() as f64, error);
        };
        check_rank(1, 0, 0);
        check_rank(2, 0, 1);
        check_rank(3, 0, 2);
        check_rank(4, 4, 1);
        check_rank(5, 4, 0);
        check_rank(6, 4, 1);
        check_rank(7, 6, 0);
        check_rank(8, 6, 1);
        check_rank(9, 9, 1);
        check_rank(10, 9, 0);
    }
}
