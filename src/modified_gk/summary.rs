/// Represent each saved sample
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct Sample<T: Ord> {
    value: T,
    g: u64,
    delta: u64,
}

/// Implement a modified version of the algorithm by Greenwald and Khanna in
/// Space-Efficient Online Computation of Quantile Summaries
/// TODO: describe the diferences and explain why
#[derive(Debug, Clone)]
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
    /// If you want to insert many values with a higher performance, use SummaryWriter
    pub fn insert_one(&mut self, value: T) {
        // Find first index such that sample[i] > value
        let insert_at = self.samples.iter().position(|sample| sample.value > value);

        // Insert
        self.num += 1;
        match insert_at {
            None => {
                // value is larger than everything -> new max
                self.samples.push(Sample {
                    value,
                    g: 1,
                    delta: 0,
                });
            }
            Some(0) => {
                // new minimum
                println!("some(0)");
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
                println!("some(pos={})", pos);
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
        if self.num % compress_frequency == 0 {
            println!("compress");
            self.compress()
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

    /// Get the current limit on g+delta
    /// An invariant of this structure is that:
    /// max(sample.g + sample.delta) <= max_g_delta, for all samples
    fn max_g_delta(&self) -> u64 {
        return (2. * self.max_expected_error * self.num as f64).floor() as u64;
    }

    /// Apply an inplace compression: search for sample to "forget"
    fn compress(&mut self) {
        if self.samples.len() <= 2 {
            // Minimum and maximum are always kept
            return;
        }

        // Look at each sample and compress with previous ones if possible
        // This is an inplace algorithm that will move the samples to drop to the end
        // of the vector, then the vector will be trimmed to the final size
        let mut new_g = self.samples[1].g; // new g value for next compressed block
        let mut insertion = 1; // next sample insertion point
        let cap = self.max_g_delta();
        for i in 2..self.samples.len() {
            let Sample {
                g: sample_g,
                delta: sample_delta,
                ..
            } = self.samples[i];

            if sample_g + sample_delta + new_g <= cap {
                // Include this sample in the current compressed block
                println!("add to block {}", i);
                new_g += sample_g;
            } else {
                // Commit the current compression block
                println!("commit {}", insertion);
                self.samples.swap(insertion, i - 1);
                self.samples[insertion].g = new_g;
                new_g = sample_g;
                insertion += 1;
            }
        }

        // Commit last compression block
        println!("final commit {}", insertion);
        let last_i = self.samples.len() - 1;
        self.samples.swap(insertion, last_i);
        self.samples[insertion].g = new_g;

        // Drop forgotten samples
        self.samples.truncate(insertion + 1);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fmt::Debug;

    #[test]
    fn insert_ones() {
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

        // compression (cap=4)
        summary.compress();
        assert_samples(&summary, vec![(0, 1, 0), (4, 4, 0), (8, 4, 0), (9, 1, 0)]);
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