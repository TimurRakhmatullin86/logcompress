#!/usr/bin/env -S cargo +nightly -Zscript
//! Benchmark dataset generator for LogCompress recall measurement.
//! Generates 100k JSON log lines with 50 hidden incidents + ground truth.
//!
//! Usage: rustc generate.rs -o generate && ./generate
//! Or:    rust-script generate.rs

use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};

fn main() {
    let total_lines: usize = 100_000;
    let seed: u64 = 42;
    let mut rng = SimpleRng::new(seed);

    let incidents = build_incidents(total_lines, &mut rng);

    let incident_lines: HashMap<usize, &Incident> = incidents
        .iter()
        .flat_map(|inc| inc.lines.iter().map(move |&l| (l, inc)))
        .collect();

    let services = ["payment-gateway", "order-service", "user-auth", "inventory", "notification"];
    let normal_messages = [
        "Health check passed",
        "Request processed successfully",
        "Cache hit for session lookup",
        "Database connection pool: 12/50 active",
        "Metrics exported to prometheus",
        "gRPC keepalive sent",
        "TLS handshake completed",
        "Rate limiter: 847/1000 tokens remaining",
        "Config reload: no changes detected",
        "Upstream latency p99: 12ms",
        "Queue depth: 3 messages pending",
        "Worker thread pool: 8/16 busy",
        "DNS resolution cached for payment.provider.com",
        "Load balancer health: all backends UP",
        "Garbage collection: 2.1ms pause",
        "Connection recycled after 300s idle",
        "Span exported to Jaeger",
        "Feature flag evaluated: dark_launch=false",
        "Circuit breaker: closed (0 failures)",
        "Batch write: 50 rows in 3ms",
    ];
    let endpoints = [
        "/api/v1/payments", "/api/v1/orders", "/api/v1/users",
        "/api/v1/inventory", "/api/v1/notifications", "/api/v1/health",
        "/api/v2/checkout", "/api/v1/refunds", "/api/v1/webhooks",
        "/internal/metrics",
    ];

    let f = fs::File::create("logs_100k.jsonl").expect("create logs file");
    let mut w = BufWriter::with_capacity(1 << 20, f);

    for i in 0..total_lines {
        let ts = format!(
            "2026-09-06T{:02}:{:02}:{:02}.{:03}Z",
            (i / 3600) % 24,
            (i / 60) % 60,
            i % 60,
            rng.next_range(0, 999)
        );

        if let Some(inc) = incident_lines.get(&i) {
            let line = inc.generate_line(i, &ts, &mut rng);
            writeln!(w, "{}", line).unwrap();
        } else {
            let service = services[rng.next_range(0, services.len() as u64) as usize];
            let msg = normal_messages[rng.next_range(0, normal_messages.len() as u64) as usize];
            let endpoint = endpoints[rng.next_range(0, endpoints.len() as u64) as usize];
            let trace_id = format!("tr-{:016x}", rng.next());
            let span_id = format!("sp-{:08x}", rng.next() & 0xFFFFFFFF);
            let latency = rng.next_range(1, 50);

            writeln!(
                w,
                r#"{{"timestamp":"{}","level":"INFO","service":"{}","message":"{}","trace_id":"{}","span_id":"{}","endpoint":"{}","latency_ms":{},"request_id":"req-{:012x}"}}"#,
                ts, service, msg, trace_id, span_id, endpoint, latency,
                rng.next() & 0xFFFFFFFFFFFF
            ).unwrap();
        }
    }
    w.flush().unwrap();

    let gt = build_ground_truth(&incidents);
    fs::write("ground_truth.json", serde_json_mini(&gt)).expect("write ground truth");

    eprintln!("Generated {} lines with {} incidents", total_lines, incidents.len());
    eprintln!("Incident types: timeout={}, oom={}, deadlock={}, auth_failure={}, rate_limit={}",
        incidents.iter().filter(|i| i.kind == IncidentKind::Timeout).count(),
        incidents.iter().filter(|i| i.kind == IncidentKind::Oom).count(),
        incidents.iter().filter(|i| i.kind == IncidentKind::Deadlock).count(),
        incidents.iter().filter(|i| i.kind == IncidentKind::AuthFailure).count(),
        incidents.iter().filter(|i| i.kind == IncidentKind::RateLimit).count(),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncidentKind {
    Timeout,
    Oom,
    Deadlock,
    AuthFailure,
    RateLimit,
}

struct Incident {
    id: usize,
    kind: IncidentKind,
    lines: Vec<usize>,
    trace_id: String,
    service: &'static str,
    order_id: String,
}

impl Incident {
    fn generate_line(&self, line_idx: usize, ts: &str, rng: &mut SimpleRng) -> String {
        let pos = self.lines.iter().position(|&l| l == line_idx).unwrap();
        let total = self.lines.len();

        match self.kind {
            IncidentKind::Timeout => {
                self.timeout_line(pos, total, ts, rng)
            }
            IncidentKind::Oom => {
                self.oom_line(pos, total, ts, rng)
            }
            IncidentKind::Deadlock => {
                self.deadlock_line(pos, total, ts, rng)
            }
            IncidentKind::AuthFailure => {
                self.auth_failure_line(pos, total, ts, rng)
            }
            IncidentKind::RateLimit => {
                self.rate_limit_line(pos, total, ts, rng)
            }
        }
    }

    fn timeout_line(&self, pos: usize, total: usize, ts: &str, rng: &mut SimpleRng) -> String {
        let span = format!("sp-{:08x}", rng.next() & 0xFFFFFFFF);
        match pos {
            0 => format!(
                r#"{{"timestamp":"{}","level":"INFO","service":"{}","message":"Initiating payment request to gateway","trace_id":"{}","span_id":"{}","order_id":"{}","endpoint":"/api/v1/payments","latency_ms":{}}}"#,
                ts, self.service, self.trace_id, span, self.order_id, rng.next_range(5, 20)
            ),
            p if p == total - 1 => format!(
                r#"{{"timestamp":"{}","level":"ERROR","service":"{}","message":"Payment gateway timeout after 30000ms: connection to payment.provider.com timed out, order {} stuck in pending state","trace_id":"{}","span_id":"{}","order_id":"{}","error_id":"err-timeout-{}","endpoint":"/api/v1/payments","latency_ms":30000}}"#,
                ts, self.service, self.order_id, self.trace_id, span, self.order_id, self.id
            ),
            p if p == total - 2 => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"Gateway response slow, approaching timeout threshold 30s","trace_id":"{}","span_id":"{}","order_id":"{}","endpoint":"/api/v1/payments","latency_ms":{}}}"#,
                ts, self.service, self.trace_id, span, self.order_id, rng.next_range(25000, 29000)
            ),
            _ => format!(
                r#"{{"timestamp":"{}","level":"INFO","service":"{}","message":"Retrying payment gateway connection, attempt {}","trace_id":"{}","span_id":"{}","order_id":"{}","endpoint":"/api/v1/payments","latency_ms":{}}}"#,
                ts, self.service, pos, self.trace_id, span, self.order_id, rng.next_range(1000, 5000)
            ),
        }
    }

    fn oom_line(&self, pos: usize, total: usize, ts: &str, rng: &mut SimpleRng) -> String {
        let span = format!("sp-{:08x}", rng.next() & 0xFFFFFFFF);
        let heap_mb = 512 + pos as u64 * 128;
        match pos {
            0 => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"Heap usage elevated: {}MB / 2048MB, GC pressure increasing","trace_id":"{}","span_id":"{}","order_id":"{}","heap_used_mb":{}}}"#,
                ts, self.service, heap_mb, self.trace_id, span, self.order_id, heap_mb
            ),
            p if p == total - 1 => format!(
                r#"{{"timestamp":"{}","level":"FATAL","service":"{}","message":"OutOfMemoryError: Java heap space exhausted, process killed by OOM killer, allocated 2048MB","trace_id":"{}","span_id":"{}","order_id":"{}","error_id":"err-oom-{}","heap_used_mb":2048}}"#,
                ts, self.service, self.trace_id, span, self.order_id, self.id
            ),
            _ => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"Memory allocation spike: large batch processing {}MB heap, GC unable to free sufficient memory","trace_id":"{}","span_id":"{}","order_id":"{}","heap_used_mb":{}}}"#,
                ts, self.service, heap_mb, self.trace_id, span, self.order_id, heap_mb
            ),
        }
    }

    fn deadlock_line(&self, pos: usize, total: usize, ts: &str, rng: &mut SimpleRng) -> String {
        let span = format!("sp-{:08x}", rng.next() & 0xFFFFFFFF);
        match pos {
            0 => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"Lock acquisition timeout: thread waiting for mutex on account_balance table","trace_id":"{}","span_id":"{}","order_id":"{}","endpoint":"/api/v1/payments","latency_ms":{}}}"#,
                ts, self.service, self.trace_id, span, self.order_id, rng.next_range(5000, 10000)
            ),
            p if p == total - 1 => format!(
                r#"{{"timestamp":"{}","level":"ERROR","service":"{}","message":"Deadlock detected: circular wait between transaction {} and concurrent update on account_balance, both threads blocked for >60s","trace_id":"{}","span_id":"{}","order_id":"{}","error_id":"err-deadlock-{}","endpoint":"/api/v1/payments"}}"#,
                ts, self.service, self.order_id, self.trace_id, span, self.order_id, self.id
            ),
            _ => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"Lock contention increasing: {} threads waiting for account_balance mutex","trace_id":"{}","span_id":"{}","order_id":"{}","endpoint":"/api/v1/payments","latency_ms":{}}}"#,
                ts, self.service, pos + 2, self.trace_id, span, self.order_id, rng.next_range(10000, 30000)
            ),
        }
    }

    fn auth_failure_line(&self, pos: usize, total: usize, ts: &str, rng: &mut SimpleRng) -> String {
        let span = format!("sp-{:08x}", rng.next() & 0xFFFFFFFF);
        match pos {
            0 => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"JWT token validation failed: signature mismatch for user session","trace_id":"{}","span_id":"{}","order_id":"{}","user_id":"usr-{}","endpoint":"/api/v1/payments"}}"#,
                ts, self.service, self.trace_id, span, self.order_id, rng.next_range(10000, 99999)
            ),
            p if p == total - 1 => format!(
                r#"{{"timestamp":"{}","level":"ERROR","service":"{}","message":"Authentication failure: invalid API key or expired token, payment {} rejected, possible credential compromise","trace_id":"{}","span_id":"{}","order_id":"{}","error_id":"err-auth-{}","endpoint":"/api/v1/payments"}}"#,
                ts, self.service, self.order_id, self.trace_id, span, self.order_id, self.id
            ),
            _ => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"Auth retry {}: re-validating credentials against auth service","trace_id":"{}","span_id":"{}","order_id":"{}","endpoint":"/api/v1/payments","latency_ms":{}}}"#,
                ts, self.service, pos, self.trace_id, span, self.order_id, rng.next_range(100, 500)
            ),
        }
    }

    fn rate_limit_line(&self, pos: usize, total: usize, ts: &str, rng: &mut SimpleRng) -> String {
        let span = format!("sp-{:08x}", rng.next() & 0xFFFFFFFF);
        let remaining = if pos == 0 { 5 } else { 0 };
        match pos {
            0 => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"Rate limit approaching: {}/1000 requests remaining in current window","trace_id":"{}","span_id":"{}","order_id":"{}","endpoint":"/api/v1/payments","rate_limit_remaining":{}}}"#,
                ts, self.service, remaining, self.trace_id, span, self.order_id, remaining
            ),
            p if p == total - 1 => format!(
                r#"{{"timestamp":"{}","level":"ERROR","service":"{}","message":"Rate limit exceeded: 429 Too Many Requests from payment gateway, order {} queued, retry after 60s","trace_id":"{}","span_id":"{}","order_id":"{}","error_id":"err-ratelimit-{}","endpoint":"/api/v1/payments","rate_limit_remaining":0}}"#,
                ts, self.service, self.order_id, self.trace_id, span, self.order_id, self.id
            ),
            _ => format!(
                r#"{{"timestamp":"{}","level":"WARN","service":"{}","message":"Rate limit throttling: request queued, position {} in backoff queue","trace_id":"{}","span_id":"{}","order_id":"{}","endpoint":"/api/v1/payments","latency_ms":{}}}"#,
                ts, self.service, pos, self.trace_id, span, self.order_id, rng.next_range(1000, 5000)
            ),
        }
    }
}

fn build_incidents(total_lines: usize, rng: &mut SimpleRng) -> Vec<Incident> {
    let kinds = [
        IncidentKind::Timeout,
        IncidentKind::Oom,
        IncidentKind::Deadlock,
        IncidentKind::AuthFailure,
        IncidentKind::RateLimit,
    ];
    let services = ["payment-gateway", "order-service", "user-auth", "inventory", "notification"];

    let mut incidents = Vec::with_capacity(50);
    let mut used_positions: Vec<usize> = Vec::new();

    for id in 0..50 {
        let kind = kinds[id % 5];
        let service = services[id % 5];
        let lines_per_incident = rng.next_range(3, 7) as usize;

        let mut start;
        loop {
            start = rng.next_range(100, (total_lines - 100) as u64) as usize;
            let end = start + lines_per_incident;
            let conflict = used_positions.iter().any(|&p| {
                start <= p + 10 && end >= p.saturating_sub(10)
            });
            if !conflict {
                break;
            }
        }

        let lines: Vec<usize> = (start..start + lines_per_incident).collect();
        for &l in &lines {
            used_positions.push(l);
        }

        let trace_id = format!("tr-inc-{:04}-{:08x}", id, rng.next() & 0xFFFFFFFF);
        let order_id = format!("ord-{:06}", 100000 + id);

        incidents.push(Incident {
            id,
            kind,
            lines,
            trace_id,
            service,
            order_id,
        });
    }

    incidents
}

fn build_ground_truth(incidents: &[Incident]) -> String {
    let mut entries = Vec::new();

    // Per-incident queries — include order_id as a discriminator (realistic: oncall debugs a specific order)
    for inc in incidents {
        let query = match inc.kind {
            IncidentKind::Timeout => format!("payment gateway timeout {}", inc.order_id),
            IncidentKind::Oom => format!("OutOfMemoryError OOM heap exhausted {}", inc.order_id),
            IncidentKind::Deadlock => format!("deadlock circular wait blocked {}", inc.order_id),
            IncidentKind::AuthFailure => format!("authentication failure invalid token {}", inc.order_id),
            IncidentKind::RateLimit => format!("rate limit exceeded 429 {}", inc.order_id),
        };

        let line_numbers: Vec<String> = inc.lines.iter().map(|l| (l + 1).to_string()).collect();

        entries.push(format!(
            r#"    {{"query": "{}", "incident_id": {}, "incident_kind": "{:?}", "expected_lines": [{}], "trace_id": "{}", "order_id": "{}"}}"#,
            query, inc.id, inc.kind, line_numbers.join(", "), inc.trace_id, inc.order_id
        ));
    }

    // Cross-incident aggregate queries (one per kind)
    let kinds = [
        (IncidentKind::Timeout, "timeout payment gateway connection timed out"),
        (IncidentKind::Oom, "OOM OutOfMemoryError heap memory killed"),
        (IncidentKind::Deadlock, "deadlock mutex circular wait threads blocked"),
        (IncidentKind::AuthFailure, "auth failure invalid token JWT credential"),
        (IncidentKind::RateLimit, "rate limit 429 Too Many Requests throttle"),
    ];

    for (kind, query) in &kinds {
        let mut all_lines: Vec<usize> = Vec::new();
        let mut incident_ids: Vec<usize> = Vec::new();
        for inc in incidents {
            if inc.kind == *kind {
                all_lines.extend(&inc.lines);
                incident_ids.push(inc.id);
            }
        }
        all_lines.sort();
        let line_numbers: Vec<String> = all_lines.iter().map(|l| (l + 1).to_string()).collect();
        let ids: Vec<String> = incident_ids.iter().map(|i| i.to_string()).collect();

        entries.push(format!(
            r#"    {{"query": "{}", "aggregate": true, "incident_ids": [{}], "incident_kind": "{:?}", "expected_lines": [{}]}}"#,
            query, ids.join(", "), kind, line_numbers.join(", ")
        ));
    }

    format!("[\n{}\n]", entries.join(",\n"))
}

fn serde_json_mini(s: &str) -> String {
    s.to_string()
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_range(&mut self, min: u64, max: u64) -> u64 {
        if max <= min {
            return min;
        }
        min + (self.next() % (max - min))
    }
}
