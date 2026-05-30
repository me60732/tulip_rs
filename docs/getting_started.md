# Getting Started

## Installation

=== "Rust"

    Add TulipRS to your `Cargo.toml`:

    ```toml
    [dependencies]
    tulip_rs = "0.1.10"
    ```

    To get the very latest unreleased changes, use the Git source directly:

    ```toml
    [dependencies]
    tulip_rs = { git = "https://github.com/me60732/tulip_rs" }
    ```

    TulipRS uses the `portable_simd` nightly language feature internally and requires a **nightly** Rust toolchain. The correct nightly version is pinned automatically via the `rust-toolchain.toml` file at the root of the repository — no manual toolchain management is needed.

=== "Python"

    **From PyPI (recommended):**

    ```bash
    pip install tulip-rs
    ```

    **From source (for development or to enable native CPU optimisations):**

    ```bash
    git clone https://github.com/me60732/tulip-rs-python
    cd tulip_rs_python
    RUSTFLAGS="-C target-cpu=native" maturin develop --release
    ```

    Requirements: Python 3.8+, Rust 1.70+

=== "Node.js"

    **From npm (recommended):**

    ```bash
    npm install tulip-rs-node
    ```

    Prebuilt binaries are provided for Linux x64, macOS x64, and macOS arm64. No Rust toolchain required.

    **From source (for development or native CPU optimisations):**

    ```bash
    git clone https://github.com/me60732/tulip-rs-node
    cd tulip-rs-node
    npm install
    npm run build
    ```

    **Requirements:** Node.js 18+, Rust nightly (only needed when building from source)

---

## Feature Flags

| Feature | Default | Description |
|---|---|---|
| `simd_assets` | ✅ on | Compiles `indicator_by_assets::<N>` for every indicator |
| `simd_options` | ✅ on | Compiles `indicator_by_options::<N>` for every indicator |

!!! note "Nightly toolchain"
    The nightly toolchain version is pinned automatically by `rust-toolchain.toml` in the repository root. You do not need to run `rustup override set nightly` manually — Cargo will select the correct toolchain when you build inside the workspace.

!!! warning "`portable_simd` is always required"
    `portable_simd` is a nightly language feature used throughout the core indicator engine — including scalar indicators — and cannot be disabled. A nightly toolchain is therefore always required regardless of which Cargo features are enabled.

To disable the SIMD multi-asset and multi-option variants (e.g. to reduce compile time):

```toml
[dependencies]
tulip_rs = { git = "https://github.com/me60732/tulip_rs", default-features = false }
```

---

## Calling Convention

Every indicator in TulipRS follows the same universal signature. Once you understand it for one indicator you understand it for all of them.

=== "Rust"

    ```rust
    indicator(
        inputs:          &[&[f64]],          // one slice per input series
        options:         &[f64],             // indicator parameters
        optional_outputs: Option<&[bool]>,   // which optional outputs to compute (or None)
    ) -> Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError>
    ```

    - `inputs` — a slice of data slices. Single-input indicators take `&[close.as_slice()]`; multi-input indicators take e.g. `&[high.as_slice(), low.as_slice(), close.as_slice()]`.
    - `options` — indicator parameters as `f64`, in the order documented for each indicator.
    - `optional_outputs` — pass `None` unless you specifically want to suppress optional output series.
    - The return value is a tuple of `(outputs, state)`:
        - `outputs` is a `Vec<Vec<f64>>` — one inner `Vec` per output series, already trimmed to the valid output length.
        - `state` is an `IndicatorState` that can be used to continue computation on new bars without reprocessing history.

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.<name>.indicator(inputs, options)
    ```

    - `inputs` — a list of NumPy `float64` arrays, one per input series.
    - `options` — a list of `float` values, in the order documented for each indicator.
    - `outputs` — a list of NumPy arrays, one per output series, already trimmed to valid length.
    - `state` — an `IndicatorState` object that exposes `batch_indicator()` and JSON serialisation.

    !!! note "Candlestick patterns use plain Python lists, not NumPy arrays."
        See the [Candlestick Patterns](candlestick_patterns.md) page for details.

=== "Node.js"

    ```javascript
    const [outputs, state] = ti.<name>.indicator(inputs, options);
    ```

    - `inputs` — an array of `number[]`, one per input series.
    - `options` — an array of `number` values, in the order documented for each indicator.
    - `outputs` — an array of `number[]`, one per output series, already trimmed to valid length.
    - `state` — a state object that exposes `batchIndicator()` and JSON/Buffer serialisation.

    !!! note "Candlestick patterns use separate arrays per OHLC series."
        See the [Candlestick Patterns](candlestick_patterns.md) page for details.

---

## Examples

### SMA — 1 input, 1 option, 1 output

=== "Rust"

    ```rust
    use tulip_rs::indicators::sma::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, state) = indicator(&[close.as_slice()], &[5.0], None).unwrap();

    println!("{:?}", outputs[0]); // SMA(5) — length is close.len() - period + 1
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.sma.indicator([close], [5.0])

    print(outputs[0])  # SMA(5) values
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36];

    const [outputs, state] = ti.sma.indicator([close], [5]);

    console.log(outputs[0]); // SMA(5) values
    ```

---

### MACD — 1 input, 3 options, 3 outputs

=== "Rust"

    ```rust
    use tulip_rs::indicators::macd::indicator;

    // options: [fast_period, slow_period, signal_period]
    let (outputs, state) = indicator(&[close.as_slice()], &[12.0, 26.0, 9.0], None).unwrap();

    let macd_line  = &outputs[0]; // MACD line
    let signal     = &outputs[1]; // Signal line
    let histogram  = &outputs[2]; // Histogram
    ```

=== "Python"

    ```python
    # options: [fast_period, slow_period, signal_period]
    outputs, state = tulip_rs.indicators.macd.indicator([close], [12.0, 26.0, 9.0])

    macd_line = outputs[0]  # MACD line
    signal    = outputs[1]  # Signal line
    histogram = outputs[2]  # Histogram
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    // options: [fast_period, slow_period, signal_period]
    const [outputs, state] = ti.macd.indicator([close], [12, 26, 9]);

    const macdLine  = outputs[0]; // MACD line
    const signal    = outputs[1]; // Signal line
    const histogram = outputs[2]; // Histogram
    ```

---

### ADX — 3 inputs, 1 option, 1 output

=== "Rust"

    ```rust
    use tulip_rs::indicators::adx::indicator;

    let high  = vec![/* ... */];
    let low   = vec![/* ... */];
    let close = vec![/* ... */];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, state) = indicator(&inputs, &[14.0], None).unwrap();

    println!("{:?}", outputs[0]); // ADX values
    ```

=== "Python"

    ```python
    high  = np.array([...], dtype=np.float64)
    low   = np.array([...], dtype=np.float64)
    close = np.array([...], dtype=np.float64)

    outputs, state = tulip_rs.indicators.adx.indicator([high, low, close], [14.0])

    print(outputs[0])  # ADX values
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.adx.indicator([high, low, close], [14]);

    console.log(outputs[0]); // ADX values
    ```

---

## Error Handling

=== "Rust"

    `indicator()` returns a `Result`. The `IndicatorError` enum covers the common failure cases:

    | Variant | Cause |
    |---|---|
    | `IndicatorError::NotEnoughData` | Input length is shorter than the indicator's minimum lookback |
    | `IndicatorError::InvalidOption` | An option value is out of range (e.g. period < 1) |
    | `IndicatorError::InputLengthMismatch` | Multi-input indicators received slices of different lengths |

    ```rust
    match indicator(&[close.as_slice()], &[5.0], None) {
        Ok((outputs, state)) => { /* use outputs */ }
        Err(e) => eprintln!("Indicator error: {e}"),
    }
    ```

=== "Python"

    On failure, the Python bindings raise a `ValueError` with a descriptive message:

    ```python
    try:
        outputs, state = tulip_rs.indicators.sma.indicator([close], [5.0])
    except ValueError as e:
        print(f"Indicator error: {e}")
    ```

=== "Node.js"

    ```javascript
    try {
        const [outputs, state] = ti.sma.indicator([close], [5]);
        // use outputs
    } catch (e) {
        console.error(`Indicator error: ${e.message}`);
    }
    ```

---

## Next Steps

| Topic | Page |
|---|---|
| Full indicator reference | [Indicators — Overview](indicators/index.md) |
| Indicator metadata, optional outputs, min data | [Indicator API](indicators/indicator_api.md) |
| SIMD acceleration concepts | [SIMD](simd.md) |
| Streaming / incremental computation | [State Management](state_management.md) |
| Language bindings details | [Language Bindings](language_bindings.md) |
| Candlestick patterns | [Candlestick Patterns](candlestick_patterns.md) |
