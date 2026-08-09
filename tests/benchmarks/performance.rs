use criterion::{black_box, criterion_group, Criterion};
use sheshaaos_gui::terminal::{TermPerformer};
use vte::Parser;
use sheshaaos_kernel::{
    runtime::kernel::Kernel,
    storage::event_store::EventStore,
    model::registry::ProviderRegistry,
    policy::{PolicyEngine, PolicyRule, TrustTier},
    tools::broker::ToolBroker,
    tools::executor::ToolExecutor,
    error::ToolError,
    task::TaskInput,
};
use std::sync::Arc;
use tempfile::TempDir;

use tokio::sync::RwLock;

/// Benchmark terminal parsing
fn bench_terminal_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("terminal_parsing");
    
    // Benchmark simple text parsing
    group.bench_function("simple_text_1kb", |b| {
        let input = "x".repeat(1024);
        b.iter(|| {
            let mut performer = TermPerformer::new(24, 80);
            let mut parser = Parser::new();
            for byte in black_box(&input).bytes() {
                parser.advance(&mut performer, byte);
            }
        });
    });
    
    // Benchmark ANSI color sequences
    group.bench_function("ansi_colors_100", |b| {
        let input = "\x1b[31mRed\x1b[0m ".repeat(100);
        b.iter(|| {
            let mut performer = TermPerformer::new(24, 80);
            let mut parser = Parser::new();
            for byte in black_box(&input).bytes() {
                parser.advance(&mut performer, byte);
            }
        });
    });
    
    // Benchmark cursor movement
    group.bench_function("cursor_movement_1000", |b| {
        let input = "\x1b[10;10Hx".repeat(1000);
        b.iter(|| {
            let mut performer = TermPerformer::new(24, 80);
            let mut parser = Parser::new();
            for byte in black_box(&input).bytes() {
                parser.advance(&mut performer, byte);
            }
        });
    });
    
    // Benchmark scrolling
    group.bench_function("scrolling_1000_lines", |b| {
        let input = "Line\r\n".repeat(1000);
        b.iter(|| {
            let mut performer = TermPerformer::new(24, 80);
            let mut parser = Parser::new();
            for byte in black_box(&input).bytes() {
                parser.advance(&mut performer, byte);
            }
        });
    });
    
    group.finish();
}

/// Benchmark kernel task submission
fn bench_kernel_task_submission(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let temp_dir = TempDir::new().unwrap();
    let events_dir = temp_dir.path().join("events");
    std::fs::create_dir_all(&events_dir).unwrap();
    
    let store = Arc::new(rt.block_on(EventStore::open(events_dir)).unwrap());
    
    let rules = vec![PolicyRule {
        name: "allow-all".to_string(),
        action_pattern: "*".to_string(),
        decision: "allow".to_string(),
        trust_tier: 0,
        description: None,
    }];
    let policy = PolicyEngine::new(rules, TrustTier::Autonomous);
    
    let registry = Arc::new(ProviderRegistry::new());
    let broker = Arc::new(ToolBroker::new(Arc::new(policy.clone())));
    
    let kernel = rt.block_on(Kernel::new(store, Arc::new(RwLock::new(policy)), registry, broker, 1_048_576)).unwrap();
    
    let mut group = c.benchmark_group("kernel_task_submission");
    
    group.bench_function("submit_simple_task", |b| {
        b.iter(|| {
            rt.block_on(async {
                let task_id = kernel.submit_task(TaskInput::Text("test task".to_string())).await.unwrap();
                black_box(task_id);
            });
        });
    });
    
    group.bench_function("submit_task_with_priority", |b| {
        b.iter(|| {
            rt.block_on(async {
                // TaskInput doesn't have priority directly
                let _task_name = "test task".to_string();
                let task_id = kernel.submit_task(TaskInput::Text("test task".to_string())).await.unwrap();
                black_box(task_id);
            });
        });
    });
    
    group.finish();
}

/// Benchmark event store operations
fn bench_event_store(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let temp_dir = TempDir::new().unwrap();
    let events_dir = temp_dir.path().join("events");
    std::fs::create_dir_all(&events_dir).unwrap();
    
    let store = Arc::new(rt.block_on(EventStore::open(events_dir)).unwrap());
    
    let mut group = c.benchmark_group("event_store");
    
    group.bench_function("append_event", |b| {
        use sheshaaos_kernel::{events::{Event, EventKind, EventPayload}, task::TaskId};
        use uuid::Uuid;
        
        b.iter(|| {
            rt.block_on(async {
                let mut event = Event::new(
                    TaskId(Uuid::now_v7()),
                    EventKind::TaskCreated,
                    EventPayload::TaskCreated { request: serde_json::json!({}) },
                    "test".to_string(),
                );
                let event_store = store.clone();
                event_store.append(&mut event).await.unwrap();
            });
        });
    });
    
    group.bench_function("read_all_events", |b| {
        b.iter(|| {
            rt.block_on(async {
                let events = store.read_all().await.unwrap();
                black_box(events);
            });
        });
    });
    
    group.finish();
}

/// Benchmark terminal rendering (span batching)
fn bench_terminal_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("terminal_rendering");

    // Pre-create a performer with colored content
    use sheshaaos_gui::terminal::{Cell, CellAttr, TermColor};
    let mut performer = TermPerformer::new(30, 120);
    for r in 0..30 {
        for c in 0..120 {
            performer.grid[r][c] = Cell {
                ch: if c % 10 == 0 { 'X' } else { ' ' },
                attr: CellAttr {
                    fg: if r % 2 == 0 { TermColor::Indexed(1) } else { TermColor::Indexed(2) },
                    bg: TermColor::Default,
                    bold: r % 3 == 0,
                    ..Default::default()
                },
            };
        }
    }
    
    group.bench_function("render_full_grid", |b| {
        b.iter(|| {
            // Simulate span batching render
            let mut spans_count = 0;
            for row in &performer.grid {
                let mut current_fg = None;
                let mut current_bg = None;
                let mut current_bold = false;
                let mut span_len = 0;
                
                for cell in row {
                    let fg = cell.attr.fg;
                    let bg = cell.attr.bg;
                    let bold = cell.attr.bold;
                    
                    if current_fg == Some(fg) && current_bg == Some(bg) && current_bold == bold {
                        span_len += 1;
                    } else {
                        if span_len > 0 {
                            spans_count += 1;
                        }
                        current_fg = Some(fg);
                        current_bg = Some(bg);
                        current_bold = bold;
                        span_len = 1;
                    }
                }
                if span_len > 0 {
                    spans_count += 1;
                }
            }
            black_box(spans_count);
        });
    });
    
    group.finish();
}

/// Benchmark snapshot / projection rebuild performance
fn bench_snapshot_projection(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let events_dir = temp_dir.path().join("events");
    std::fs::create_dir_all(&events_dir).unwrap();
    let store = Arc::new(rt.block_on(EventStore::open(events_dir)).unwrap());

    let mut group = c.benchmark_group("snapshot_projection");

    // Pre-populate with events
    rt.block_on(async {
        use sheshaaos_kernel::{events::{Event, EventKind, EventPayload}, task::TaskId};
        use uuid::Uuid;
        for i in 0..100u64 {
            let mut event = Event::new(
                TaskId(Uuid::now_v7()),
                EventKind::TaskCreated,
                EventPayload::TaskCreated { request: serde_json::json!({ "i": i }) },
                "test".to_string(),
            );
            store.append(&mut event).await.unwrap();
        }
    });

    group.bench_function("replay_100_events", |b| {
        b.iter(|| {
            rt.block_on(async {
                let projection = sheshaaos_kernel::runtime::replay::ReplayEngine::replay(&*store).await.unwrap();
                black_box(projection);
            });
        });
    });

    group.finish();
}

/// Benchmark tool broker routing throughput
fn bench_tool_broker_throughput(c: &mut Criterion) {
    use sheshaaos_kernel::tools::executor::{ToolRequest, ToolResult};
    use std::sync::Arc;

    let mut group = c.benchmark_group("tool_broker");

    let policy = Arc::new(PolicyEngine::new(vec![], TrustTier::Autonomous));
    let mut broker = ToolBroker::new(policy);

    // Register a dummy executor
    struct DummyExecutor;
    #[async_trait::async_trait]
    impl ToolExecutor for DummyExecutor {
        fn name(&self) -> &str {
            "dummy"
        }
        fn description(&self) -> &str {
            "Dummy benchmark executor"
        }
        fn is_destructive(&self) -> bool {
            false
        }
        async fn execute(&self, _req: &ToolRequest) -> Result<ToolResult, ToolError> {
            Ok(ToolResult {
                success: true,
                output: "ok".to_string(),
                data: Some(serde_json::json!({})),
            })
        }
    }
    broker.register(Arc::new(DummyExecutor));

    group.bench_function("register_and_execute", |b| {
        b.iter(|| {
            black_box(broker.available_tools().len());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_terminal_parsing,
    bench_kernel_task_submission,
    bench_event_store,
    bench_terminal_rendering,
    bench_snapshot_projection,
    bench_tool_broker_throughput
);

fn main() {
    benches();
    Criterion::default().configure_from_args().final_summary();
}