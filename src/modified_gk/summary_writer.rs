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
        SummaryWriter {
            summary: Summary::new(max_expected_error),
            buffer: Vec::with_capacity(DEFAULT_BUFFER_CAPACITY),
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
        }
    }

    /// Wrap a summary into a writer
    pub fn with_summary(summary: Summary<T>) -> SummaryWriter<T> {
        SummaryWriter {
            summary,
            buffer: Vec::with_capacity(DEFAULT_BUFFER_CAPACITY),
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
        }
    }

    /// Update (increase or decrease) the buffer's capacity
    /// This should be called right after creation, before new elements are added
    pub fn update_capacity(&mut self, new_capacity: usize) {
        assert_eq!(self.buffer.len(), 0);
        self.buffer_capacity = new_capacity;
        self.buffer = Vec::with_capacity(new_capacity);
    }

    /// Insert a single new value into the Summary
    pub fn insert_one(&mut self, value: T) {
        unimplemented!()
    }

    /// Write out any pending value into the Summary and return it
    pub fn into_summary(self) -> Summary<T> {
        unimplemented!()
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