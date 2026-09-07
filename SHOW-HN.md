# Show HN: LogCompress – Query-aware log compression for LLM agents (100k → 200 lines)

I built a tool that takes a natural language query and a log file, and returns only the relevant lines.

**The problem:** LLM context windows are finite. When an oncall engineer asks "why did payment 42 timeout?", dumping 100k raw log lines into an LLM prompt wastes tokens on health checks and heartbeats. grep gives exact string matches but misses related context. jq filters by field but doesn't rank by relevance.

**How it works:**

1. Byte-level FNV hash tokenization — builds an inverted index with zero String allocation
2. TF-IDF cosine similarity scoring between query and each line
3. Trace expansion — follows `trace_id` / `request_id` from high-scoring hits to pull the full incident timeline
4. Score gap filter — drops noise lines that score < 25% of top
5. Context windows — ±N lines around each hit, merged

**Results on a 100k-line benchmark (50 hidden incidents):**

- 100% recall (55/55 queries, 5 incident types)
- < 250ms for 100k lines (parallel tokenization via rayon)
- 100k input → typically 100-200 output lines

The trace expansion is the key differentiator. A timeout error line mentions "timeout" — but the preceding retry attempts and the slow response warnings use different vocabulary. TF-IDF alone misses them. By following the shared trace_id, we get the full story.

Written in Rust. CLI tool + library crate. Dual-licensed MIT/Apache-2.0.

https://github.com/TimurRakhmatullin86/logcompress

Looking for feedback on:
- What query patterns would you need for your logs?
- Would a Python wrapper (PyO3) or MCP server be useful?
- Any scoring heuristics I'm missing?
