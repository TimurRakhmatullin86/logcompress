# Building a Query-Aware Log Compressor in Rust: From 100k Lines to 200

## The Problem

You're on-call. A payment failed. You pull the logs:

```bash
kubectl logs deployment/payment-svc --since=1h | wc -l
# 100,000
```

Now what? You could grep for "timeout" — but that gives you 47 exact matches out of context. You could pipe it into an LLM, but 100k lines blows past any context window. You need the 200 lines that matter.

That's what [logcompress](https://github.com/TimurRakhmatullin86/logcompress) does. Give it a natural language query and a log file, and it returns only the relevant lines — scored, ranked, and expanded with trace context.

```bash
logcompress search -q "why did payment 42 timeout" -f app.log --stats
```

Output: the 200 lines that tell the story, from the initial request through retries to the final timeout error.

## The Architecture

### Step 1: Tokenize (fast)

Every line is tokenized into hash tokens using FNV-1a. No String allocation — we work with `u64` hashes directly:

```rust
fn hash_token(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &b in bytes {
        let b = if b.is_ascii_uppercase() { b + 32 } else { b };
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV prime
    }
    h
}
```

This runs in parallel via rayon. 100k lines tokenize in under 50ms.

### Step 2: Score with TF-IDF

Each candidate line gets a TF-IDF cosine similarity score against the query. This is the standard information retrieval approach — terms that appear rarely in the corpus but match the query get high scores.

But raw TF-IDF has a problem with structured logs: if every JSON line contains `"service":"payment"`, then "payment" appears in every document and gets zero discriminative power. The solution is a **score gap filter**: after scoring, we drop lines scoring less than 25% of the top score. This cleanly separates signal from noise.

### Step 3: JSON Field Boost

If the query mentions "trace-abc123" and a log line has `"trace_id":"trace-abc123"`, that's a stronger signal than the same string appearing in a message. We give configurable multipliers to structured fields:

```
error_id: 3.0x
trace_id: 3.0x
span_id: 2.5x
order_id: 2.0x
```

### Step 4: Trace Expansion (the key feature)

This is where logcompress differs from a simple TF-IDF search.

Consider a timeout incident:

```
Line 1: "Initiating payment request to gateway" (trace_id: tr-001)
Line 2: "Retrying gateway connection, attempt 1" (trace_id: tr-001)
Line 3: "Gateway response slow, approaching timeout" (trace_id: tr-001)
Line 4: "Payment gateway timeout after 30000ms" (trace_id: tr-001)
```

Only Line 4 matches the query "timeout". Lines 1-3 use different vocabulary. But they share the same `trace_id`. After scoring, we extract trace IDs from high-scoring hits and pull all lines sharing those IDs.

This takes recall from ~70% to 100% on our benchmark.

### Step 5: Context Windows

Finally, we expand ±N lines around each hit and merge overlapping ranges. This catches log lines immediately before/after an incident that don't share a trace ID — like the request that triggered the timeout, or the recovery action that followed.

## Benchmark Results

We tested against a 100k-line synthetic dataset with 50 hidden incidents (timeout, OOM, deadlock, auth failure, rate limit):

| Metric | Value |
|--------|-------|
| Recall | 100% (55/55 queries) |
| Latency (100k lines) | < 250ms |
| Compression | 100k → 100-200 lines |

### vs. Alternatives

| Tool | Approach | Relevance |
|------|----------|-----------|
| grep | Exact string match | None |
| jq | Field filtering | None |
| ripgrep | Fast regex | None |
| **logcompress** | TF-IDF + trace expansion | Ranked |

## Usage

Install from source (crates.io publish coming):

```bash
cargo install --git https://github.com/TimurRakhmatullin86/logcompress logcompress-cli
```

As a library:

```rust
use logcompress::{compress, CompressConfig};

let result = compress("payment timeout", &logs, &CompressConfig::default());
println!("Found {} relevant lines out of {}", 
    result.stats.output_lines, result.stats.input_lines);
```

## What's Next

- **Python bindings** via PyO3 (`pip install logcompress`)
- **MCP server** for LLM tool use
- Semantic search via embedding similarity (hybrid TF-IDF + embedding)

The code is MIT/Apache-2.0: [github.com/TimurRakhmatullin86/logcompress](https://github.com/TimurRakhmatullin86/logcompress)

I'd love to hear what query patterns you'd need for your production logs. What edge cases would break this approach?
