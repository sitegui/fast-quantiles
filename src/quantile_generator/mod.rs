mod random;
mod sequential;

pub use random::*;
pub use sequential::*;

#[cfg(test)]
mod test {
    use super::*;

    use crate::quantile_to_rank;
    use std::cmp::Ordering;
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

    fn check_one<T>(gen: T, quantile: f64, value: f64, num: usize)
    where
        T: Iterator<Item = f64>,
    {
        // Collect iterator into a vector
        let mut values: Vec<f64> = gen.collect();

        // Calculate observed quantile
        values.sort_by(compare_floats);
        let rank: usize = quantile_to_rank(quantile, num as u64) as usize;
        let actual = values[rank - 1];

        assert_eq!(value, actual, "Sorted values: {:?}", values);
    }

    fn compare_floats(a: &f64, b: &f64) -> Ordering {
        a.partial_cmp(b).unwrap()
    }
}