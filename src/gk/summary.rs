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
    pub fn query(&self, quantile: f64) -> Option<&T> {
        // Note: unlike the original article, this operation will return the
        // closest tuple instead of the least one when there are multiple possible
        // answers
        if self.len == 0 {
            return None;
        }

        let rank = quantile_to_rank(quantile, self.len);
        let mut min_rank = 0;
        let max_err = (self.epsilon * self.len as f64).floor() as u64;
        let mut best_sample: (&Sample<T>, f64) =
            (self.samples.first().unwrap(), std::f64::INFINITY);
        for sample in &self.samples {
            min_rank += sample.g;
            let max_rank = min_rank + sample.delta;
            let mid = min_rank as f64 + (sample.delta as f64 / 2.).ceil();
            let error = rank as f64 - mid;
            if rank <= max_err + min_rank
                && max_rank <= max_err + rank
                && error.abs() < best_sample.1.abs()
            {
                best_sample = (sample, error);
            }
        }

        Some(&best_sample.0.value)
    }

    /// Merge another summary into this oen
    pub fn merge(&mut self, mut other: Summary<T>) {
        assert_eq!(
            self.epsilon, other.epsilon,
            "Both Summary epsilons must be the same"
        );

        // Add all other samples and sort by value
        self.compress();
        other.compress();
        self.len += other.len;
        self.samples.extend(other.samples);
        self.samples.sort();
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