use crate::quantile_to_rank;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use std::iter::{ExactSizeIterator, FusedIterator};
use rand::Rng;

/// An iterator that will generate `num` random values and that holds:
/// rank(x) = ceil(quantile * (num - 1)), where
/// rank(x) is defined as the number of values strictly smaller than x
/// At the extremes, with quantile = 0, x is the minimum of the sequence and
/// with quantile = 1, x is the maximum
pub struct RandomGenerator {
    remaining_lesser: usize,
    remaining: usize, // excluding the target value
    value: f64,
    published_value: bool,
    rng: Pcg64,
}

impl RandomGenerator {
    pub fn new(quantile: f64, value: f64, num: usize, seed: u64) -> RandomGenerator {
        assert!(num > 0);
        let remaining_lesser = quantile_to_rank(quantile, num) - 1;
        RandomGenerator {
            remaining_lesser,
            remaining: num - 1,
            value,
            published_value: false,
            rng: Pcg64::seed_from_u64(seed),
        }
    }
}

impl RandomGenerator {
    fn next_random(&mut self) -> f64 {
        self.rng.gen()
    }

    fn next_non_zero_random(&mut self) -> f64 {
        let mut r = self.next_random();
        while r == 0. {
            r = self.next_random();
        }
        r
    }
}

impl Iterator for RandomGenerator {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        // At each step, we'll select whether to generate a greater, lesser or the target value
        // This decision is random, however with weights proportional to the number
        // of remaining draws

        // Check end of cursor
        if self.remaining == 0 && self.published_value {
            return None;
        }

        // Publish target value
        if !self.published_value {
            let remaining_ratio = 1. / (self.remaining + 1) as f64;
            if self.next_random() < remaining_ratio {
                self.published_value = true;
                return Some(self.value);
            }
        }

        // Publish other values
        let ratio = self.remaining_lesser as f64 / self.remaining as f64;
        self.remaining -= 1;
        if self.next_random() >= ratio {
            // Greater or equal
            Some(self.value + self.next_random())
        } else {
            // Lesser (multiply by 1-E to make sure it will be lesser even when the random value is zero)
            self.remaining_lesser -= 1;
            Some(self.value - self.next_non_zero_random())
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut size = self.remaining;
        if !self.published_value {
            size += 1;
        }
        return (size, Some(size));
    }
}

impl FusedIterator for RandomGenerator {}

impl ExactSizeIterator for RandomGenerator {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn respect_seed() {
        fn check(seed: u64, expected_values: Vec<f64>) {
            let values: Vec<f64> = RandomGenerator::new(0.5, 17., 7, seed).collect();
            assert_eq!(values, expected_values);
        }

        check(
            1,
            vec![
                16.886098289795832,
                17.475850141634222,
                17.0,
                16.448791316457317,
                17.04676166514965,
                17.708530834207153,
                16.61197782022746,
            ],
        );

        check(
            2,
            vec![
                17.0,
                16.723992847202776,
                17.473140469528996,
                17.890264969958412,
                16.184033271866923,
                16.300485594323114,
                17.527720330285856,
            ],
        );
    }
}