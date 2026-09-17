# Standard Performance: Go

Reference: **cinar/indicator/v2** (pure Go), compared against the **`tulip_rs_go`** cgo binding.

Only indicators where cinar is a genuine param-compatible equivalent are shown (cinar v2's stream API fixes some parameters or computes fewer output rows; those indicators have `tulip_rs_go` timings in the database but no comparable competitor).

| Indicator | Rust native (ns) | tulip_rs_go (ns) | cinar (ns) | Speedup vs cinar |
|-----------|----------------:|-----------------:|-----------:|-----------------:|
| `ad` | 4,799 | 5,478 | 9,095,226 | 1,660× |
| `adosc` | 5,657 | 6,626 | 23,164,241 | 3,496× |
| `ao` | 5,427 | 6,366 | 22,578,745 | 3,547× |
| `apo` | 4,872 | 5,452 | 5,162,216 | 947× |
| `aroon` | 18,771 | 17,936 | 13,797,216 | 769× |
| `atr` | 4,825 | 5,455 | 16,812,744 | 3,082× |
| `bbands` | 6,490 | 10,015 | 19,839,429 | 1,981× |
| `bop` | 2,440 | 3,137 | 7,441,486 | 2,372× |
| `cci` | 78,669 | 79,522 | 33,112,153 | 416× |
| `chaikinmf` | 7,201 | 10,203 | 22,629,489 | 2,218× |
| `chandelierexit` | 18,411 | 18,221 | 23,706,438 | 1,301× |
| `dema` | 6,336 | 6,405 | 14,623,870 | 2,283× |
| `donchianchannel` | 13,219 | 14,053 | 18,828,184 | 1,340× |
| `dpo` | 2,701 | 3,093 | 18,967,214 | 6,133× |
| `elderray` | 6,065 | 7,441 | 16,205,745 | 2,178× |
| `ema` | 4,784 | 5,434 | 3,270,234 | 602× |
| `emv` | 2,449 | 3,134 | 23,471,574 | 7,491× |
| `fisher` | 48,681 | 64,177 | 33,741,271 | 526× |
| `hma` | 9,349 | 9,323 | 18,461,147 | 1,980× |
| `kama` | 7,436 | 7,623 | 29,176,244 | 3,828× |
| `max` | 5,203 | 6,644 | 12,656,791 | 1,905× |
| `mfi` | 7,987 | 9,726 | 24,488,713 | 2,518× |
| `min` | 7,022 | 7,290 | 11,092,422 | 1,522× |
| `nvi` | 2,678 | 3,345 | 22,079,337 | 6,601× |
| `obv` | 3,513 | 4,181 | 5,771,541 | 1,380× |
| `ppo` | 5,062 | 6,154 | 22,736,404 | 3,695× |
| `qstick` | 2,571 | 3,237 | 15,645,711 | 4,833× |
| `roc` | 2,437 | 3,022 | 3,651,393 | 1,208× |
| `rsi` | 4,994 | 6,005 | 22,302,800 | 3,714× |
| `sma` | 2,558 | 3,071 | 10,903,048 | 3,550× |
| `smaenvelope` | 7,103 | 8,596 | 20,025,959 | 2,330× |
| `stddev` | 3,795 | 7,780 | 8,482,636 | 1,090× |
| `stoch` | 20,383 | 22,865 | 18,320,716 | 801× |
| `supertrend` | 12,074 | 13,478 | 35,116,577 | 2,606× |
| `tema` | 6,789 | 7,406 | 26,263,849 | 3,546× |
| `tr` | 1,393 | 2,096 | 10,745,694 | 5,126× |
| `trima` | 5,626 | 5,991 | 17,223,907 | 2,875× |
| `trix` | 6,410 | 7,345 | 21,044,608 | 2,865× |
| `typprice` | 1,086 | 1,721 | 10,168,992 | 5,910× |
| `ultosc` | 15,579 | 16,061 | 33,914,080 | 2,112× |
| `vwma` | 3,501 | 5,458 | 19,700,709 | 3,610× |
| `wcprice` | 1,075 | 1,697 | 8,078,139 | 4,762× |
| `wilders` | 4,777 | 5,413 | 1,919,589 | 355× |
| `willr` | 17,282 | 15,429 | 24,589,898 | 1,594× |
| `wma` | 4,943 | 5,408 | 4,593,886 | 849× |

??? success "Notable results"

    `tulip_rs_go` beats `cinar` on **45 of 45 compared indicators** (cinar is wired where it is param-compatible; the remaining indicators have no pure-Go competitor run).
    Median speedup: **2,283×** — cinar v2's goroutine/channel stream API pays a per-bar scheduling cost the cgo path does not.

    **Largest wins vs cinar:**

    | Indicator | Speedup vs cinar |
    |-----------|-----------------:|
    | `emv` | **7,491×** |
    | `nvi` | **6,601×** |
    | `dpo` | **6,133×** |
    | `typprice` | **5,910×** |
    | `tr` | **5,126×** |
    | `qstick` | **4,833×** |
    | `wcprice` | **4,762×** |
    | `kama` | **3,828×** |
    | `rsi` | **3,714×** |
    | `ppo` | **3,695×** |

    Smallest margins (where cinar's implementation is closest, or computes fewer output rows):

    | Indicator | Rust native (ns) | tulip_rs_go (ns) | cinar (ns) | Speedup vs cinar |
    |----------:|-----------------:|-----------------:|-----------:|-----------------:|
    | `wilders` | 4,777 | 5,413 | 1,919,589 | **355×** |
    | `cci` | 78,669 | 79,522 | 33,112,153 | **416×** |
    | `fisher` | 48,681 | 64,177 | 33,741,271 | **526×** |
    | `ema` | 4,784 | 5,434 | 3,270,234 | **602×** |
    | `aroon` | 18,771 | 17,936 | 13,797,216 | **769×** |
    | `stoch` | 20,383 | 22,865 | 18,320,716 | **801×** |

    **cgo boundary overhead** — the gap between Rust native and `tulip_rs_go` columns reflects the fixed per-call cost of the cgo boundary (argument marshalling, pointer handoff), 1.2–1.6× for the fastest-running indicators:

    | Indicator | Rust native (ns) | tulip_rs_go (ns) | Overhead |
    |-----------|----------------:|-----------------:|---------:|
    | `roc` | 2,437 | 3,022 | ~1.2× |
    | `emv` | 2,449 | 3,134 | ~1.3× |
    | `bop` | 2,440 | 3,137 | ~1.3× |
    | `tr` | 1,393 | 2,096 | ~1.5× |
    | `wcprice` | 1,075 | 1,697 | ~1.6× |
    | `typprice` | 1,086 | 1,721 | ~1.6× |
