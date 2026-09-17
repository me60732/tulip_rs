# Standard Performance

Single asset, averaged across 4 option sets. Ratios show how many times slower the competitor is relative to Rust — higher means Rust wins by more.

| Language | Compared against | Highlight | Full results |
|---|---|---|---|
| Rust | C Tulip, TA-Lib, RustTa, Kand | Beats C Tulip on 64/69 indicators (93%), TA-Lib on 35/37 (95%), RustTa on 18/18, Kand on 35/35 | [Standard Performance: Rust →](standard/rust.md) |
| Python | `ta`, `pandas_ta` | Beats `ta` on 35/35 compared indicators (median ~27×); beats `pandas_ta` on 72/72 (median ~42×) | [Standard Performance: Python →](standard/python.md) |
| Node | `technicalindicators`, `indicatorts` | Median speedup 56.71× vs technicalindicators, 6.62× vs indicatorts | [Standard Performance: Node →](standard/node.md) |
| C | Tulip Indicators (C), TA-Lib | Beats C Tulip on 45/72 compared indicators (median 1.73×) | [Standard Performance: C →](standard/c.md) |
| Go | `cinar/indicator` | Beats cinar on 45/45 compared indicators (median 2,283×) | [Standard Performance: Go →](standard/go.md) |
