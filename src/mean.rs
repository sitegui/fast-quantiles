use crate::Operation;

pub struct MeanOperation {
    count: usize,
    sum: f64,
}

impl Operation for MeanOperation {
    type Item = f64;
    type Output = f64;

    fn create() -> Self {
        MeanOperation { count: 0, sum: 0. }
    }

    fn update(&mut self, value: Self::Item) {
        self.count += 1;
        self.sum += value;
    }

    fn merge_with(&mut self, other: Self) {
        self.count += other.count;
        self.sum += other.sum;
    }

    fn finish(self) -> Self::Output {
        self.sum / (self.count as f64)
    }
}