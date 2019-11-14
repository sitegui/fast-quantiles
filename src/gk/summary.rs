use super::sample::Sample;

use crate::quantile_to_rank;
use std::fmt;

/// Implement the algorithm by Greenwald and Khanna in
/// Space-Efficient Online Computation of Quantile Summaries
/// This is NOT meant to be a performant implementation, but instead a correct
/// baseline, against which more performant variants can be tested
#[derive(Clone)]
pub struct Summary<T: Ord> {
    samples: Vec<Sample<T>>,
    /// Maximum error
    epsilon: f64,
    /// Number of samples already seen
    len: u64,
}

impl<T: Ord> Summary<T> {
    pub fn new(epsilon: f64) -> Self {
        Summary {
            samples: Vec::new(),
            epsilon,
            len: 0,
        }
    }

    /// Insert a new value into the summary
    /// The summary is compressed from time to time to keep only some samples
    pub fn insert_one(&mut self, value: T) {
        let compress_frequency = (1. / (2. * self.epsilon)).ceil() as u64;
        if self.len > 0 && self.len % compress_frequency == 0 {
            self.compress();
        }
        self.insert_without_compression(value);
    }

    /// Query the structure for a given epsilon-approximate quantile
    /// Return None if and only if no value was inserted
    pub fn query_with_error(&self, quantile: f64) -> Option<(&T, f64)> {
        // Note: unlike the original article, this operation will return the
        // closest tuple instead of the least one when there are multiple possible
        // answers
        if self.len == 0 {
            return None;
        }

        let target_rank = quantile_to_rank(quantile, self.len);
        let mut min_rank = 0;
        let max_err = (self.epsilon * self.len as f64).floor() as u64;
        let mut best_sample: (&Sample<T>, u64) = (self.samples.first().unwrap(), std::u64::MAX);
        for sample in &self.samples {
            min_rank += sample.g;
            let max_rank = min_rank + sample.delta;
            let mid_rank = (min_rank + max_rank) / 2;
            let max_rank_error = if target_rank > mid_rank {
                target_rank - min_rank
            } else {
                max_rank - target_rank
            };
            if target_rank <= max_err + min_rank
                && max_rank <= max_err + target_rank
                && max_rank_error < best_sample.1
            {
                best_sample = (sample, max_rank_error);
            }
        }

        Some((&best_sample.0.value, best_sample.1 as f64 / self.len as f64))
    }

    /// Query the structure for a given epsilon-approximate quantile
    /// Return None if and only if no value was inserted
    pub fn query(&self, quantile: f64) -> Option<&T> {
        self.query_with_error(quantile).map(|x| x.0)
    }

    /// Merge another summary into this oen
    pub fn merge(&mut self, other: Summary<T>) {
        // The GK algorithm is a bit unclear about it, but we need to adjust the statistics during the
        // merging. The main idea is that samples that come from one side will suffer from the lack of
        // precision of the other.
        // As a concrete example, take two QuantileSummaries whose samples (value, g, delta) are:
        // `a = [(0, 1, 0), (20, 99, 0)]` and `b = [(10, 1, 0), (30, 49, 0)]`
        // This means `a` has 100 values, whose minimum is 0 and maximum is 20,
        // while `b` has 50 values, between 10 and 30.
        // The resulting samples of the merge will be:
        // a+b = [(0, 1, 0), (10, 1, ??), (20, 99, ??), (30, 49, 0)]
        // The values of `g` do not change, as they represent the minimum number of values between two
        // consecutive samples. The values of `delta` should be adjusted, however.
        // Take the case of the sample `10` from `b`. In the original stream, it could have appeared
        // right after `0` (as expressed by `g=1`) or right before `20`, so `delta=99+0-1=98`.
        // In the GK algorithm's style of working in terms of maximum bounds, one can observe that the
        // maximum additional uncertainty over samples comming from `b` is `max(g_a + delta_a) =
        // floor(2 * eps_a * n_a)`. Likewise, additional uncertainty over samples from `a` is
        // `floor(2 * eps_b * n_b)`.
        // Only samples that interleave the other side are affected. That means that samples from
        // one side that are lesser (or greater) than all samples from the other side are just copied
        // unmodifed.
        // If the merging instances have different `relativeError`, the resulting instance will cary
        // the largest one: `eps_ab = max(eps_a, eps_b)`.
        // The main invariant of the GK algorithm is kept:
        // `max(g_ab + delta_ab) <= floor(2 * eps_ab * (n_a + n_b))` since
        // `max(g_ab + delta_ab) <= floor(2 * eps_a * n_a) + floor(2 * eps_b * n_b)`
        // Finally, one can see how the `insert(x)` operation can be expressed as `merge([(x, 1, 0])`

        let mut merged_samples = Vec::with_capacity(self.samples.len() + other.samples.len());
        let merged_epsilon = self.epsilon.max(other.epsilon);
        let merged_len = self.len + other.len;
        let additional_self_delta = (2. * other.epsilon * other.len as f64).floor() as u64;
        let additional_other_delta = (2. * self.epsilon * self.len as f64).floor() as u64;

        // Do a merge of two sorted lists until one of the lists is fully consumed
        let mut self_samples = std::mem::replace(&mut self.samples, Vec::new())
            .into_iter()
            .peekable();
        let mut other_samples = other.samples.into_iter().peekable();
        let mut started_self = false;
        let mut started_other = false;
        loop {
            match (self_samples.peek(), other_samples.peek()) {
                (Some(self_sample), Some(other_sample)) => {
                    // Detect next sample
                    let (next_sample, additional_delta) = if self_sample.value < other_sample.value
                    {
                        started_self = true;
                        (
                            self_samples.next().unwrap(),
                            if started_other {
                                additional_self_delta
                            } else {
                                0
                            },
                        )
                    } else {
                        started_other = true;
                        (
                            other_samples.next().unwrap(),
                            if started_self {
                                additional_other_delta
                            } else {
                                0
                            },
                        )
                    };

                    // Insert it
                    let next_sample = Sample {
                        value: next_sample.value,
                        g: next_sample.g,
                        delta: next_sample.delta + additional_delta,
                        band: 0,
                    };
                    merged_samples.push(next_sample);
                }
                _ => break,
            }
        }

        // Copy the remaining samples from the other list
        // (by construction, at most one `while` loop will run)
        merged_samples.extend(self_samples);
        merged_samples.extend(other_samples);

        self.samples = merged_samples;
        self.epsilon = merged_epsilon;
        self.len = merged_len;
        self.compress();
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    /// Compress the current summary, so that it will probably use less memory
    /// but still answer to any quantile query within the desired error margin
    fn compress(&mut self) {
        let compression_threshold = (2. * self.epsilon * self.len as f64).floor() as u64;
        self.update_bands(compression_threshold);

        // Iterate over each pair of samples in reverse order to merge them
        let mut i = self.samples.len() - 1;
        while i > 1 {
            i -= 1;

            let sample = &self.samples[i];
            let next_sample = &self.samples[i + 1];

            if sample.band > next_sample.band {
                // Can't be merged: incompatible bands
                continue;
            }

            let (first_descendent, g_star) = self.scan_all_descendents(i);
            let new_g = g_star + next_sample.g;
            if new_g + next_sample.delta >= compression_threshold {
                // Can't be merged: would produce a full sample
                continue;
            }

            // Merge [first_descendent, i] into i+1
            self.samples[i + 1].g = new_g;
            self.samples.drain(first_descendent..=i);
            i -= i - first_descendent;
        }
    }

    /// Insert a single new sample to the structure
    fn insert_without_compression(&mut self, value: T) {
        self.len += 1;

        // Special case: new minimum
        if self.samples.len() == 0 || value < self.samples[0].value {
            self.samples.insert(0, Sample::new(value, 0));
            return;
        }

        // Special case: new maximum
        if value >= self.samples.last().unwrap().value {
            self.samples.push(Sample::new(value, 0));
            return;
        }

        // Find point of insertion `i` such that:
        // v[i-1] <= value < v[i]
        // TODO: use binary search?
        for (i, sample) in self.samples.iter().enumerate().skip(1) {
            if value < sample.value {
                let delta = (2. * self.epsilon * self.len as f64).floor() as u64;
                self.samples.insert(i, Sample::new(value, delta));
                return;
            }
        }

        unreachable!();
    }

    /// Calculate the band for a given `delta` and `p` = 2 * epsilon * num
    /// The full valid interval of delta (that is, 0 <= delta <= p) is split into
    /// bands, starting from the right:
    /// band_0 := delta = p
    /// band_1 := p - 2 - (p mod 2) < delta <= p - 1
    /// band_a := p - 2^a - (p mod 2^a) < delta <= p - 2^(a-1) - (p mod 2^(a-1))
    /// for 1 <= a <= floor(log2(p)) + 1
    /// For example: for p = 22, the bands are:
    /// band_0 = {22}; band_1 = (20, 21], band_2 = (16, 20], band_3 = (8, 16], band_4 = (0, 8], band_5 = {0}
    fn band(delta: u64, p: u64) -> u64 {
        assert!(delta <= p);

        // Special case: for delta = 0, lower_bound would be negative and since
        // we're working with u64, that is impossible
        if delta == 0 {
            return if p == 0 {
                0
            } else {
                (p as f64).log2().floor() as u64 + 1
            };
        }

        // Search for increasing `a` (only the lower_bound need to be checked)
        // This is not meant to be an efficient implementation, but rather a correct one
        let mut a: u64 = 0;
        loop {
            let lower_bound = p - (1 << a) - (p % (1 << a));
            if delta > lower_bound {
                return a;
            }
            a += 1;
        }
    }

    /// Update the value of band for all samples
    fn update_bands(&mut self, p: u64) {
        for sample in &mut self.samples {
            sample.band = Self::band(sample.delta, p);
        }
    }

    /// Detect where all descendents of a given sample are and sum their `g` values
    /// By construction, the descendents will be a contiguous space in the vector
    /// ending up on the target sample. This means we can represent it with only
    /// the initial index `j` (inclusive).
    /// The band cache in the samples MUST be up to date
    /// The first sample (min) is special and never included as child
    fn scan_all_descendents(&self, i: usize) -> (usize, u64) {
        let mut j = i;
        let max_band = self.samples[i].band;
        let mut total_g = self.samples[i].g;
        while j > 1 && self.samples[j - 1].band < max_band {
            total_g += self.samples[j - 1].g;
            j -= 1;
        }
        (j, total_g)
    }
}

impl<T: Ord + fmt::Debug> fmt::Debug for Summary<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Summary (epsilon = {}, len = {})",
            self.epsilon, self.len
        )?;
        writeln!(
            f,
            "  {:>20}{:>10}{:>10}{:>8}{:>8}{:>10}{:>10}",
            "value", "[min_rank", "max_rank]", "g", "delta", "[min_query", "max_query]"
        )?;
        let mut min_rank = 0;
        let max_err = (self.epsilon * self.len as f64).floor() as u64;
        for sample in &self.samples {
            min_rank += sample.g;
            writeln!(
                f,
                "  {:>20?}{:>10}{:>10}{:>8}{:>8}{:>10}{:>10}",
                sample.value,
                min_rank,
                min_rank + sample.delta,
                sample.g,
                sample.delta,
                (min_rank + sample.delta) as i64 - max_err as i64,
                min_rank + max_err
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ascending_insertion() {
        let mut s = Summary::new(0.2);

        for i in 0..10 {
            s.insert_without_compression(i);
        }

        assert_eq!(s.samples.len(), 10);
        for (i, sample) in s.samples.iter().enumerate() {
            assert_eq!(sample.value, i as i32);
            assert_eq!(sample.g, 1);
            assert_eq!(sample.delta, 0);
        }
        println!("{:?}", s);
    }

    #[test]
    fn unordered_insertion() {
        let mut s = Summary::new(0.2);

        s.insert_without_compression(0);
        s.insert_without_compression(9);
        for i in 1..9 {
            s.insert_without_compression(i);
        }

        assert_eq!(s.samples.len(), 10);
        for (i, sample) in s.samples.iter().enumerate() {
            assert_eq!(sample.value, i);
            assert_eq!(sample.g, 1);
            let delta = (2. * (i + 2) as f64 * 0.2) as u64;
            assert_eq!(sample.delta, if i == 0 || i == 9 { 0 } else { delta });
        }
        println!("{:?}", s);
    }

    #[test]
    fn bands() {
        let results: Vec<Vec<u64>> = vec![
            vec![0],
            vec![1, 0],
            vec![2, 1, 0],
            vec![2, 1, 1, 0],
            vec![3, 2, 2, 1, 0],
            vec![3, 2, 2, 1, 1, 0],
            vec![3, 2, 2, 2, 2, 1, 0],
            vec![3, 2, 2, 2, 2, 1, 1, 0],
            vec![4, 3, 3, 3, 3, 2, 2, 1, 0],
            vec![4, 3, 3, 3, 3, 2, 2, 1, 1, 0],
            vec![4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 0],
            vec![4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 0],
            vec![4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 1, 0],
            vec![4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 1, 1, 0],
            vec![4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 1, 0],
            vec![4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 0],
            vec![5, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 1, 0],
            vec![5, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 1, 1, 0],
            vec![5, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 0],
            vec![5, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 0],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 1, 1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 1, 1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 1,
                0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 1,
                1, 0,
            ],
            vec![
                5, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2,
                2, 1, 0,
            ],
        ];

        for (p, row) in results.iter().enumerate() {
            for (delta, band) in row.iter().enumerate() {
                assert_eq!(
                    Summary::<i32>::band(delta as u64, p as u64),
                    *band,
                    "band({}, {}) = {}",
                    delta,
                    p,
                    band
                );
            }
        }
    }

    #[test]
    fn query_empty() {
        let s = Summary::<i32>::new(0.1);
        for i in 0..=10 {
            assert_eq!(s.query(i as f64 / 10.), None);
        }
    }

    #[test]
    fn query_full() {
        let mut s = Summary::new(0.001);
        for i in 0..20 {
            s.insert_without_compression(i);
        }
        for i in 0..20 {
            assert_eq!(s.query((i as f64 + 1.) / 20.), Some(&i));
        }
    }

    #[test]
    fn query() {
        // Represent the 20 values (1..=20) with 5 samples
        let values = vec![1, 2, 4, 7, 11, 16, 20];
        let gs = vec![1, 1, 2, 3, 4, 5, 4];
        let samples: Vec<Sample<i32>> = values
            .iter()
            .zip(gs)
            .map(|(&value, g)| Sample {
                value,
                g,
                delta: 0,
                band: 0,
            })
            .collect();
        let s = Summary {
            samples: samples,
            // max(g + delta) <= 2*epsilon*n
            epsilon: 5. / (2. * 20.),
            len: 20,
        };

        let expected_values = vec![
            1, 2, 2, 4, 4, 7, 7, 7, 7, 11, 11, 11, 11, 16, 16, 16, 16, 16, 20, 20,
        ];
        for (i, expected) in expected_values.iter().enumerate() {
            assert_eq!(s.query((i as f64 + 1.) / 20.), Some(expected));
        }
    }
}
