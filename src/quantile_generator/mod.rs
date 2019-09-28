mod ordered_f64;

pub trait QuantileGenerator: Iterator<Item = OrderedF64> {}

mod random;
mod sequential;

pub use ordered_f64::OrderedF64;
pub use random::RandomGenerator;
pub use sequential::{SequentialGenerator, SequentialOrder};


#[cfg(test)]
mod test {
    use super::*;

    use crate::quantile_to_rank;
    #[test]
    fn median() {
        check_all(0.5, 17., 1);
        check_all(0.5, 17., 2);
        check_all(0.5, 17., 3);
        check_all(0.5, 17., 1000);
        check_all(0.5, 17., 1001);
    }

    #[test]
    fn other_quantiles() {
        for quantile in vec![0., 0.1, 0.2, 0.75, 0.99, 1.] {
            for num in vec![1, 2, 5, 10, 100, 1000, 1001] {
                check_all(quantile, 17., num);
            }
        }
    }

    fn check_all(quantile: f64, value: f64, num: usize) {
        let it = RandomGenerator::new(quantile, value, num, 17);
        check_one(it, quantile, value, num);

        let it = SequentialGenerator::new(quantile, value, num, SequentialOrder::Ascending);
        check_one(it, quantile, value, num);

        let it = SequentialGenerator::new(quantile, value, num, SequentialOrder::Descending);
        check_one(it, quantile, value, num);
    }

    fn check_one<G: QuantileGenerator>(gen: G, quantile: f64, value: f64, num: usize) {
        // Collect iterator into a vector
        let mut values: Vec<_> = gen.collect();

        // Calculate observed quantile
        values.sort();
        let rank: usize = quantile_to_rank(quantile, num as u64) as usize;
        let actual = values[rank - 1];

        assert_eq!(value, actual.into_inner(), "Sorted values: {:?}", values);
    }
}