//! Criterion benchmarks for WaveObj persistence hot paths.

#![allow(clippy::unwrap_used, clippy::expect_used)]
// Reason: criterion bench targets are not covered by clippy.toml's
// allow-*-in-tests (which handles #[test]/tests/ code). Bench setup failures
// must panic loudly anyway; the no-unwrap policy targets production paths.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use shesh_waveobj::meta::MetaMap;
use shesh_waveobj::store::WaveStore;
use shesh_waveobj::types::Block;
use uuid::Uuid;

fn new_block() -> Block {
    Block {
        oid: Uuid::new_v4(),
        parent_oref: None,
        version: 0,
        runtime_opts: None,
        stickers: None,
        meta: MetaMap::new(),
        sub_block_ids: vec![],
        job_id: None,
    }
}

fn bench_wavestore(c: &mut Criterion) {
    let store = WaveStore::open_in_memory().expect("open in-memory store");

    c.bench_function("bench_wavestore/db_insert", |b| {
        b.iter(|| {
            let mut block = new_block();
            store.db_insert(black_box(&mut block)).expect("insert");
        })
    });

    // Seed for read benches.
    let mut ids = Vec::with_capacity(500);
    for _ in 0..500 {
        let mut block = new_block();
        store.db_insert(&mut block).expect("seed insert");
        ids.push(block.oid);
    }

    c.bench_function("bench_wavestore/db_get_500", |b| {
        b.iter(|| {
            for oid in &ids {
                let block: Option<Block> = store.db_get(black_box(oid)).expect("get");
                black_box(block);
            }
        })
    });
}

criterion_group!(benches, bench_wavestore);
criterion_main!(benches);
