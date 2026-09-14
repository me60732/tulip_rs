# Getting Started

## Installation

=== "Rust"

    Add TulipRS to your `Cargo.toml`:

    ```toml
    [dependencies]
    tulip_rs = "0.2.8"
    ```

    For a reproducible build, pin the source to the latest release tag (`{latest tag}` = the newest tag from the repository's [tags page](https://github.com/me60732/tulip_rs/tags), e.g. `v0.2.7`):

    ```toml
    [dependencies]
    tulip_rs = { git = "https://github.com/me60732/tulip_rs", tag = "{latest tag}" }
    ```

    Omit the `tag` to track the latest unreleased `main`.

    TulipRS uses the `portable_simd` nightly language feature internally and requires a **nightly** Rust toolchain. The correct nightly version is pinned automatically via the `rust-toolchain.toml` file at the root of the repository — no manual toolchain management is needed.

=== "C"

    ```bash
    git clone https://github.com/me60732/tulip_rs_ffi
    cd tulip_rs_ffi
    cargo build --release
    ```

    Link your application:

    ```bash
    cc -O2 app.c -I include -L target/release -ltulip_rs_ffi -Wl,-rpath,target/release
    ```

    SIMD features are always compiled in — no feature flag selection is needed for the FFI library.

=== "Python"

    **From source (recommended)** — compiling on your machine with `-C target-cpu=native` lets LLVM generate code for every instruction set your CPU supports — this speeds up the scalar indicators as much as the SIMD ones, and is measurably faster than the generic prebuilt wheels:

    ```bash
    git clone https://github.com/me60732/tulip_rs_python
    cd tulip_rs_python
    git checkout {latest tag}   # or omit for the bleeding edge — see the repo's tags page
    RUSTFLAGS="-C target-cpu=native" maturin develop --release
    ```

    Requirements: Python 3.8+, Rust nightly (pinned by the repo's `rust-toolchain.toml`).

    **From PyPI** — use this only when the deployment target architecture is unknown or a Rust toolchain can't run there (prebuilt wheels must ship generic x86-64/ARM baselines):

    ```bash
    pip install tulip-rs
    ```

=== "Node.js"

    **From source (recommended)** — native CPU codegen (`-C target-cpu=native`) lets LLVM use every instruction set your CPU supports — a substantial speed-up across both the scalar and SIMD indicator paths, well beyond the prebuilt binaries:

    ```bash
    git clone https://github.com/me60732/tulip-rs-node
    cd tulip-rs-node
    git checkout {latest tag}   # or omit for the bleeding edge — see the repo's tags page
    npm install
    RUSTFLAGS="-C target-cpu=native" npm run build
    ```

    Requirements: Node.js 18+, Rust nightly (pinned by the repo's `rust-toolchain.toml`).

    **From npm** — use this only when the deployment target architecture is unknown or a Rust toolchain can't run there. Prebuilt binaries are provided for Linux x64, macOS x64, and macOS arm64:

    ```bash
    npm install tulip-rs-node
    ```

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

!!! note "FFI library always has SIMD"
    The `tulip_rs_ffi` library builds the core with SIMD features enabled; feature flag choices only concern direct Rust consumers.

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

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    ```

    Every indicator follows the same pattern:

    ```c
    CIndicatorResult <ind>_indicator(
        const double *inputs[INPUTS],
        size_t data_len,
        const double options[OPTIONS],
        const bool optional_outputs[],
        size_t num_optional
    );
    ```

    - `inputs` — an array of pointers, one per input series (e.g. `[close]` for SMA; `[high, low, close]` for ADX).
    - `options` — indicator parameters as `f64`, in the order documented for each indicator.
    - `optional_outputs` — pass `NULL` and `0` unless you specifically want to suppress optional output series.
    - The return value is a `CIndicatorResult`:
        - `error` — check against `C_INDICATOR_ERROR_OK`; see [Error Handling](#error-handling).
        - `outputs[i]` — pointer to the i-th output series.
        - `output_lens[i]` — length of the i-th output series.
        - `num_outputs` — number of output series returned.
        - `state` — opaque state pointer for streaming via `<ind>_batch()`.
    - Memory management:
        - `tulip_ffi_result_free(result)` frees outputs and optional state from `indicator()` calls.
        - `<ind>_state_free(state)` frees the state object (call once after your last batch).
        - `tulip_ffi_batch_result_free(batch)` frees outputs from `batch()` calls.

    Streaming pattern:

    ```c
    // Seed with initial data
    CIndicatorResult r = <ind>_indicator(inputs, len, options, NULL, 0);
    void *state = r.state;
    tulip_ffi_result_free(r);  // outputs freed; state kept alive

    // Append new bars
    CBatchResult b = <ind>_batch(state, new_inputs, new_len, NULL, 0);
    tulip_ffi_batch_result_free(b);

    // Final cleanup (after last batch)
    <ind>_state_free(state);
    ```

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

    - `inputs` — an array of `Float64Array`, one per input series.
    - `options` — an array of `number` values, in the order documented for each indicator.
    - `outputs` — an array of `Float64Array`, one per output series, already trimmed to valid length.
    - `state` — a state object that exposes `batchIndicator()` and JSON/Buffer serialisation.

    !!! note "Candlestick patterns use separate arrays per OHLC series."
        See the [Candlestick Patterns](candlestick_patterns.md) page for details.

---

## Examples

### SMA — 1 input, 1 option, 1 output

=== "Rust"

    ```rust
    use tulip_rs::indicators::sma::{Sma, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, state) = Sma::indicator(&[close.as_slice()], &[5.0], None).unwrap();

    println!("{:?}", outputs[0]); // SMA(5) — length is close.len() - period + 1
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[SMA_OPTIONS] = {5.0}; // period
    const double *inputs[SMA_INPUTS] = {close};

    CIndicatorResult r = sma_indicator(inputs, 10, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "sma_indicator failed: error=%d\n", r.error);
        return 1;
    }

    printf("SMA(5): [");
    for (uintptr_t i = 0; i < r.output_lens[0]; i++) {
        printf("%.4f", r.outputs[0][i]);
        if (i + 1 < r.output_lens[0]) printf(", ");
    }
    printf("]\n");

    tulip_ffi_result_free(r);
    sma_state_free(r.state);
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

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                                     83.15, 82.84, 83.99, 84.55, 84.36]);

    const [outputs, state] = ti.sma.indicator([close], [5]);

    console.log(outputs[0]); // SMA(5) values
    ```

---

### MACD — 1 input, 3 options, 3 outputs

=== "Rust"

    ```rust
    use tulip_rs::indicators::macd::{Macd, Indicator};

    // options: [fast_period, slow_period, signal_period]
    let (outputs, state) = Macd::indicator(&[close.as_slice()], &[12.0, 26.0, 9.0], None).unwrap();

    let macd_line  = &outputs[0]; // MACD line
    let signal     = &outputs[1]; // Signal line
    let histogram  = &outputs[2]; // Histogram
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[MACD_OPTIONS] = {12.0, 26.0, 9.0}; // fast_period, slow_period, signal_period
    const double *inputs[MACD_INPUTS] = {close};

    CIndicatorResult r = macd_indicator(inputs, 10, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "macd_indicator failed: error=%d\n", r.error);
        return 1;
    }

    printf("MACD(12,26,9):\n");
    printf("  macd_line:   [");
    for (uintptr_t i = 0; i < r.output_lens[0]; i++) {
        printf("%.4f", r.outputs[0][i]);
        if (i + 1 < r.output_lens[0]) printf(", ");
    }
    printf("]\n");

    printf("  signal:      [");
    for (uintptr_t i = 0; i < r.output_lens[1]; i++) {
        printf("%.4f", r.outputs[1][i]);
        if (i + 1 < r.output_lens[1]) printf(", ");
    }
    printf("]\n");

    printf("  histogram:   [");
    for (uintptr_t i = 0; i < r.output_lens[2]; i++) {
        printf("%.4f", r.outputs[2][i]);
        if (i + 1 < r.output_lens[2]) printf(", ");
    }
    printf("]\n");

    tulip_ffi_result_free(r);
    macd_state_free(r.state);
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

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                                     83.15, 82.84, 83.99, 84.55, 84.36]);

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
    use tulip_rs::indicators::adx::{Adx, Indicator};

    let high  = vec![/* ... */];
    let low   = vec![/* ... */];
    let close = vec![/* ... */];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, state) = Adx::indicator(&inputs, &[14.0], None).unwrap();

    println!("{:?}", outputs[0]); // ADX values
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    double options[ADX_OPTIONS] = {14.0}; // period
    const double *inputs[ADX_INPUTS] = {high, low, close};

    CIndicatorResult r = adx_indicator(inputs, 10, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "adx_indicator failed: error=%d\n", r.error);
        return 1;
    }

    printf("ADX(14): [");
    for (uintptr_t i = 0; i < r.output_lens[0]; i++) {
        printf("%.4f", r.outputs[0][i]);
        if (i + 1 < r.output_lens[0]) printf(", ");
    }
    printf("]\n");

    tulip_ffi_result_free(r);
    adx_state_free(r.state);
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

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

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
    match [indicator].indicator(&[close.as_slice()], &[5.0], None) {
        Ok((outputs, state)) => { /* use outputs */ }
        Err(e) => eprintln!("Indicator error: {e}"),
    }
    ```

=== "C"

    Every `CIndicatorResult` starts with an `error` field of type `CIndicatorError`. Check it against `C_INDICATOR_ERROR_OK`:

    ```c
    CIndicatorResult r = <ind>_indicator(inputs, len, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "<ind> failed: error=%d\n", r.error);
        return 1;
    }
    ```

    | Variant | Cause |
    |---|---|
    | `C_INDICATOR_ERROR_OK` | Success |
    | `C_INDICATOR_ERROR_INVALID_INPUTS` | NULL inputs or insufficient data pointers |
    | `C_INDICATOR_ERROR_NOT_ENOUGH_DATA` | Input length shorter than minimum required |
    | `C_INDICATOR_ERROR_INVALID_OPTIONS` | Invalid option values (e.g. period < 1) |
    | `C_INDICATOR_ERROR_INVALID_INDICATOR_STATE` | State pointer is invalid or has been freed |

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
