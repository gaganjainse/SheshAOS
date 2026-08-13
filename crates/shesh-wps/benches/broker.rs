//! Criterion benchmarks for the pub/sub broker hot paths.

#![allow(clippy::unwrap_used, clippy::expect_used)]
// Reason: criterion bench targets are not covered by clippy.toml's
// allow-*-in-tests (which handles #[test]/tests/ code). Bench setup failures
// must panic loudly anyway; the no-unwrap policy targets production paths.

use criterion::{criterion_group, criterion_main, Criterion};
use shesh_wps::broker::Broker;
use shesh_wps::events::SubscriptionRequest;
use shesh_wps::events::WaveEvent;

fn bench_broker(c: &mut Criterion) {
    c.bench_function("bench_broker_throughput/publish_no_subscribers", |b| {
        let broker = Broker::new(64);
        let mut i = 0u64;
        b.iter(|| {
            broker.publish(WaveEvent::new("block", vec![], serde_json::json!({ "i": i })));
            i += 1;
        })
    });

    c.bench_function("bench_broker_throughput/matching_routes_10subs", |b| {
        let broker = Broker::new(64);
        for n in 0..10 {
            broker.subscribe(
                &format!("route-{n}"),
                SubscriptionRequest { topic: "block".to_string(), scopes: vec![] },
            );
        }
        let ev = WaveEvent::new("block", vec![], serde_json::json!({}));
        b.iter(|| {
            let routes = broker.get_matching_routes(std::hint::black_box(&ev));
            std::hint::black_box(routes);
        })
    });
}

criterion_group!(benches, bench_broker);
criterion_main!(benches);
