# logcompress

Query-aware log compression for LLM context. Feed 100k log lines → get back only the relevant ones.

## The problem

LLM context windows are finite. Dumping 100k raw log lines into a prompt wastes tokens on noise while burying the signal. `grep` gives you exact matches but no context. `jq` gives you structure but no relevance ranking.

**logcompress** takes a natural language query and a log file, scores every line by TF-IDF relevance, follows trace IDs across the incident, and returns a compressed view — typically 100-200 lines out of 100k.

## Quick start

```bash
cargo install logcompress-cli

# Find why a payment timed out
logcompress search -q "payment gateway timeout" -f app.log

# JSON output with stats
logcompress search -q "OOM heap exhausted" -f app.log --format json --stats

# Pipe from stdin
kubectl logs deployment/payment-svc | logcompress search -q "deadlock" -f -
```

## How it works

1. **Tokenize + index** — byte-level FNV hash tokenization, inverted index (zero String allocation)
2. **Score** — TF-IDF cosine similarity between query and each candidate line
3. **Field boost** — JSON fields like `trace_id`, `error_id` get score multipliers when they overlap with query tokens
4. **Trace expansion** — after scoring, extract `trace_id` / `request_id` from top hits and pull all related lines
5. **Score gap filter** — drop lines scoring < 25% of top score (removes common-token noise)
6. **Context windows** — expand ±N lines around each hit, merge overlapping ranges

## Benchmark results

Measured on Apple M-series, 100k JSON log lines (25MB), 50 hidden incidents across 5 types.

| Metric | Value |
|--------|-------|
| **Recall** | 100% (55/55 queries) |
| **Precision** | 9.5% avg (intentional — includes trace and context expansion) |
| **Latency (1k lines)** | 2.4ms |
| **Latency (10k lines)** | 23ms |
| **Latency (100k lines)** | < 250ms |
| **Compression** | 100k → 100-200 lines (0.1-0.2%) |

### Comparison

| Tool | 100k lines | Output | Relevance-ranked |
|------|-----------|--------|-----------------|
| `grep "timeout"` | ~50ms | exact matches only | No |
| `jq 'select(.level=="ERROR")'` | ~500ms | all errors (thousands) | No |
| **logcompress** | ~250ms | 100-200 relevant lines | Yes |

## CLI

```
logcompress search [OPTIONS]

Options:
  -q, --query <QUERY>      Search query (required)
  -f, --file <FILE>        Log file path, or - for stdin (required)
      --top <TOP>          Max result lines [default: 50]
      --context <CONTEXT>  Context lines ±N [default: 2]
      --format <FORMAT>    Output format: text or json [default: text]
      --stats              Print stats (input lines, output lines, time)
      --no-trace           Disable trace-based expansion
```

## Library usage

```rust
use logcompress::{compress, CompressConfig};

let logs = std::fs::read_to_string("app.log").unwrap();
let result = compress("payment timeout", &logs, &CompressConfig::default());

for line in &result.lines {
    println!("{}: {} (score={:.2})", line.line_number, line.content, line.score);
}
```

## Configuration

```rust
CompressConfig {
    top_k: 200,              // max scored lines before filtering
    context_lines: 2,         // ±N context around each hit
    min_score: 0.01,          // minimum TF-IDF score threshold
    max_output_lines: 1000,   // hard cap on output
    trace_expansion: true,    // follow trace_id across the incident
    trace_fields: vec![       // fields to use for trace expansion
        "trace_id", "request_id", "correlation_id"
    ],
    field_boosts: {           // score multipliers for matching fields
        "error_id": 3.0, "trace_id": 3.0,
        "span_id": 2.5, "request_id": 2.5,
        "order_id": 2.0, "payment_id": 2.0,
    },
    always_include_errors: true,  // always include ERROR/FATAL lines
    format: InputFormat::Auto,    // auto-detect JSON vs plain text
}
```

## Building from source

```bash
git clone https://github.com/TimurRakhmatullin86/logcompress
cd logcompress
cargo build --release
./target/release/logcompress search -q "your query" -f your.log
```

## Running tests

```bash
cargo test --lib           # 34 unit tests
cargo bench                # criterion benchmarks

# Generate benchmark dataset and run recall test
cd testdata/benchmark
rustc generate.rs -o generate && ./generate
cd ../..
cargo test --release --test benchmark_recall
```

## License

MIT OR Apache-2.0
