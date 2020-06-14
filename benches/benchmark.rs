use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::prelude::*;

use chainmap::ChainMap;
use std::collections::HashMap;

fn insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("Insert");
    let mut rng = rand::thread_rng();
    let mut ch = ChainMap::new();
    group.bench_function("chainmap", |b| b.iter(|| ch.insert(rng.gen_range(0, 1000), rng.gen::<char>())));
    let mut h = HashMap::new();
    group.bench_function("hashmap", |b| b.iter(|| h.insert(rng.gen_range(0, 1000), rng.gen::<char>())));
    group.finish();
}

criterion_group!(benches, insert);
criterion_main!(benches);
