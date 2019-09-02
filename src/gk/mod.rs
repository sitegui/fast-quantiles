mod summary;
pub use summary::Summary;
mod sample;

#[cfg(test)]
mod test {
    use super::*;
    use crate::quantile_generator::*;
    use crate::rank_to_quantile;
    use std::cmp::Ordering;

    #[test]
    fn check_max_error() {
        fn check(epsilon: f64, num: usize) -> Summary {
            let mut s = Summary::new(epsilon);
            let values = consume_generator(RandomGenerator::new(0.5, 17., num, 17), &mut s);
            println!("{:?}", s);
            let mut max_error = (0 as f64, 0 as usize, 0 as usize);

            for desired_rank in 1..=num {
                let queried = s.query(rank_to_quantile(desired_rank, num)).unwrap();
                let got_rank = values.iter().position(|&v| v == queried).unwrap() + 1;
                let error = (got_rank as f64 - desired_rank as f64) / num as f64;
                if error.abs() > max_error.0.abs() {
                    max_error = (error, desired_rank, got_rank)
                }
                assert!(
                    error.abs() <= epsilon,
                    "desired_rank={}, queried={}, got_rank={}, error={}",
                    desired_rank,
                    queried,
                    got_rank,
                    error
                );
            }

            println!("max_error={:?}", max_error);

            assert_eq!(s.query(0.), Some(*values.first().unwrap()));
            //assert_eq!(s.query(1.), Some(*values.last().unwrap()));
            s
        }

        check(0.1, 10);
        check(0.1, 100);
        check(0.1, 1000);
        check(0.1, 10000);

        check(0.2, 10);
        check(0.2, 100);
        check(0.2, 1000);
        check(0.2, 10000);

        check(0.01, 10);
        check(0.01, 100);
        check(0.01, 1000);
        check(0.01, 10000);
    }

    fn consume_generator<T>(gen: T, s: &mut Summary) -> Vec<f64>
    where
        T: Iterator<Item = f64>,
    {
        // Collect
        let mut values = Vec::new();
        for value in gen {
            values.push(value);
            s.insert(value);
        }

        // Sort
        values.sort_by(compare_floats);
        values
    }

    fn compare_floats(a: &f64, b: &f64) -> Ordering {
        a.partial_cmp(b).unwrap()
    }
}