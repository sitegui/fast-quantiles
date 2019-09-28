
use super::sample::Sample;
use super::Summary;
const DEFAULT_BUFFER_CAPACITY: usize = 1_000;

/// An efficient interface to write a lot of values to a single Summary
pub struct SummaryWriter<T: Ord> {
    summary: Summary<T>,
    buffer: Vec<T>,
    buffer_capacity: usize,
}

impl<T: Ord> SummaryWriter<T> {
    /// Create a new empty writer
    pub fn new(max_expected_error: f64) -> SummaryWriter<T> {
        SummaryWriter::with_capacity(max_expected_error, DEFAULT_BUFFER_CAPACITY)
    }

    /// Create a new empty writer with a custom buffer size
    pub fn with_capacity(max_expected_error: f64, buffer_capacity: usize) -> SummaryWriter<T> {
        SummaryWriter::with_summary_and_capacity(Summary::new(max_expected_error), buffer_capacity)
    }

    /// Wrap a summary into a writer
    pub fn with_summary(summary: Summary<T>) -> SummaryWriter<T> {
        SummaryWriter::with_summary_and_capacity(summary, DEFAULT_BUFFER_CAPACITY)
    }

    /// Wrap a summary into a writer with a custom buffer size
    pub fn with_summary_and_capacity(
        summary: Summary<T>,
        buffer_capacity: usize,
    ) -> SummaryWriter<T> {
        SummaryWriter {
            summary,
            buffer: Vec::with_capacity(buffer_capacity),
            buffer_capacity,
        }
    }

    /// Insert a single new value into the Summary
    pub fn insert_one(&mut self, value: T) {
        self.buffer.push(value);
        if self.buffer.len() == self.buffer_capacity {
            self.flush();
        }
    }

    /// Write out any pending value into the Summary and return it
    pub fn into_summary(mut self) -> Summary<T> {
        self.flush();
        self.summary
    }

    /// Write all pending values into the underlying Summary
    fn flush(&mut self) {
        let len = self.buffer.len();
        if len == 0 {
            return;
        }
        self.buffer.sort();
        let samples = self.buffer.drain(..).map(|value| Sample {
            value,
            g: 1,
            delta: 0,
        });
        self.summary.merge_sorted_samples(samples, len as u64, 0);
    }
}

/// Consume an interator into the Summary
impl<T: Ord> Extend<T> for SummaryWriter<T> {
    fn extend<Iter>(&mut self, iter: Iter)
    where
        Iter: IntoIterator<Item = T>,
    {
        for value in iter {
            self.insert_one(value);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn writer() {
        assert_eq!(
            get_samples_spec(0.2, 3, vec![8, 6, 0]),
            vec![(0, 1, 0), (6, 1, 0), (8, 1, 0)]
        );

        assert_eq!(
            get_samples_spec(0.2, 3, vec![8, 6, 0, 4, 3, 9]),
            vec![(0, 1, 0), (4, 2, 0), (8, 2, 0), (9, 1, 0)]
        );

        assert_eq!(
            get_samples_spec(0.2, 3, vec![8, 6, 0, 4, 3, 9, 2, 5, 1]),
            vec![(0, 1, 0), (2, 2, 1), (4, 2, 0), (8, 3, 0), (9, 1, 0)]
        );

        assert_eq!(
            get_samples_spec(0.2, 3, vec![8, 6, 0, 4, 3, 9, 2, 5, 1, 7]),
            vec![(0, 1, 0), (4, 4, 0), (8, 4, 0), (9, 1, 0)]
        );
    }

    fn get_samples_spec(
        max_expected_error: f64,
        buffer_capacity: usize,
        values: Vec<i32>,
    ) -> Vec<(i32, u64, u64)> {
        let mut writer = SummaryWriter::with_capacity(max_expected_error, buffer_capacity);
        writer.extend(values.into_iter());
        writer.into_summary().samples_spec()
    }
}