use crate::quantile_generator::QuantileGenerator;
use std::cmp::Ordering;

#[test]
fn median() {
    check_quantile(0.5, 17., 1);
    check_quantile(0.5, 17., 2);
    check_quantile(0.5, 17., 3);
    check_quantile(0.5, 17., 1000);
    check_quantile(0.5, 17., 1001);
}

#[test]
fn min() {
    assert_eq!(
        QuantileGenerator::new(0., 17., 1)
            .min_by(compare_floats)
            .unwrap(),
        17.
    );
    assert_eq!(
        QuantileGenerator::new(0., 17., 10)
            .min_by(compare_floats)
            .unwrap(),
        17.
    );
    assert_eq!(
        QuantileGenerator::new(0., 17., 100)
            .min_by(compare_floats)
            .unwrap(),
        17.
    );
}

#[test]
fn max() {
    assert_eq!(
        QuantileGenerator::new(1., 17., 1)
            .max_by(compare_floats)
            .unwrap(),
        17.
    );
    assert_eq!(
        QuantileGenerator::new(1., 17., 10)
            .max_by(compare_floats)
            .unwrap(),
        17.
    );
    assert_eq!(
        QuantileGenerator::new(1., 17., 100)
            .max_by(compare_floats)
            .unwrap(),
        17.
    );
}

fn check_quantile(quantile: f64, value: f64, num: usize) {
    // Create iterator
    let gen = QuantileGenerator::new(quantile, value, num);

    // Collect iterator into a vector
    let mut values: Vec<f64> = gen.collect();

    // Calculate observed quantile
    values.sort_by(compare_floats);
    let rank: usize = (quantile * (num - 1) as f64).ceil() as usize;
    let actual = values[rank];

    assert_eq!(value, actual);
}

fn compare_floats(a: &f64, b: &f64) -> Ordering {
    a.partial_cmp(b).unwrap()
}