use std::cmp::Ordering;

pub struct Summary {
    samples: Vec<Sample>,
}

struct Sample {
    value: f64,
    g: usize,
    delta: usize,
}

impl PartialEq for Sample {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl std::cmp::Eq for Sample {}

impl std::cmp::PartialOrd for Sample {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Sample {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.partial_cmp(&other.value).unwrap()
    }
}

impl Summary {
    pub fn new() -> Summary {
        Summary {
            samples: Vec::new(),
        }
    }

    pub fn insert(&mut self, value: f64) {
        if self.samples.len() == 0 || value > self.samples[self.samples.len() - 1].value {
            // New maximum
            self.samples.push(Sample {
                value,
                g: 1,
                delta: 0,
            });
        } else if value < self.samples[0].value {
            // New minimum
            self.samples.insert(
                0,
                Sample {
                    value,
                    g: 1,
                    delta: 0,
                },
            );
        } else {
            // Find insertion point
            for (i, sample) in self.samples.iter().enumerate() {
                if value < sample.value {
                    self.samples.insert(
                        i,
                        Sample {
                            value,
                            g: 1,
                            delta: 0,
                        },
                    );
                    break;
                }
            }
        }
    }
}