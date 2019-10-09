#[cfg(test)]
#[macro_use]
extern crate lazy_static;

mod btree;
pub mod gk;
pub mod modified_gk;
pub mod quantile_generator;

/// Convert from quantile to the rank
/// 0 <= quantile <= 1
/// 1 <= rank <= num
/// Example, for num = 4:
/// quantile   -> rank
/// [0, 1/4]   -> 1
/// (1/4, 2/4] -> 2
/// (2/4, 3/4] -> 3
/// (3/4, 1]   -> 4
pub fn quantile_to_rank(quantile: f64, num: u64) -> u64 {
    assert!(
        quantile >= 0. && quantile <= 1.,
        "Invalid quantile {}: out of range",
        quantile
    );
    ((quantile * num as f64).ceil() as u64).max(1)
}

// Reverse quantile_to_rank()
pub fn rank_to_quantile(rank: u64, num: u64) -> f64 {
    assert!(
        rank > 0 && rank <= num,
        "Invalid rank {}: out of range",
        rank
    );
    if rank == 1 {
        0.
    } else {
        rank as f64 / num as f64
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const E: f64 = std::f64::EPSILON;

    #[test]
    fn test_quantiles() {
        assert_eq!(quantile_to_rank(0., 4), 1);
        assert_eq!(quantile_to_rank(E, 4), 1);
        assert_eq!(quantile_to_rank(1. / 4., 4), 1);

        assert_eq!(quantile_to_rank(1. / 4. + E, 4), 2);
        assert_eq!(quantile_to_rank(2. / 4., 4), 2);

        assert_eq!(quantile_to_rank(2. / 4. + E, 4), 3);
        assert_eq!(quantile_to_rank(3. / 4., 4), 3);

        assert_eq!(quantile_to_rank(3. / 4. + E, 4), 4);
        assert_eq!(quantile_to_rank(1., 4), 4);
    }

    #[test]
    #[should_panic]
    fn quantile_too_small() {
        quantile_to_rank(-E, 4);
    }

    #[test]
    #[should_panic]
    fn quantile_too_big() {
        quantile_to_rank(1. + E, 4);
    }

    #[test]
    fn test_ranks() {
        assert_eq!(rank_to_quantile(1, 1), 0.);

        assert_eq!(rank_to_quantile(1, 2), 0.);
        assert_eq!(rank_to_quantile(2, 2), 1.);

        assert_eq!(rank_to_quantile(1, 4), 0.);
        assert_eq!(rank_to_quantile(2, 4), 2. / 4.);
        assert_eq!(rank_to_quantile(3, 4), 3. / 4.);
        assert_eq!(rank_to_quantile(4, 4), 1.);
    }

    #[test]
    #[should_panic]
    fn rank_too_small() {
        rank_to_quantile(0, 0);
    }

    #[test]
    #[should_panic]
    fn rank_too_big() {
        rank_to_quantile(11, 10);
    }
}
