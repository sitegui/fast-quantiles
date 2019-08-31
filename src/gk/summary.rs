use super::sample::Sample;
use std::fmt;

/// Implement the algorithm by Greenwald and Khanna in
/// Space-Efficient Online Computation of Quantile Summaries
pub struct Summary {
    samples: Vec<Sample>,
    /// Maximum error
    epsilon: f64,
    /// Number of samples already seen
    num: usize,
}

impl Summary {
    pub fn new(epsilon: f64) -> Summary {
        Summary {
            samples: Vec::new(),
            epsilon,
            num: 0,
        }
    }

    /// Insert a single new sample to the structure
    pub fn insert(&mut self, value: f64) {
        self.num += 1;

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
                let delta = (2. * self.epsilon * self.num as f64).floor() as usize;
                self.samples.insert(i, Sample::new(value, delta));
                return;
            }
        }

        unreachable!();
    }

    /// Compress the current summary, so that it will probably use less memory
    /// but still answer to any quantile query within the desired error margin
    pub fn compress(&mut self) {
        let compression_threshold = (2. * self.epsilon * self.num as f64).floor() as usize;
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

    /// Calculate the band for a given `delta` and `p` = 2 * epsilon * num
    /// The full valid interval of delta (that is, 0 <= delta <= p) is split into
    /// bands, starting from the right:
    /// band_0 := delta = p
    /// band_1 := p - 2 - (p mod 2) < delta <= p - 1
    /// band_a := p - 2^a - (p mod 2^a) < delta <= p - 2^(a-1) - (p mod 2^(a-1))
    /// for 1 <= a <= floor(log2(p)) + 1
    /// For example: for p = 22, the bands are:
    /// band_0 = {22}; band_1 = (20, 21], band_2 = (16, 20], band_3 = (8, 16], band_4 = (0, 8], band_5 = {0}
    fn band(delta: usize, p: usize) -> usize {
        assert!(delta <= p);

        // Special case: for delta = 0, lower_bound would be negative and since
        // we're working with usize, that is impossible
        if delta == 0 {
            return if p == 0 {
                0
            } else {
                (p as f64).log2().floor() as usize + 1
            };
        }

        // Search for increasing `a` (only the lower_bound need to be checked)
        // This is not meant to be an efficient implementation, but rather a correct one
        let mut a: usize = 0;
        loop {
            let lower_bound = p - (1 << a) - (p % (1 << a));
            if delta > lower_bound {
                return a;
            }
            a += 1;
        }
    }

    /// Update the value of band for all samples
    fn update_bands(&mut self, p: usize) {
        for sample in &mut self.samples {
            sample.band = Summary::band(sample.delta, p);
        }
    }

    /// Detect where all descendents of a given sample are and sum their `g` values
    /// By construction, the descendents will be a contiguous space in the vector
    /// ending up on the target sample. This means we can represent it with only
    /// the initial index `j` (inclusive).
    /// The band cache in the samples MUST be up to date
    /// The first sample (min) is special and never included as child
    fn scan_all_descendents(&self, i: usize) -> (usize, usize) {
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

impl fmt::Debug for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Summary (epsilon = {}, num = {})",
            self.epsilon, self.num
        )?;
        writeln!(
            f,
            "  {:>20}{:>10}{:>10}{:>8}{:>8}",
            "value", "[min_rank", "max_rank]", "g", "delta"
        )?;
        let mut min_rank = 0;
        for sample in &self.samples {
            min_rank += sample.g;
            writeln!(
                f,
                "  {:>20}{:>10}{:>10}{:>8}{:>8}",
                sample.value,
                min_rank,
                min_rank + sample.delta,
                sample.g,
                sample.delta
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
            s.insert(i as f64);
        }

        assert_eq!(s.samples.len(), 10);
        for (i, sample) in s.samples.iter().enumerate() {
            assert_eq!(sample.value, i as f64);
            assert_eq!(sample.g, 1);
            assert_eq!(sample.delta, 0);
        }
        println!("{:?}", s);
    }

    #[test]
    fn unordered_insertion() {
        let mut s = Summary::new(0.2);

        s.insert(0.);
        s.insert(9.);
        for i in 1..9 {
            s.insert(i as f64);
        }

        assert_eq!(s.samples.len(), 10);
        for (i, sample) in s.samples.iter().enumerate() {
            assert_eq!(sample.value, i as f64);
            assert_eq!(sample.g, 1);
            let delta = (2. * (i + 2) as f64 * 0.2) as usize;
            assert_eq!(sample.delta, if i == 0 || i == 9 { 0 } else { delta });
        }
        println!("{:?}", s);
    }

    #[test]
    fn bands() {
        let results: Vec<Vec<usize>> = vec![
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
                    Summary::band(delta, p),
                    *band,
                    "band({}, {}) = {}",
                    delta,
                    p,
                    band
                );
            }
        }
    }
}