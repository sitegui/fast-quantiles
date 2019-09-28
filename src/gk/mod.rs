mod summary;
pub use summary::Summary;
mod sample;

#[cfg(test)]
mod test {
    use super::*;
    use crate::quantile_generator::*;
    use crate::rank_to_quantile;

    #[test]
    fn check_max_error() {
        fn check(epsilon: f64, num: usize) {
            let mut s = Summary::new(epsilon);
            let values = consume_generator(RandomGenerator::new(0.5, 17., num, 17), &mut [&mut s]);
            println!("{:?}", s);
            check_all_ranks(s, values, epsilon);
        }

        check(0.1, 10);
        check(0.1, 100);
        check(0.1, 1000);

        check(0.2, 10);
        check(0.2, 100);
        check(0.2, 1000);

        check(0.01, 10);
        check(0.01, 100);
        check(0.01, 1000);
    }

    #[test]
    fn check_merge_error() {
        // This test will consume from a generator into two Summary structures
        // then merge them. The final max error will be measured
        let epsilon = 0.1;
        let mut s1 = Summary::new(epsilon);
        let mut s2 = Summary::new(epsilon);
        let gen = RandomGenerator::new(0.5, 17., 10_000, 17);
        let values = consume_generator(gen, &mut [&mut s1, &mut s2]);

        println!("Before merge");
        println!("{:?}", s1);
        println!("{:?}", s2);

        println!("After merge");
        s1.merge(s2);
        println!("{:?}", s1);

        check_all_ranks(s1, values, epsilon);
    }

    #[test]
    fn check_tree_merge_error() {
        // This test will consume from a generator into eight Summary structures
        // then merge them in a tree-like structure.
        // The final max error will be measured
        let epsilon = 0.1;
        let mut s1 = Summary::new(epsilon);
        let mut s2 = Summary::new(epsilon);
        let mut s3 = Summary::new(epsilon);
        let mut s4 = Summary::new(epsilon);
        let mut s5 = Summary::new(epsilon);
        let mut s6 = Summary::new(epsilon);
        let mut s7 = Summary::new(epsilon);
        let mut s8 = Summary::new(epsilon);
        let gen = RandomGenerator::new(0.5, 17., 10_000, 17);
        let values = consume_generator(
            gen,
            &mut [
                &mut s1, &mut s2, &mut s3, &mut s4, &mut s5, &mut s6, &mut s7, &mut s8,
            ],
        );

        // Merge all summaries
        s1.merge(s2);
        s3.merge(s4);
        s5.merge(s6);
        s7.merge(s8);
        s1.merge(s3);
        s5.merge(s7);
        s1.merge(s5);

        println!("After merge");
        println!("{:?}", s1);

        check_all_ranks(s1, values, epsilon);
    }

    #[test]
    fn check_list_merge_error() {
        // This test will consume from a generator into eight Summary structures
        // then merge them all sequentially into the first one.
        // The final max error will be measured
        let epsilon = 0.1;
        let mut s1 = Summary::new(epsilon);
        let mut s2 = Summary::new(epsilon);
        let mut s3 = Summary::new(epsilon);
        let mut s4 = Summary::new(epsilon);
        let mut s5 = Summary::new(epsilon);
        let mut s6 = Summary::new(epsilon);
        let mut s7 = Summary::new(epsilon);
        let mut s8 = Summary::new(epsilon);
        let gen = RandomGenerator::new(0.5, 17., 10_000, 17);
        let values = consume_generator(
            gen,
            &mut [
                &mut s1, &mut s2, &mut s3, &mut s4, &mut s5, &mut s6, &mut s7, &mut s8,
            ],
        );

        // Merge all summaries
        s1.merge(s2);
        s1.merge(s3);
        s1.merge(s4);
        s1.merge(s5);
        s1.merge(s6);
        s1.merge(s7);
        s1.merge(s8);

        println!("After merge");
        println!("{:?}", s1);

        check_all_ranks(s1, values, epsilon);
    }

    fn consume_generator<G>(gen: G, summaries: &mut [&mut Summary<G::Item>]) -> Vec<OrderedF64>
    where
        G: QuantileGenerator,
    {
        // Collect
        let mut values = Vec::new();
        for (i, value) in gen.enumerate() {
            values.push(value);
            summaries[i % summaries.len()].insert_one(value);
        }

        // Sort
        values.sort();
        values
    }

    fn check_all_ranks(s: Summary<OrderedF64>, values: Vec<OrderedF64>, epsilon: f64) -> f64 {
        let mut max_error = (0f64, 0u64, 0u64);
        let num = s.len();

        for desired_rank in 1..=num {
            let queried = s.query(rank_to_quantile(desired_rank, num)).unwrap();
            let got_rank = (values.iter().position(|v| v == queried).unwrap() + 1) as u64;
            let error = (got_rank as f64 - desired_rank as f64) / num as f64;
            if error.abs() > max_error.0.abs() {
                max_error = (error, desired_rank, got_rank)
            }
            assert!(
                error.abs() <= epsilon,
                "desired_rank={}, queried={}, got_rank={}, error={}",
                desired_rank,
                queried.into_inner(),
                got_rank,
                error
            );
        }
        println!("max_error={:?}", max_error);

        assert_eq!(s.query(0.), values.first());
        assert_eq!(s.query(1.), values.last());

        max_error.0
    }
}