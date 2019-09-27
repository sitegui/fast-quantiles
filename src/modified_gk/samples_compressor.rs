use super::sample::Sample;

/// Helper structure that compress samples as they are given, in sorted order
pub struct SamplesCompressor<T: Ord> {
    max_g_delta: u64,
    compressed_samples: Vec<Sample<T>>,
    block_tail: Option<Sample<T>>,
}

impl<T: Ord> SamplesCompressor<T> {
    pub fn new(max_g_delta: u64, capacity: usize) -> Self {
        SamplesCompressor {
            max_g_delta,
            compressed_samples: Vec::with_capacity(capacity),
            block_tail: None,
        }
    }

    pub fn push(&mut self, mut sample: Sample<T>) {
        if let Some(tail_sample) = std::mem::replace(&mut self.block_tail, None) {
            if tail_sample.g + sample.g + sample.delta <= self.max_g_delta {
                // Add new sample to the current compression block
                sample.g += tail_sample.g;
            } else {
                // Commit previous block and start new
                self.compressed_samples.push(tail_sample);
            }
            self.block_tail = Some(sample);
        } else if self.compressed_samples.len() == 0 {
            // Commit minimum
            self.compressed_samples.push(sample);
        } else {
            // Start first block
            self.block_tail = Some(sample);
        }
    }

    pub fn into_samples(mut self) -> Vec<Sample<T>> {
        if let Some(tail_sample) = self.block_tail {
            // Commit last block
            self.compressed_samples.push(tail_sample);
        }
        self.compressed_samples
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compress() {
        let samples = (0..9).map(|value| Sample {
            value,
            g: 1,
            delta: 2,
        });

        let mut compressor = SamplesCompressor::new(5, 0);
        for sample in samples {
            compressor.push(sample);
        }

        assert_eq!(
            compressor.into_samples(),
            vec![
                Sample {
                    value: 0,
                    g: 1,
                    delta: 2
                },
                Sample {
                    value: 3,
                    g: 3,
                    delta: 2
                },
                Sample {
                    value: 6,
                    g: 3,
                    delta: 2
                },
                Sample {
                    value: 8,
                    g: 2,
                    delta: 2
                }
            ]
        );
    }

    #[test]
    fn no_compression() {
        for len in 0..3 {
            let mut compressor = SamplesCompressor::<i32>::new(1, 0);
            let samples = (0..len)
                .map(|value| Sample {
                    value,
                    g: 1,
                    delta: 1,
                })
                .collect::<Vec<Sample<i32>>>();
            for &sample in &samples {
                compressor.push(sample);
            }
            assert_eq!(compressor.into_samples(), samples);
        }
    }
}