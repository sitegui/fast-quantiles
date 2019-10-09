use super::sample::Sample;
use super::samples_compressor::SamplesCompressor;

/// Keep metadata about an incoming iterator of sorted samples
pub struct IncomingMergeState<T: Ord + Clone, I: Iterator<Item = Sample<T>>> {
    iterator: I,
    next_sample: Option<Sample<T>>,
    has_started: bool,
}

impl<T: Ord + Clone, I: Iterator<Item = Sample<T>>> IncomingMergeState<T, I> {
    /// Wrap an iterator
    pub fn new(mut iter: I) -> Self {
        IncomingMergeState {
            next_sample: iter.next(),
            iterator: iter,
            has_started: false,
        }
    }

    /// Get a reference to next sample
    pub fn peek(&self) -> Option<&Sample<T>> {
        self.next_sample.as_ref()
    }

    /// Return the next sample and prepare the next one
    pub fn pop_front(&mut self) -> Sample<T> {
        self.has_started = true;
        std::mem::replace(&mut self.next_sample, self.iterator.next()).unwrap()
    }

    /// Calculate by how much a sample's delta from another incoming iterator should be increased
    pub fn aditional_delta(&self) -> u64 {
        match &self.next_sample {
            Some(real_sample) if self.has_started => real_sample.g + real_sample.delta - 1,
            _ => 0,
        }
    }

    /// Exaust the iterator, moving values to the given compressor
    pub fn push_remaining_to(self, compressor: &mut SamplesCompressor<T>) {
        if let Some(sample) = self.next_sample {
            compressor.push(sample);
            for sample in self.iterator {
                compressor.push(sample);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pop_all() {
        let sample1 = Sample {
            value: 3,
            g: 1,
            delta: 2,
        };
        let sample2 = Sample {
            value: 14,
            g: 3,
            delta: 4,
        };
        let sample3 = Sample {
            value: 15,
            g: 5,
            delta: 6,
        };
        let samples = vec![sample1, sample2, sample3].into_iter();

        let mut incoming = IncomingMergeState::new(samples);

        assert_eq!(incoming.peek(), Some(&sample1));
        assert_eq!(incoming.aditional_delta(), 0);
        assert_eq!(incoming.pop_front(), sample1);

        assert_eq!(incoming.peek(), Some(&sample2));
        assert_eq!(incoming.aditional_delta(), 6);
        assert_eq!(incoming.pop_front(), sample2);

        assert_eq!(incoming.peek(), Some(&sample3));
        assert_eq!(incoming.aditional_delta(), 10);
        assert_eq!(incoming.pop_front(), sample3);

        assert_eq!(incoming.peek(), None);
        assert_eq!(incoming.aditional_delta(), 0);

        let mut empty = SamplesCompressor::new(1);
        incoming.push_remaining_to(&mut empty);
        assert_eq!(empty.into_samples().len(), 0);
    }

    #[test]
    fn pop_none() {
        let samples = vec![3, 14, 15].into_iter().map(|value| Sample {
            value,
            g: 1,
            delta: 0,
        });
        let incoming = IncomingMergeState::new(samples);
        let mut empty = SamplesCompressor::new(1);
        incoming.push_remaining_to(&mut empty);
        assert_eq!(
            empty
                .into_samples()
                .iter()
                .map(|sample| sample.value)
                .collect::<Vec<i32>>(),
            vec![3, 14, 15]
        );
    }
}
