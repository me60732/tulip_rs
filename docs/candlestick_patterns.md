# Candlestick Patterns

TulipRS includes a dedicated candlestick recognition engine covering **77+ classical patterns** — from single-bar formations such as Hammer and Shooting Star through to four-bar structures such as Concealing Baby Swallow. Every pattern carries a **Japanese name** and a **forecast type** (bullish/bearish reversal or continuation), and all patterns are detected in a **single pass** over the input bars. The candlestick engine uses internally computed body and wick size averages, plus a trend signal, to ensure that context-dependent patterns (e.g. Hanging Man vs Hammer) are classified correctly.

!!! note "Output shape differs from regular indicators"
    The candlestick API takes the same NumPy array inputs as every other indicator in TulipRS, but its **output is different** — instead of a numeric series it returns a list of pattern-match lists (one entry per output bar, each entry being a list of matched pattern dicts).

---

## Options

The candlestick engine accepts three options in the following order:

| Position | Name | Description |
|---|---|---|
| `options[0]` | `candle_period` | Lookback window used to compute rolling averages for body size and upper/lower wick size. A larger value smooths out the reference size over more bars — use a value greater than 5 in practice. |
| `options[1]` | `trend_period` | Lookback window for the raw trend calculation. Controls how many bars are included when determining whether the market is in an uptrend or downtrend at the point of pattern detection. |
| `options[2]` | `trend_signal_period` | Smoothing period applied to the raw trend value to produce a trend signal. Higher values reduce noise in trend classification. |

---

## Basic Usage

=== "Rust"

    ```rust
    use tulip_rs::indicators::candlestick::indicator;

    let open  = vec![81.85_f64, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25];
    let high  = vec![82.15_f64, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00];
    let low   = vec![81.29_f64, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11];
    let close = vec![81.59_f64, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36];

    let inputs  = [open.as_slice(), high.as_slice(), low.as_slice(), close.as_slice()];
    let options = [5.0_f64, 1.0, 1.0]; // candle_period, trend_period, trend_signal_period

    // result is Vec<Option<Vec<Pattern>>> — one entry per output bar
    // Pattern is an enum; call .get_info() on each variant to retrieve its metadata
    let (result, mut state) = indicator(&inputs, &options, None).unwrap();

    for (i, bar) in result.iter().enumerate() {
        if let Some(patterns) = bar.as_ref() {
            for pattern in patterns {
                let info = pattern.get_info();
                println!("Bar {i}: {} ({}), bars: {}",
                    info.full_name, info.japanese_name, info.bars);
            }
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    cdl = tulip_rs.indicators.candlestick

    open_  = np.array([81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25,
                       87.30, 86.40, 84.30, 85.60], dtype=np.float64)
    high_  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                       87.30, 86.40, 85.50, 85.65], dtype=np.float64)
    low_   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       86.30, 85.30, 84.00, 83.85], dtype=np.float64)
    close_ = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       86.30, 85.30, 84.00, 83.90], dtype=np.float64)

    options = [5.0, 1.0, 1.0]  # candle_period, trend_period, trend_signal_period

    # Run all patterns (matching current trend direction)
    result, state = cdl.candlestick(open_, high_, low_, close_, options=options)

    # result is a list — one entry per output bar — each entry is a list of pattern dicts
    for i, bar_patterns in enumerate(result):
        if bar_patterns:
            for p in bar_patterns:
                print(f"Bar {i}: {p['full_name']} ({p['japanese_name']}) — {p['forecast']}")
    ```

Each matched pattern is returned as a dict (Python) or a struct (Rust) with the following fields:

| Field | Type | Description |
|---|---|---|
| `name` | `str` | Short machine-readable pattern name |
| `full_name` | `str` | Full English pattern name |
| `japanese_name` | `str` | Traditional Japanese name |
| `bars` | `int` | Number of bars the pattern spans (1–4) |
| `forecast` | `str` / enum | One of `BullishReversal`, `BearishReversal`, `BullishContinuation`, `BearishContinuation` |

---

## Filtering by Forecast Type

Pass a `forecast_type` argument to return only patterns with a specific forecast. This is useful when you only care about, for example, bullish reversal setups.

=== "Rust"

    ```rust
    use tulip_rs::indicators::candlestick::{indicator, ForecastType};

    let inputs  = [open.as_slice(), high.as_slice(), low.as_slice(), close.as_slice()];
    let options = [5.0_f64, 1.0, 1.0];

    // Only bullish reversal patterns
    let (result, _) = indicator(&inputs, &options, Some(ForecastType::BullishReversal)).unwrap();

    // Inspect the last bar for matches
    if let Some(patterns) = result.last().and_then(|bar| bar.as_ref()) {
        for pattern in patterns {
            let info = pattern.get_info();
            println!("  - {} ({}), bars: {}",
                info.full_name, info.japanese_name, info.bars);
        }
    }

    // Other available variants:
    // ForecastType::BearishReversal
    // ForecastType::BullishContinuation
    // ForecastType::BearishContinuation
    ```

=== "Python"

    ```python
    import tulip_rs

    cdl          = tulip_rs.indicators.candlestick
    ForecastType = cdl.ForecastType

    options = [5.0, 1.0, 1.0]

    # Only bullish reversal patterns
    result, _ = cdl.candlestick(open_, high_, low_, close_, options=options,
                                forecast_type=ForecastType.BullishReversal)

    # Other available values:
    # ForecastType.BearishReversal
    # ForecastType.BullishContinuation
    # ForecastType.BearishContinuation

    for i, bar_patterns in enumerate(result):
        if bar_patterns:
            for p in bar_patterns:
                print(f"Bar {i}: {p['full_name']} — {p['forecast']}")
    ```

When `forecast_type` is omitted (or `None`), all matched patterns are returned regardless of their forecast direction.

---

## State Continuation

Like every other indicator in TulipRS, the candlestick engine returns a `state` object that you can use to continue detection on new bars without reprocessing history. Pass only the **new bars** to `batch_indicator`.

=== "Rust"

    ```rust
    use tulip_rs::indicators::candlestick::indicator;

    let open  = vec![81.85_f64, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25];
    let high  = vec![82.15_f64, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00];
    let low   = vec![81.29_f64, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11];
    let close = vec![81.59_f64, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36];

    let inputs  = [open.as_slice(), high.as_slice(), low.as_slice(), close.as_slice()];
    let options = [5.0_f64, 1.0, 1.0];

    // Step 1: run on historical data and capture state
    let (_, mut state) = indicator(&inputs, &options, None).unwrap();

    // Step 2: feed only the new bars
    let new_open  = [84.00_f64];
    let new_high  = [84.50_f64];
    let new_low   = [83.20_f64];
    let new_close = [83.50_f64];
    let new_inputs = [new_open.as_slice(), new_high.as_slice(),
                      new_low.as_slice(), new_close.as_slice()];

    let result = state.batch_indicator(&new_inputs, None).unwrap();

    if let Some(patterns) = result.last().and_then(|bar| bar.as_ref()) {
        for pattern in patterns {
            let info = pattern.get_info();
            println!("  - {} ({}), bars: {}",
                info.full_name, info.japanese_name, info.bars);
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    cdl     = tulip_rs.indicators.candlestick
    options = [5.0, 1.0, 1.0]

    # Step 1: run on historical data and capture state
    _result, state = cdl.candlestick(open_, high_, low_, close_, options=options)

    # Step 2: feed new bars as NumPy arrays
    new_open  = np.array([84.00], dtype=np.float64)
    new_high  = np.array([84.50], dtype=np.float64)
    new_low   = np.array([83.20], dtype=np.float64)
    new_close = np.array([83.50], dtype=np.float64)

    new_result = state.batch_indicator([new_open, new_high, new_low, new_close])

    entry = new_result[0]  # patterns detected on this new bar
    if entry:
        for p in entry:
            print(f"{p['full_name']} — {p['forecast']}")
    ```

---

## Pattern Reference

### One-Bar Patterns

| Pattern | Japanese Name | Forecast |
|---|---|---|
| Hammer | Kanazuchi | BullishReversal |
| Hanging Man | Kubitsuri | BearishReversal |
| Bullish Belt Hold | Yorikiri | BullishReversal |
| Bearish Belt Hold | Yorikiri | BearishReversal |
| Bullish Strong Line | Yorikiri Sen | BullishContinuation |
| Bearish Strong Line | Yorikiri Sen | BearishContinuation |
| Northern Doji | Kita no Doji | BearishReversal |
| Southern Doji | Minami no Doji | BullishReversal |
| Gapping Up Doji | Ue-hanare Doji | BearishReversal |
| Gapping Down Doji | Shita-hanare Doji | BullishReversal |
| Shooting Star | Nagare Boshi | BearishReversal |
| Takuri Line | Takuri | BullishReversal |

### Two-Bar Patterns

| Pattern | Forecast |
|---|---|
| Bullish Engulfing | BullishReversal |
| Bearish Engulfing | BearishReversal |
| Dark Cloud Cover | BearishReversal |
| Piercing | BullishReversal |
| Bullish Harami | BullishReversal |
| Bearish Harami | BearishReversal |
| Bullish Harami Cross | BullishReversal |
| Bearish Harami Cross | BearishReversal |
| Inverted Hammer | BullishReversal |
| Bullish Doji Star | BullishReversal |
| Bearish Doji Star | BearishReversal |
| Kicking Bullish | BullishReversal |
| Kicking Bearish | BearishReversal |
| Meeting Lines Bullish | BullishReversal |
| Meeting Lines Bearish | BearishReversal |
| On Neck | BearishContinuation |
| In Neck | BearishContinuation |
| Thrusting | BearishContinuation |

### Three-Bar Patterns

| Pattern | Forecast |
|---|---|
| Three White Soldiers | BullishReversal |
| Three Black Crows | BearishReversal |
| Morning Star | BullishReversal |
| Morning Doji Star | BullishReversal |
| Evening Star | BearishReversal |
| Evening Doji Star | BearishReversal |
| Three Inside Up | BullishReversal |
| Three Inside Down | BearishReversal |
| Three Outside Up | BullishReversal |
| Three Outside Down | BearishReversal |
| Bullish Tristar | BullishReversal |
| Bearish Tristar | BearishReversal |
| Advance Block | BearishReversal |
| Deliberation | BearishReversal |
| Unique Three River Bottom | BullishReversal |
| Three Stars in the South | BullishReversal |
| Upside Gap Three Methods | BullishContinuation |
| Downside Gap Three Methods | BearishContinuation |
| Upside Tasuki Gap | BullishContinuation |
| Downside Tasuki Gap | BearishContinuation |
| Upside Gap Two Crows | BearishReversal |
| Two Crows | BearishReversal |
| Identical Three Crows | BearishReversal |
| Bull Side-by-Side White Lines | BullishContinuation |
| Bear Side-by-Side White Lines | BearishContinuation |
| Collapsing Doji Star | BearishReversal |
| Bull Abandoned Baby | BullishReversal |
| Bear Abandoned Baby | BearishReversal |

### Four-Bar Patterns

| Pattern | Forecast |
|---|---|
| Concealing Baby Swallow | BullishReversal |
| Bullish Three Line Strike | BullishContinuation |
| Bearish Three Line Strike | BearishContinuation |
