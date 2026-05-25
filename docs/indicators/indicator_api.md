# Indicator API Reference

Every TulipRS indicator exposes a consistent set of functions beyond the core `indicator()` call. This page covers the metadata and utility functions that let you introspect an indicator's inputs, outputs, and data requirements at runtime.

---

## `info()` — Indicator Metadata

Every indicator module exports an `info()` function that returns a fully-populated `Info` struct describing the indicator. This is the canonical place to discover what an indicator needs and what it produces — without reading source code or docs.

```rust
pub struct Info<'a> {
    pub name:             &'a str,           // short identifier, e.g. "adosc"
    pub full_name:        &'a str,           // e.g. "Accumulation/Distribution Oscillator"
    pub indicator_type:   IndicatorType,     // Trend | Momentum | Volume | Volatility | Price | Cycle
    pub display_type:     DisplayType,       // Overlay | Indicator | Math
    pub inputs:           &'a [&'a str],     // names of required input series
    pub options:          &'a [&'a str],     // names of option parameters, in order
    pub outputs:          &'a [&'a str],     // names of primary output series, in order
    pub optional_outputs: &'a [&'a str],     // names of optional output series, in order
}
```

### Usage

=== "Rust"

    ```rust
    use tulip_rs::indicators::adosc;

    let meta = adosc::info();

    println!("Name:             {}", meta.name);         // adosc
    println!("Full name:        {}", meta.full_name);    // Accumulation/Distribution Oscillator
    println!("Type:             {}", meta.indicator_type); // Trend
    println!("Display:          {}", meta.display_type);   // Indicator
    println!("Inputs:           {:?}", meta.inputs);       // ["high", "low", "close", "volume"]
    println!("Options:          {:?}", meta.options);      // ["short_period", "long_period"]
    println!("Outputs:          {:?}", meta.outputs);      // ["adosc"]
    println!("Optional outputs: {:?}", meta.optional_outputs); // ["short_ema", "long_ema", "ad"]
    ```

=== "Python"

    The Python bindings expose indicator metadata through module-level attributes on each indicator:

    ```python
    import tulip_rs

    meta = tulip_rs.indicators.adosc.info()

    print(meta.name)              # adosc
    print(meta.full_name)         # Accumulation/Distribution Oscillator
    print(meta.inputs)            # ['high', 'low', 'close', 'volume']
    print(meta.options)           # ['short_period', 'long_period']
    print(meta.outputs)           # ['adosc']
    print(meta.optional_outputs)  # ['short_ema', 'long_ema', 'ad']
    ```

### What each field means

| Field | Description |
|---|---|
| `name` | The short identifier used to locate the module: `tulip_rs::indicators::<name>` |
| `full_name` | Human-readable name suitable for display in UIs or reports |
| `indicator_type` | Broad category — useful for filtering or grouping indicators |
| `display_type` | How the indicator is typically charted: **Overlay** (on the price chart), **Indicator** (sub-chart), **Math** (raw transform) |
| `inputs` | Input series names, in the order they must be passed to `indicator()` |
| `options` | Option parameter names, in the order they must be passed to `indicator()` |
| `outputs` | Primary output series names. `outputs[i]` corresponds to `indicator_result[i]` |
| `optional_outputs` | Optional intermediate output series. See [Optional Outputs](#optional-outputs) below |

### Common use cases

- **Building dynamic UIs** — populate dropdowns, form labels, and axis titles without hardcoding strings.
- **Validation** — check `inputs.len()` and `options.len()` before constructing a call.
- **Introspection in tests** — confirm that the number of returned output vecs matches `outputs.len() + optional_outputs.len()`.
- **Auto-generating documentation** — iterate all indicator modules and call `info()` to produce a live reference table.

---

## Optional Outputs

Many indicators compute intermediate series as part of their normal calculation. Rather than discarding these values, TulipRS can return them alongside the primary outputs — at **no extra computation cost**, since they were calculated anyway.

This is a meaningful advantage over C Tulip and TA-Lib, which require a **separate function call** for each intermediate result, each re-reading the input data from scratch. TulipRS computes the primary output and every optional output in a **single pass** through the data. Depending on the indicator, requesting all optional outputs via TulipRS is **1.3× – 8.7× faster** than equivalent multi-call C code — see the [Optional Outputs benchmark](../benchmarks.md#4-optional-outputs-single-pass-computation-advantage) for full numbers per indicator.

Optional outputs are **off by default**. Requesting them never changes the primary output values; it only captures values that would otherwise be thrown away.

### Which optional outputs does an indicator have?

Call `info()` and inspect the `optional_outputs` field:

```rust
use tulip_rs::indicators::adx;

let meta = adx::info();
println!("{:?}", meta.optional_outputs); // ["dx", "atr", "tr"]
```

Common examples:

| Indicator | Primary output | Optional outputs |
|---|---|---|
| `adosc` | `adosc` | `short_ema`, `long_ema`, `ad` |
| `adx` | `adx` | `dx`, `atr`, `tr` |
| `adxr` | `adxr` | `adx`, `dx`, `atr`, `tr` |
| `ao` | `ao` | `short_sma`, `long_sma`, `medprice` |
| `macd` | `macd` | *(primary outputs include signal and histogram)* |

### Requesting optional outputs

The third argument to `indicator()` is `optional_outputs: Option<&[bool]>`. Each element corresponds to one optional output, **in the same order as `info().optional_outputs`**:

- `None` — no optional outputs are returned (default; best performance when you don't need them).
- `Some(&[bool; N])` — a mask where `true` means "return this series" and `false` means "skip it".

=== "Rust"

    ```rust
    use tulip_rs::indicators::adosc;

    let high  = vec![/* ... */];
    let low   = vec![/* ... */];
    let close = vec![/* ... */];
    let vol   = vec![/* ... */];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice(), vol.as_slice()];

    // info().optional_outputs == ["short_ema", "long_ema", "ad"]
    //                              ^^^^^^^^^^^  ^^^^^^^^^^  ^^^^
    //                              index 0      index 1     index 2

    // Request only the AD line (index 2); skip short_ema and long_ema
    let mask = [false, false, true];
    let (outputs, state) = adosc::indicator(&inputs, &[6.0, 20.0], Some(&mask)).unwrap();

    let adosc_line = &outputs[0]; // primary output — always present
    // outputs[1] and outputs[2] are empty (not requested)
    let ad_line    = &outputs[3]; // optional output at index 2 — present because mask[2] == true
    ```

    !!! note "Output vector layout"
        `outputs` always has length `outputs.len() + optional_outputs.len()` (from `info()`).
        Primary outputs come first (always populated), then optional outputs in declaration order
        (populated or empty depending on the mask).

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([...], dtype=np.float64)
    low   = np.array([...], dtype=np.float64)
    close = np.array([...], dtype=np.float64)
    vol   = np.array([...], dtype=np.float64)

    # Request the AD line only (index 2 of optional_outputs)
    outputs, state = tulip_rs.indicators.adosc.indicator(
        [high, low, close, vol],
        [6.0, 20.0],
        optional_outputs=[False, False, True],
    )

    adosc_line = outputs[0]   # primary output
    ad_line    = outputs[3]   # optional output at index 2
    ```

### All optional outputs at once

Pass a mask of all `true` to capture every intermediate series:

=== "Rust"

    ```rust
    // adosc has 3 optional outputs
    let mask = [true, true, true];
    let (outputs, state) = adosc::indicator(&inputs, &[6.0, 20.0], Some(&mask)).unwrap();

    let adosc_line     = &outputs[0]; // adosc     (primary)
    let short_ema_line = &outputs[1]; // short_ema (optional 0)
    let long_ema_line  = &outputs[2]; // long_ema  (optional 1)
    let ad_line        = &outputs[3]; // ad        (optional 2)
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.adosc.indicator(
        [high, low, close, vol],
        [6.0, 20.0],
        optional_outputs=[True, True, True],
    )

    adosc_line     = outputs[0]
    short_ema_line = outputs[1]
    long_ema_line  = outputs[2]
    ad_line        = outputs[3]
    ```

### Optional outputs in streaming mode

Optional output masks work the same way with `batch_indicator()`. Pass the same mask you used in the initial `indicator()` call:

=== "Rust"

    ```rust
    // Initial batch — request AD line
    let mask = [false, false, true];
    let (outputs, mut state) = adosc::indicator(&inputs, &[6.0, 20.0], Some(&mask)).unwrap();

    // Continue streaming — same mask
    let new_inputs = [new_high.as_slice(), new_low.as_slice(), new_close.as_slice(), new_vol.as_slice()];
    let continued = state.batch_indicator(&new_inputs, Some(&mask)).unwrap();

    let new_adosc = &continued[0];
    let new_ad    = &continued[3];
    ```

### Performance note

Optional outputs are computed as part of the indicator's normal calculation loop — requesting them adds **zero algorithmic overhead**. The only cost is the memory allocation for the extra output vectors and the store instructions to write them. Passing `None` (or an all-`false` mask) allows the compiler to elide those stores entirely, which is why `None` is the default.

The performance difference between requesting all optional outputs and requesting none is documented in the [Benchmarks](../benchmarks.md#optional-outputs-single-pass) page — typically 5–15% depending on the indicator.

---

## `min_data()` — Minimum Input Length

```rust
pub fn min_data(options: &[f64]) -> usize
```

Returns the **absolute minimum number of input bars** needed to produce at least one output bar. If you call `indicator()` with fewer bars than this, it returns `Err(IndicatorError::NotEnoughData)`.

The value depends on the indicator's options because period-based indicators require at least `period` bars to produce their first output.

=== "Rust"

    ```rust
    use tulip_rs::indicators::adx;

    // ADX with period = 14 needs at least 14*2 = 28 bars
    let minimum = adx::min_data(&[14.0]);
    println!("Min data: {minimum}"); // 28

    // Check before calling
    if close.len() < minimum {
        eprintln!("Not enough data: have {}, need {}", close.len(), minimum);
    } else {
        let (outputs, state) = adx::indicator(&[high.as_slice(), low.as_slice(), close.as_slice()], &[14.0], None).unwrap();
    }
    ```

=== "Python"

    ```python
    import tulip_rs

    minimum = tulip_rs.indicators.adx.min_data([14.0])
    print(f"Min data: {minimum}")  # 28

    if len(close) < minimum:
        print(f"Not enough data: have {len(close)}, need {minimum}")
    else:
        outputs, state = tulip_rs.indicators.adx.indicator([high, low, close], [14.0])
    ```

---

## `min_data_accuracy()` — Minimum Input for Decimal Accuracy

```rust
pub fn min_data_accuracy(options: &[f64], decimal_places: usize) -> usize
```

Returns the number of input bars needed to produce output values accurate to **`decimal_places`** decimal places.

This is only relevant for indicators that use **exponential smoothing** (EMA, KAMA, Wilder's smoothing, etc.). The first value an EMA produces is seeded from its initial bar, and that seed's influence decays exponentially. With a short lookback you can get a correct EMA value, but with fewer bars than `min_data_accuracy` the result may differ from a "true" EMA (one that started from the infinite past) in digits beyond `decimal_places`.

For indicators without exponential smoothing (SMA, Max, Min, etc.), `min_data_accuracy` returns the same value as `min_data`.

### Scanning without full history

The most practical use of `min_data_accuracy` is **event scanning across a large universe of assets**. To detect something like a MACD crossover you don't need to feed in years of daily bars — you only need enough bars for the EMA values to have converged to the required precision. `min_data_accuracy` tells you exactly how many that is, so you can:

- Fetch only the most recent `min_data_accuracy(options, 6)` bars per asset from your database instead of the full history.
- Run the indicator over that window and check for your signal.
- Scale across thousands of assets with a fraction of the data transfer and compute cost.

=== "Rust"

    ```rust
    use tulip_rs::indicators::macd;

    // MACD(12, 26, 9) — how many bars do we need for 6dp accuracy?
    let options = &[12.0, 26.0, 9.0];
    let window = macd::min_data_accuracy(options, 6);

    // Fetch only the last `window` bars from the database for each asset
    // instead of its entire history.
    for asset in &universe {
        let close = db.fetch_last_n_bars(asset, window);
        let (outputs, _state) = macd::indicator(&[close.as_slice()], options, None).unwrap();

        // Check the last value of each output for a crossover
        let macd_line = &outputs[0];
        let signal    = &outputs[1];
        if macd_line.last() > signal.last() {
            println!("{asset}: MACD crossover detected");
        }
    }
    ```

=== "Python"

    ```python
    import tulip_rs

    options = [12.0, 26.0, 9.0]
    window = tulip_rs.indicators.macd.min_data_accuracy(options, 6)

    for asset in universe:
        close = db.fetch_last_n_bars(asset, window)
        outputs, _ = tulip_rs.indicators.macd.indicator([close], options)

        macd_line = outputs[0]
        signal    = outputs[1]
        if macd_line[-1] > signal[-1]:
            print(f"{asset}: MACD crossover detected")
    ```

### When to use `min_data_accuracy`

| Scenario | Use |
|---|---|
| Checking whether a call will succeed at all | `min_data` |
| Production systems where precision matters (backtesting P&L, signal generation) | `min_data_accuracy` with your required decimal places |
| Exploratory / visual charting where a few ticks of warmup drift are acceptable | `min_data` is sufficient |
| Comparing indicator values to a reference implementation | `min_data_accuracy` with the required precision |

!!! tip
    For most production backtesting scenarios, `min_data_accuracy(options, 6)` is a safe default. It ensures that floating-point drift from the EMA seed is below one millionth of a unit — negligible for any realistic price series.

---

## Function Summary

| Function | Signature | Returns |
|---|---|---|
| `info()` | `() -> Info<'static>` | Full metadata: names, types, input/option/output lists |
| `min_data()` | `(options: &[f64]) -> usize` | Minimum bars to get any output |
| `min_data_accuracy()` | `(options: &[f64], decimals: usize) -> usize` | Minimum bars for `decimals`-place accuracy |
| `indicator()` | `(inputs, options, optional_outputs) -> Result<(Vec<Vec<f64>>, State), Error>` | Primary computation |
| `state.batch_indicator()` | `(inputs, optional_outputs) -> Result<Vec<Vec<f64>>, Error>` | Streaming continuation |
