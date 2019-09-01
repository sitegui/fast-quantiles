#[cfg(test)]
mod tests;

pub mod mean;

pub mod quantile_generator;

pub mod gk;

pub trait Operation {
    type Item;
    type Output;

    fn create() -> Self;

    fn update(&mut self, value: Self::Item);

    fn merge_with(&mut self, other: Self);

    fn finish(self) -> Self::Output;
}

/// Convert from quantile to the rank
/// 0 <= quantile <= 1
/// 1 <= rank <= num
/// Example, for num = 4:
/// quantile   -> rank
/// [0, 1/4]   -> 1
/// (1/4, 2/4] -> 2
/// (2/4, 3/4] -> 3
/// (3/4, 1]   -> 4
pub fn quantile_to_rank(quantile: f64, num: usize) -> usize {
    assert!(
        quantile >= 0. && quantile <= 1.,
        "Invalid quantile {}: out of range",
        quantile
    );
    ((quantile * num as f64).ceil() as usize).max(1)
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
    fn too_small() {
        quantile_to_rank(-E, 4);
    }

    #[test]
    #[should_panic]
    fn too_big() {
        quantile_to_rank(1. + E, 4);
    }
}