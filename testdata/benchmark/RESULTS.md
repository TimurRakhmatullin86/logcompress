# LogCompress Benchmark Results

Dataset: 100000 lines
Ground truth: 55 queries (50 per-incident + 5 aggregate)

## Per-query results

| # | Kind | Aggregate | Expected | Found | Recall | Precision | Time |
|---|------|-----------|----------|-------|--------|-----------|------|
| 0 | Timeout | no | 5 | 5/92 | 100% | 5% | 399685us |
| 1 | Oom | no | 3 | 3/84 | 100% | 4% | 336355us |
| 2 | Deadlock | no | 6 | 6/88 | 100% | 7% | 327377us |
| 3 | AuthFailure | no | 4 | 4/430 | 100% | 1% | 324225us |
| 4 | RateLimit | no | 6 | 6/430 | 100% | 1% | 330893us |
| 5 | Timeout | no | 6 | 6/92 | 100% | 7% | 384416us |
| 6 | Oom | no | 6 | 6/84 | 100% | 7% | 320437us |
| 7 | Deadlock | no | 5 | 5/88 | 100% | 6% | 322232us |
| 8 | AuthFailure | no | 6 | 6/430 | 100% | 1% | 318711us |
| 9 | RateLimit | no | 5 | 5/430 | 100% | 1% | 336122us |
| 10 | Timeout | no | 4 | 4/92 | 100% | 4% | 385136us |
| 11 | Oom | no | 3 | 3/84 | 100% | 4% | 341089us |
| 12 | Deadlock | no | 6 | 6/88 | 100% | 7% | 316529us |
| 13 | AuthFailure | no | 4 | 4/430 | 100% | 1% | 320965us |
| 14 | RateLimit | no | 4 | 4/430 | 100% | 1% | 330767us |
| 15 | Timeout | no | 6 | 6/92 | 100% | 7% | 388391us |
| 16 | Oom | no | 5 | 5/84 | 100% | 6% | 316223us |
| 17 | Deadlock | no | 3 | 3/88 | 100% | 3% | 316366us |
| 18 | AuthFailure | no | 3 | 3/430 | 100% | 1% | 321422us |
| 19 | RateLimit | no | 4 | 4/430 | 100% | 1% | 331501us |
| 20 | Timeout | no | 4 | 4/92 | 100% | 4% | 389601us |
| 21 | Oom | no | 6 | 6/84 | 100% | 7% | 331493us |
| 22 | Deadlock | no | 3 | 3/88 | 100% | 3% | 327126us |
| 23 | AuthFailure | no | 5 | 5/430 | 100% | 1% | 316316us |
| 24 | RateLimit | no | 5 | 5/430 | 100% | 1% | 338740us |
| 25 | Timeout | no | 6 | 6/92 | 100% | 7% | 385543us |
| 26 | Oom | no | 4 | 4/84 | 100% | 5% | 318001us |
| 27 | Deadlock | no | 6 | 6/88 | 100% | 7% | 317829us |
| 28 | AuthFailure | no | 4 | 4/430 | 100% | 1% | 315116us |
| 29 | RateLimit | no | 4 | 4/430 | 100% | 1% | 335110us |
| 30 | Timeout | no | 5 | 5/92 | 100% | 5% | 385098us |
| 31 | Oom | no | 5 | 5/84 | 100% | 6% | 327502us |
| 32 | Deadlock | no | 5 | 5/88 | 100% | 6% | 321361us |
| 33 | AuthFailure | no | 4 | 4/430 | 100% | 1% | 335848us |
| 34 | RateLimit | no | 4 | 4/430 | 100% | 1% | 340048us |
| 35 | Timeout | no | 6 | 6/92 | 100% | 7% | 428067us |
| 36 | Oom | no | 5 | 5/84 | 100% | 6% | 343056us |
| 37 | Deadlock | no | 6 | 6/88 | 100% | 7% | 328792us |
| 38 | AuthFailure | no | 3 | 3/430 | 100% | 1% | 320783us |
| 39 | RateLimit | no | 6 | 6/430 | 100% | 1% | 333120us |
| 40 | Timeout | no | 5 | 5/92 | 100% | 5% | 406374us |
| 41 | Oom | no | 3 | 3/84 | 100% | 4% | 323078us |
| 42 | Deadlock | no | 5 | 5/88 | 100% | 6% | 321215us |
| 43 | AuthFailure | no | 3 | 3/430 | 100% | 1% | 335894us |
| 44 | RateLimit | no | 4 | 4/430 | 100% | 1% | 334685us |
| 45 | Timeout | no | 5 | 5/92 | 100% | 5% | 390575us |
| 46 | Oom | no | 4 | 4/84 | 100% | 5% | 321461us |
| 47 | Deadlock | no | 3 | 3/88 | 100% | 3% | 325233us |
| 48 | AuthFailure | no | 5 | 5/430 | 100% | 1% | 344115us |
| 49 | RateLimit | no | 3 | 3/430 | 100% | 1% | 329971us |
| 50 | Timeout | yes | 52 | 52/52 | 100% | 100% | 413725us |
| 51 | Oom | yes | 44 | 44/230 | 100% | 19% | 321918us |
| 52 | Deadlock | yes | 48 | 48/230 | 100% | 21% | 320660us |
| 53 | AuthFailure | yes | 41 | 41/41 | 100% | 100% | 409343us |
| 54 | RateLimit | yes | 45 | 45/45 | 100% | 100% | 337492us |

## Summary

- **Average recall: 100.0%**
- **Average precision: 9.5%**
- Queries with recall >= 90%: 55/55

## Per-kind recall

- **AuthFailure**: 100.0% avg recall (11 queries)
- **Deadlock**: 100.0% avg recall (11 queries)
- **Oom**: 100.0% avg recall (11 queries)
- **RateLimit**: 100.0% avg recall (11 queries)
- **Timeout**: 100.0% avg recall (11 queries)