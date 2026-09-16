# Standard Performance: Go

Reference: **cinar/indicator/v2** (pure Go), compared against the **`tulip_rs_go`** cgo binding.

Only indicators where cinar is a genuine param-compatible equivalent are shown (cinar v2's stream API fixes some parameters or computes fewer output rows; those indicators have `tulip_rs_go` timings in the database but no comparable competitor).

| Indicator | Rust native (ns) | tulip_rs_go (ns) | cinar (ns) | Speedup vs cinar |
|-----------|----------------:|-----------------:|-----------:|-----------------:|
| `ad` | 4,799 | 10,666 | 9,750,278 | 914× |
| `adosc` | 5,657 | 8,752 | 15,194,062 | 1,736× |
| `ao` | 5,427 | 6,722 | 17,125,431 | 2,548× |
| `apo` | 4,872 | 8,481 | 3,473,150 | 410× |
| `aroon` | 18,771 | 35,676 | 13,911,132 | 390× |
| `atr` | 4,825 | 9,689 | 12,662,715 | 1,307× |
| `bbands` | 6,574 | 15,446 | 18,834,396 | 1,219× |
| `bop` | 2,440 | 8,234 | 9,246,560 | 1,123× |
| `cci` | 78,669 | 97,465 | 31,323,732 | 321× |
| `chaikinmf` | 7,201 | 23,958 | 21,610,263 | 902× |
| `chandelierexit` | 18,411 | 50,375 | 23,649,091 | 469× |
| `dema` | 6,336 | 16,139 | 11,097,340 | 688× |
| `donchianchannel` | 13,219 | 38,536 | 17,843,230 | 463× |
| `dpo` | 2,701 | 4,211 | 12,635,996 | 3,001× |
| `elderray` | 6,065 | 8,673 | 10,904,809 | 1,257× |
| `ema` | 4,784 | 11,431 | 2,522,108 | 221× |
| `emv` | 2,449 | 9,278 | 21,871,362 | 2,357× |
| `fisher` | 48,681 | 72,406 | 24,455,999 | 338× |
| `hma` | 9,349 | 13,140 | 14,309,162 | 1,089× |
| `kama` | 7,436 | 14,310 | 24,908,985 | 1,741× |
| `max` | 5,203 | 8,708 | 6,141,907 | 705× |
| `mfi` | 7,987 | 16,969 | 24,415,648 | 1,439× |
| `min` | 7,022 | 13,842 | 9,139,787 | 660× |
| `nvi` | 2,678 | 8,766 | 19,408,466 | 2,214× |
| `obv` | 3,513 | 8,739 | 4,637,874 | 531× |
| `ppo` | 5,062 | 17,546 | 20,684,703 | 1,179× |
| `qstick` | 2,571 | 10,608 | 12,781,495 | 1,205× |
| `roc` | 2,437 | 4,450 | 2,950,139 | 663× |
| `rsi` | 4,994 | 11,957 | 15,624,571 | 1,307× |
| `sma` | 2,558 | 6,001 | 7,457,551 | 1,243× |
| `smaenvelope` | 7,103 | 10,530 | 12,490,281 | 1,186× |
| `stddev` | 3,795 | 15,844 | 5,261,197 | 332× |
| `stoch` | 20,383 | 34,164 | 18,773,302 | 550× |
| `supertrend` | 12,074 | 27,563 | 30,354,666 | 1,101× |
| `tema` | 6,789 | 10,010 | 16,620,666 | 1,660× |
| `tr` | 1,393 | 2,729 | 6,632,641 | 2,430× |
| `trima` | 5,626 | 14,027 | 12,848,190 | 916× |
| `trix` | 6,410 | 14,964 | 13,469,565 | 900× |
| `typprice` | 1,086 | 2,913 | 7,210,727 | 2,476× |
| `ultosc` | 15,579 | 26,569 | 34,616,118 | 1,303× |

??? success "Notable results"

    `tulip_rs_go` beats `cinar` on **40 of 40 compared indicators** (cinar is wired where it is param-compatible; the remaining indicators have no pure-Go competitor run).
    Median speedup: **1,112×** — cinar v2's goroutine/channel stream API pays a per-bar scheduling cost the cgo path does not.

    **Largest wins vs cinar:**

    | Indicator | Speedup vs cinar |
    |-----------|-----------------:|
    | `dpo` | **3,001×** |
    | `ao` | **2,548×** |
    | `typprice` | **2,476×** |
    | `tr` | **2,430×** |
    | `emv` | **2,357×** |
    | `nvi` | **2,214×** |
    | `kama` | **1,741×** |
    | `adosc` | **1,736×** |
    | `tema` | **1,660×** |
    | `mfi` | **1,439×** |

    Smallest margins (where cinar's implementation is closest, or computes fewer output rows):

    | Indicator | Rust native (ns) | tulip_rs_go (ns) | cinar (ns) | Speedup vs cinar |
    |----------:|-----------------:|-----------------:|-----------:|-----------------:|
    | `ema` | 4,784 | 11,431 | 2,522,108 | **221×** |
    | `cci` | 78,669 | 97,465 | 31,323,732 | **321×** |
    | `stddev` | 3,795 | 15,844 | 5,261,197 | **332×** |
    | `fisher` | 48,681 | 72,406 | 24,455,999 | **338×** |
    | `aroon` | 18,771 | 35,676 | 13,911,132 | **390×** |
    | `apo` | 4,872 | 8,481 | 3,473,150 | **410×** |

    **cgo boundary overhead** — the gap between Rust native and `tulip_rs_go` columns reflects the fixed per-call cost of the cgo boundary (argument marshalling, pointer handoff), 1.8–3.8× for the fastest-running indicators:

    | Indicator | Rust native (ns) | tulip_rs_go (ns) | Overhead |
    |-----------|----------------:|-----------------:|---------:|
    | `roc` | 2,437 | 4,450 | ~1.8× |
    | `tr` | 1,393 | 2,729 | ~2.0× |
    | `sma` | 2,558 | 6,001 | ~2.3× |
    | `typprice` | 1,086 | 2,913 | ~2.7× |
    | `bop` | 2,440 | 8,234 | ~3.4× |
    | `emv` | 2,449 | 9,278 | ~3.8× |
