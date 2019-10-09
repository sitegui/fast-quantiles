#[macro_use]
extern crate criterion;
extern crate space_efficient_quantile;

use criterion::{BenchmarkId, Criterion};
use space_efficient_quantile::*;

pub fn quantile_generator_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("quantile_generator");
    let nums: Vec<usize> = vec![1_000, 10_000, 100_000];
    for num in nums {
        group.bench_with_input(BenchmarkId::new("Random", num), &num, |b, &num| {
            b.iter(|| quantile_generator::RandomGenerator::new(0.5, 17., num, 17))
        });
        group.bench_with_input(BenchmarkId::new("Sequential", num), &num, |b, &num| {
            b.iter(|| {
                quantile_generator::SequentialGenerator::new(
                    0.5,
                    17.,
                    num,
                    quantile_generator::SequentialOrder::Ascending,
                )
            })
        });
    }
}

pub fn summary_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("summary");
    let nums: Vec<usize> = vec![100, 1_000, 10_000, 100_000];
    let epsilon = 0.001;
    for num in nums {
        if num <= 1_000 {
            group.bench_with_input(BenchmarkId::new("GK", num), &num, |b, &num| {
                b.iter(|| {
                    let mut sum = gk::Summary::new(epsilon);
                    for value in quantile_generator::RandomGenerator::new(0.5, 17., num, 17) {
                        sum.insert_one(value);
                    }
                    assert_ne!(sum.query(0.5), None);
                })
            });
        }

        group.bench_with_input(BenchmarkId::new("Modified GK", num), &num, |b, &num| {
            b.iter(|| {
                let mut sum = modified_gk::Summary::new(epsilon);
                for value in quantile_generator::RandomGenerator::new(0.5, 17., num, 17) {
                    sum.insert_one(value);
                }
                assert_ne!(sum.query(0.5), None);
            })
        });

        group.bench_with_input(BenchmarkId::new("Exact naive", num), &num, |b, &num| {
            b.iter(|| {
                let mut values = Vec::with_capacity(num);
                values.extend(quantile_generator::RandomGenerator::new(0.5, 17., num, 17));
                values.sort();
                let median = values[(values.len() - 1) / 2];
                assert_eq!(median.into_inner(), 17.);
            })
        });
    }
}

criterion_group!(benches, quantile_generator_benchmark, summary_benchmark);
criterion_main!(benches);
