use std::iter::{ExactSizeIterator, FusedIterator};

/// Create a Iterator that will create a random sequence
/// with a known number of elements and quantile
pub struct QuantileGenerator {
    remaining_lesser: usize,
    remaining: usize, // excluding the target value
    value: f64,
    published_value: bool,
}

impl QuantileGenerator {
    /// Return an iterator that will generate `num` random values and that holds:
    /// rank(x) = ceil(quantile * (num - 1)), where
    /// rank(x) is defined as the number of values strictly smaller than x
    /// At the extremes, with quantile = 0, x is the minimum of the sequence and
    /// with quantile = 1, x is the maximum
    pub fn new(quantile: f64, value: f64, num: usize) -> QuantileGenerator {
        assert!(num > 0);
        let remaining_lesser = (quantile * (num - 1) as f64).ceil() as usize;
        QuantileGenerator {
            remaining_lesser,
            remaining: num - 1,
            value,
            published_value: false,
        }
    }
}

impl Iterator for QuantileGenerator {
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
            if random() < remaining_ratio {
                self.published_value = true;
                return Some(self.value);
            }
        }

        // Publish other values
        let ratio = self.remaining_lesser as f64 / self.remaining as f64;
        self.remaining -= 1;
        if random() >= ratio {
            // Greater or equal
            Some(self.value + random())
        } else {
            // Lesser (multiply by 1-E to make sure it will be lesser even when the random value is zero)
            self.remaining_lesser -= 1;
            Some(self.value - non_zero_random())
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

impl FusedIterator for QuantileGenerator {}

impl ExactSizeIterator for QuantileGenerator {}

fn random() -> f64 {
    rand::random::<f64>()
}

fn non_zero_random() -> f64 {
    let mut r = rand::random::<f64>();
    while r == 0. {
        r = rand::random::<f64>();
    }
    r
}