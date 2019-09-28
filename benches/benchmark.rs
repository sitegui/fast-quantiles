#[macro_use]
extern crate criterion;
extern crate space_efficient_quantile;


use criterion::black_box;
use criterion::{BenchmarkId, Criterion};
use space_efficient_quantile::*;

pub fn quantile_generator_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("quantile_generator");
    let nums: Vec<usize> = vec![1_000, 1_000_000];
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
    let nums: Vec<usize> = vec![1_000, 1_000_000];
    for num in nums {
        if num < 1_000_000 {
            group.bench_with_input(BenchmarkId::new("GK", num), &num, |b, &num| {
                b.iter(|| {
                    let mut sum = gk::Summary::new(0.01);
                    for value in quantile_generator::RandomGenerator::new(0.5, 17., num, 17) {
                        sum.insert_one(value);
                    }
                    assert_ne!(sum.query(0.5), None);
                })
            });
        }

        group.bench_with_input(BenchmarkId::new("Modified GK", num), &num, |b, &num| {
            b.iter(|| {
                let mut sum = modified_gk::Summary::new(0.01);
                for value in quantile_generator::RandomGenerator::new(0.5, 17., num, 17) {
                    sum.insert_one(value);
                }
                assert_ne!(sum.query(0.5), None);
            })
        });

        group.bench_with_input(
            BenchmarkId::new("Batch modified GK", num),
            &num,
            |b, &num| {
                b.iter(|| {
                    let mut writer = modified_gk::SummaryWriter::new(0.01);
                    writer.extend(quantile_generator::RandomGenerator::new(0.5, 17., num, 17));
                    let sum = writer.into_summary();
                    assert_ne!(sum.query(0.5), None);
                })
            },
        );
    }
}

criterion_group!(benches, quantile_generator_benchmark, summary_benchmark);
criterion_main!(benches);