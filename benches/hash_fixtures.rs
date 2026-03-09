use std::path::PathBuf;

use criterion::{Criterion, criterion_group, criterion_main};
use telfhash_rs::{TelfhashEngine, TelfhashOptions};

fn benchmark_hash_fixtures(criterion: &mut Criterion) {
    let engine = TelfhashEngine::new();
    let options = TelfhashOptions::default();
    let fixtures = [
        "tests/fixtures/bin/x86_64_dyn_stripped.so",
        "tests/fixtures/bin/i386_dyn_stripped.so",
        "tests/fixtures/bin/arm32_dyn_stripped.so",
        "tests/fixtures/bin/aarch64_dyn_stripped.so",
        "tests/fixtures/bin/arm32_tnull.so",
        "tests/fixtures/bin/x86_64_not_stripped.so",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();

    criterion.bench_function("hash_fixtures", |bencher| {
        bencher.iter(|| engine.hash_paths(fixtures.iter(), &options).unwrap())
    });
}

criterion_group!(benches, benchmark_hash_fixtures);
criterion_main!(benches);
