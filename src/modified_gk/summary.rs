use super::incoming_merge_state::IncomingMergeState;
use super::sample::Sample;
use super::samples_compressor::SamplesCompressor;
use crate::quantile_to_rank;

/// Implement a modified version of the algorithm by Greenwald and Khanna in
/// Space-Efficient Online Computation of Quantile Summaries
/// TODO: describe the diferences and explain why
#[derive(Debug, Clone)]
pub struct Summary<T: Ord> {
    samples: Vec<Sample<T>>,
    /// Maximum error
    max_expected_error: f64,
    /// Number of samples already seen
    len: u64,
}

impl<T: Ord> Summary<T> {
    /// Create a new empty Summary
    pub fn new(max_expected_error: f64) -> Summary<T> {
        // In the worst-case scenario, we'll hold twice the number of samples to
        // answer to quantile queries at the desired precision
        let expected_capacity = (1. / max_expected_error).ceil() as usize;
        Summary {
            samples: Vec::with_capacity(expected_capacity),
            max_expected_error,
            len: 0,
        }
    }

    /// Insert a single new value into the Summary using an algorithm with runtime complexity of O(N)
    /// If you want to insert many values with a higher performance, use SummaryWriter
    pub fn insert_one(&mut self, value: T) {
        // Find first index such that sample[i] > value
        let insert_at = self.samples.iter().position(|sample| sample.value > value);

        // Insert
        self.len += 1;
        match insert_at {
            None => {
                // Value is larger than everything -> new max
                self.samples.push(Sample {
                    value,
                    g: 1,
                    delta: 0,
                });
            }
            Some(0) => {
                // New minimum
                self.samples.insert(
                    0,
                    Sample {
                        value,
                        g: 1,
                        delta: 0,
                    },
                );
            }
            Some(pos) => {
                let cap = self.max_g_delta();
                let sample = &mut self.samples[pos];

                if sample.delta + sample.g + 1 <= cap {
                    // We can just drop this new value, so that no new allocation is needed
                    sample.g += 1;
                } else {
                    // Insert into the vector
                    let delta = sample.g + sample.delta - 1;
                    self.samples.insert(pos, Sample { value, g: 1, delta });
                }
            }
        }

        // Compress every time max_g_delta() increases
        let compress_frequency = (1. / (2. * self.max_expected_error)).ceil() as u64;
        if self.len % compress_frequency == 0 {
            self.compress()
        }
    }

    /// Merge another Summary into this one
    pub fn merge(&mut self, other: Summary<T>) {
        assert!(
            other.max_expected_error <= self.max_expected_error,
            "The incoming Summary must have an equal or smaller max_expected_error"
        );
        let other_capacity = other.samples.capacity();
        self.merge_sorted_samples(other.samples.into_iter(), other.len, other_capacity);
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
    /// max(sample.g + sample.delta) <= max_g_delta, for all samples
    fn max_g_delta(&self) -> u64 {
        return (2. * self.max_expected_error * self.len as f64).floor() as u64;
    }

    /// Compress the samples: search for samples to "forget"
    fn compress(&mut self) {
        // Use a streaming logic that will build a new vector of samples
        // NB: this algorithm could be rewritten so that it changes the samples inplace,
        // however I've opted for sharing the same underlying implementation of merge()
        let mut compressor = SamplesCompressor::new(self.max_g_delta(), self.samples.capacity());

        // Consume the samples (since T may not implement Copy, we temporally place a zero vector)
        let samples = std::mem::replace(&mut self.samples, Vec::with_capacity(0));
        for sample in samples {
            compressor.push(sample);
        }

        self.samples = compressor.into_samples();
    }

    /// Merge a source of sorted samples into this Summary
    /// `other_len` is the number of values represented by the samples, that is, the sum of all its `g` values
    /// `other_capacity` is the minimum capacity for the final merged samples vector
    pub(crate) fn merge_sorted_samples<I>(
        &mut self,
        other_samples: I,
        other_len: u64,
        other_capacity: usize,
    ) where
        I: Iterator<Item = Sample<T>>,
    {
        // Create a streaming compressor
        // Note the use of the largest capacity to avoid reallocs in final vector
        self.len += other_len;
        let capacity = self.samples.capacity().max(other_capacity);
        let max_g_delta = self.max_g_delta();
        let mut compressor = SamplesCompressor::new(max_g_delta, capacity);

        // Get current samples as iterator
        // Note the use of replace() since T may not implement Copy
        // Besides, a zero-capacity vector does not call alloc(), that's cool
        let self_samples = std::mem::replace(&mut self.samples, Vec::with_capacity(0)).into_iter();

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
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::Debug;

    #[test]
    fn insert_ones_and_query() {
        // insert [8, 6, 0, 4, 3, 9, 2, 5, 1, 7] one by one
        let mut summary = Summary::new(0.2);

        // First
        summary.insert_one(8);
        assert_samples(&summary, vec![(8, 1, 0)]);

        // New minimum
        summary.insert_one(6);
        assert_samples(&summary, vec![(6, 1, 0), (8, 1, 0)]);

        // New minimum + compression (cap=1)
        summary.insert_one(0);
        assert_samples(&summary, vec![(0, 1, 0), (6, 1, 0), (8, 1, 0)]);

        //
        summary.insert_one(4);
        assert_samples(&summary, vec![(0, 1, 0), (4, 1, 0), (6, 1, 0), (8, 1, 0)]);

        // Local compression (cap=2)
        summary.insert_one(3);
        assert_samples(&summary, vec![(0, 1, 0), (4, 2, 0), (6, 1, 0), (8, 1, 0)]);

        // New maximum + compression (cap=2)
        summary.insert_one(9);
        assert_samples(&summary, vec![(0, 1, 0), (4, 2, 0), (8, 2, 0), (9, 1, 0)]);

        //
        summary.insert_one(2);
        assert_samples(
            &summary,
            vec![(0, 1, 0), (2, 1, 1), (4, 2, 0), (8, 2, 0), (9, 1, 0)],
        );

        // Local compression (cap=3)
        summary.insert_one(5);
        assert_samples(
            &summary,
            vec![(0, 1, 0), (2, 1, 1), (4, 2, 0), (8, 3, 0), (9, 1, 0)],
        );

        // Local compression + compression (cap=3)
        summary.insert_one(1);
        assert_samples(
            &summary,
            vec![(0, 1, 0), (2, 2, 1), (4, 2, 0), (8, 3, 0), (9, 1, 0)],
        );

        // Local compression (cap=4)
        summary.insert_one(7);
        assert_samples(
            &summary,
            vec![(0, 1, 0), (2, 2, 1), (4, 2, 0), (8, 4, 0), (9, 1, 0)],
        );

        // Compression (cap=4)
        summary.compress();
        assert_samples(&summary, vec![(0, 1, 0), (4, 4, 0), (8, 4, 0), (9, 1, 0)]);

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
        check_rank(7, 4, 2);
        check_rank(8, 8, 1);
        check_rank(9, 8, 0);
        check_rank(10, 9, 0);
    }

    fn assert_samples<T: Ord + Debug + Copy>(summary: &Summary<T>, samples: Vec<(T, u64, u64)>) {
        assert_eq!(
            summary
                .samples
                .iter()
                .map(|&sample| (sample.value, sample.g, sample.delta))
                .collect::<Vec<(T, u64, u64)>>(),
            samples
        );
    }
}