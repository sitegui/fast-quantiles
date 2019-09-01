
use crate::quantile_to_rank;
use std::iter::{ExactSizeIterator, FusedIterator};
/// An iterator that will generate `num` sequential values and that holds:
/// rank(x) = ceil(quantile * (num - 1)), where
/// rank(x) is defined as the number of values strictly smaller than x
/// At the extremes, with quantile = 0, x is the minimum of the sequence and
/// with quantile = 1, x is the maximum
pub struct SequentialGenerator {
    // `value` could be simply added to `offset`, but we keep them separate to
    // avoid float imprecisions and make sure the actual value is returned at the
    // right position
    value: f64,
    position: usize,
    direction: f64,
    offset: f64,
    num: usize,
}

pub enum SequentialOrder {
    Ascending,
    Descending,
}

impl SequentialGenerator {
    pub fn new(
        quantile: f64,
        value: f64,
        num: usize,
        order: SequentialOrder,
    ) -> SequentialGenerator {
        assert!(num > 0);
        let rank = quantile_to_rank(quantile, num);
        let (direction, offset) = match order {
            SequentialOrder::Ascending => (1., -(rank as f64) + 1.),
            _ => (-1., (num - rank) as f64),
        };
        SequentialGenerator {
            value,
            position: 0,
            direction,
            offset,
            num,
        }
    }
}

impl Iterator for SequentialGenerator {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        // The terms of the sequence are defined as:
        // v[i] = value + alpha*i + beta
        if self.position == self.num {
            None
        } else {
            let r = self.value + (self.direction * self.position as f64 + self.offset);
            self.position += 1;
            Some(r)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.num - self.position;
        (size, Some(size))
    }
}

impl FusedIterator for SequentialGenerator {}

impl ExactSizeIterator for SequentialGenerator {}

