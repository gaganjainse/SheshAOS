//! Criterion benchmarks for the append-only kernel event store.

#![allow(clippy::unwrap_used, clippy::expect_used)]
// Reason: criterion bench targets are not covered by clippy.toml's
// allow-*-in-tests (which handles #[test]/tests/ code). Bench setup failures
// must panic loudly anyway; the no-unwrap policy targets production paths.

use criterion::{criterion_group, criterion_main, Criterion};
use shesh_kernel::events::{Event, EventKind, EventPayload};
use shesh_kernel::storage::event_store::JsonlEventStore;
use shesh_kernel::task::TaskId;

fn bench_event_store(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let tmp = tempfile::tempdir().expect("tempdir");
    let store =
        rt.block_on(JsonlEventStore::open(tmp.path().to_path_buf())).expect("open event store");

    c.bench_function("bench_event_store/append", |b| {
        b.iter(|| {
            let mut ev = Event::new(
                TaskId::new(),
                EventKind::TaskCreated,
                EventPayload::SystemEvent { message: "bench".to_string() },
                "bench".to_string(),
            );
            rt.block_on(store.append(std::hint::black_box(&mut ev))).expect("append");
        })
    });

    // Seed for read benches.
    rt.block_on(async {
        for _ in 0..500 {
            let mut ev = Event::new(
                TaskId::new(),
                EventKind::TaskClassified,
                EventPayload::SystemEvent { message: "seed".to_string() },
                "bench".to_string(),
            );
            store.append(&mut ev).await.expect("seed append");
        }
    });

    c.bench_function("bench_event_store/read_all_500", |b| {
        b.iter(|| {
            let events = rt.block_on(store.read_all()).expect("read_all");
            std::hint::black_box(events);
        })
    });

    c.bench_function("bench_event_store/read_since_tail", |b| {
        b.iter(|| {
            let events =
                rt.block_on(store.read_since(std::hint::black_box(490))).expect("read_since");
            std::hint::black_box(events);
        })
    });
}

criterion_group!(benches, bench_event_store);
criterion_main!(benches);
