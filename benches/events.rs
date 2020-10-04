use async_std::sync::{Arc, Mutex};
use async_std::task;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use utrakr_api::events::event_logger::EventLogger;
use utrakr_api::events::event_reader::EventReader;
use utrakr_api::events::ulid::UlidGenerator;

fn criterion_benchmark(c: &mut Criterion) {
    task::block_on(async {
        // bench writer
        let tmp = tempfile::tempdir().unwrap().into_path();
        let gen = Arc::new(Mutex::new(UlidGenerator::new()));
        let writer: EventLogger = EventLogger::new(&tmp, "test", gen).await.unwrap();
        c.bench_function("writer", |b| {
            b.iter(|| {
                task::block_on(async { writer.log_event("cat", black_box("evt")).await.unwrap() })
            })
        });

        // bench reader
        let tmp = tempfile::tempdir().unwrap().into_path();
        let gen = Arc::new(Mutex::new(UlidGenerator::new()));
        let writer: EventLogger = EventLogger::new(&tmp, "test", gen).await.unwrap();
        let reader: EventReader = EventReader::new(&tmp);
        for _ in 0..10 {
            writer.log_event("cat", "start").await.unwrap();
        }
        c.bench_function("reader", |b| {
            b.iter(|| {
                for e in reader.iter::<String>() {
                    let _e = black_box(e);
                }
            })
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
