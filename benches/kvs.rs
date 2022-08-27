use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use kvs::{KvStore, KvsEngine, Result, SledKvsEngine};
use tempfile::TempDir;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("set_bench");

    group.bench_function("kvs_set", |b| {
        b.iter_batched(
            || {
                let temp_dir =
                    TempDir::new().expect("unable to create temporary working directory");
                (
                    KvStore::open(temp_dir.path()).expect("could not open KvStore"),
                    temp_dir,
                )
            },
            |(mut store, _temp_dir)| {
                for i in 1..(1 << 12) {
                    store.set(format!("key{}", i), "value".to_string()).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function("sled_set", |b| {
        b.iter_batched(
            || {
                let temp_dir =
                    TempDir::new().expect("unable to create temporary working directory");
                (SledKvsEngine::new(&temp_dir).unwrap(), temp_dir)
            },
            |(mut store, _temp_dir)| {
                for i in 1..(1 << 12) {
                    store.set(format!("key{}", i), "value".to_string()).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
