use criterion::{black_box, criterion_group, criterion_main, Criterion};
use logcompress::{compress, compress_default, CompressConfig};

fn generate_json_lines(count: usize) -> String {
    let mut lines = Vec::with_capacity(count);
    for i in 0..count {
        let level = match i % 100 {
            42 => "ERROR",
            0 | 50 => "WARN",
            _ => "INFO",
        };
        let msg = match i % 100 {
            42 => "payment 42 failed: timeout connecting to payment gateway",
            41 => "payment 42 processing: sending request to gateway",
            43 => "payment 42 retry: scheduling retry in 5s",
            _ => "health check OK",
        };
        lines.push(format!(
            r#"{{"timestamp":"2026-09-06T10:{:02}:00Z","level":"{level}","service":"payment","message":"{msg}","trace_id":"tr-{i:06}"}}"#,
            i % 60,
        ));
    }
    lines.join("\n")
}

fn bench_100k(c: &mut Criterion) {
    let logs = generate_json_lines(100_000);
    c.bench_function("compress_100k_json_lines", |b| {
        b.iter(|| compress_default(black_box("why did payment 42 fail?"), black_box(&logs)))
    });
}

fn bench_10k(c: &mut Criterion) {
    let logs = generate_json_lines(10_000);
    c.bench_function("compress_10k_json_lines", |b| {
        b.iter(|| compress_default(black_box("payment 42 failed"), black_box(&logs)))
    });
}

fn bench_1k(c: &mut Criterion) {
    let logs = generate_json_lines(1_000);
    c.bench_function("compress_1k_json_lines", |b| {
        b.iter(|| compress_default(black_box("payment timeout error"), black_box(&logs)))
    });
}

fn bench_no_errors_config(c: &mut Criterion) {
    let logs = generate_json_lines(10_000);
    let config = CompressConfig {
        always_include_errors: false,
        ..Default::default()
    };
    c.bench_function("compress_10k_no_error_include", |b| {
        b.iter(|| {
            compress(
                black_box("payment 42 failed"),
                black_box(&logs),
                black_box(&config),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_1k,
    bench_10k,
    bench_100k,
    bench_no_errors_config
);
criterion_main!(benches);
