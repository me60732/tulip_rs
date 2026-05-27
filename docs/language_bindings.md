# Language Bindings

TulipRS is written in Rust and exposes its full API through language bindings. The core calling convention — inputs, options, outputs, and state — is identical at the Rust boundary regardless of the target language.

---

## Python

**Repository:** [github.com/me60732/tulip_rs_python](https://github.com/me60732/tulip_rs_python)

The Python binding is built with [PyO3](https://pyo3.rs/) and packaged with [maturin](https://github.com/PyO3/maturin). It exposes every indicator, both SIMD modes, state management, and candlestick patterns to Python with a clean, idiomatic interface.

### Installation

**From PyPI (recommended):**

```bash
pip install tulip-rs
```

**From source with native CPU optimisations:**

```bash
git clone https://github.com/me60732/tulip_rs_python
cd tulip_rs_python
RUSTFLAGS="-C target-cpu=native" maturin develop --release
```

**Requirements:** Python 3.8+, Rust 1.70+

---

### Quick Examples

**SMA — single input, single output:**

```python
import numpy as np
import tulip_rs

close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                  83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

outputs, state = tulip_rs.indicators.sma.indicator([close], [5.0])
sma_values = outputs[0]
```

**MACD — three outputs via `get_all_outputs`:**

```python
outputs, state = tulip_rs.indicators.macd.indicator([close], [12.0, 26.0, 9.0])

macd_line = outputs[0]
signal    = outputs[1]
histogram = outputs[2]

# Or unpack as a list
all_out = outputs  # list of 3 numpy arrays
```

**ADX — multiple inputs (high, low, close):**

```python
high  = np.array([...], dtype=np.float64)
low   = np.array([...], dtype=np.float64)
close = np.array([...], dtype=np.float64)

outputs, state = tulip_rs.indicators.adx.indicator([high, low, close], [14.0])
adx_values = outputs[0]
```

**Candlestick pattern detection:**

```python
# Candlestick inputs are plain Python lists, NOT numpy arrays
open_  = [10.0, 10.5, 11.0, 10.8, 11.2]
high_  = [11.0, 11.2, 11.5, 11.1, 11.6]
low_   = [ 9.8, 10.2, 10.7, 10.5, 10.9]
close_ = [10.5, 11.0, 10.9, 11.0, 11.4]

result, state = tulip_rs.indicators.candlestick.candlestick(
    open_, high_, low_, close_,
    [1, 5, 3]  # [candle_period, trend_period, trend_signal_period]
)

for bar_patterns in result:
    for pattern in bar_patterns:
        print(pattern["name"], pattern["forecast"])
```

---

### `IndicatorState` Object API

The `state` returned by every call to `indicator()` (or `batch_indicator()`) exposes the following API:

| Method | Signature | Description |
|---|---|---|
| `batch_indicator` | `(inputs) -> list[np.ndarray]` | Continue computation on new bars; returns new output values only |
| `state_to_json` | `() -> Optional[str]` | Serialise state to a JSON string for persistence |
| `has_state` | property `bool` | Whether the state object holds valid indicator state |
| `num_outputs` | property `int` | Number of output series this indicator produces |

!!! note "Restoring state"
    To restore state from JSON, use the indicator-specific `restore_state(json_str)` function, e.g. `tulip_rs.indicators.sma.restore_state(json_str)`.

---

### Input Types

| Context | Input Type |
|---|---|
| All standard indicators | `list[np.ndarray]` where each array is `dtype=np.float64` |
| Candlestick patterns | Plain Python `list[float]` (one list per OHLC series) |
| SIMD by-assets | `list[list[np.ndarray]]` — outer list is assets, inner list is input series |
| SIMD by-options | `list[np.ndarray]` inputs + `list[list[float]]` options |

!!! warning "Always use float64"
    NumPy defaults to `float64` for most operations, but be explicit: `np.array(data, dtype=np.float64)`. Passing `float32` arrays will raise a `TypeError`.

---

### Introspection

Each indicator module exposes two utility functions:

```python
# Returns a dict describing the indicator
info = tulip_rs.indicators.sma.info()
# {
#   "name": "sma",
#   "full_name": "Simple Moving Average",
#   "indicator_type": "overlay",
#   "inputs": ["real"],
#   "options": ["period"],
#   "outputs": ["sma"],
#   "optional_outputs": []
# }

# Minimum number of input bars required for the given options
min_bars = tulip_rs.indicators.sma.min_data([5.0])  # returns 5
```

---

## Node.js

**Repository:** [github.com/me60732/tulip-rs-node](https://github.com/me60732/tulip-rs-node)

The Node.js binding is built with [napi-rs](https://napi.rs/) and distributed as a prebuilt native addon via npm. It exposes every indicator, both SIMD modes, state management, and candlestick patterns with a clean, idiomatic JavaScript interface. Prebuilt binaries are provided for Linux x64, macOS x64, and macOS arm64 — no Rust toolchain required for end users.

### Installation

**From npm (recommended):**

```bash
npm install tulip-rs-node
```

**From source (for development or native CPU optimisations):**

```bash
git clone https://github.com/me60732/tulip-rs-node
cd tulip-rs-node
npm install
npm run build
```

**Requirements:** Node.js 18+, Rust nightly (only needed when building from source)

---

### Quick Examples

**SMA — single input, single output:**

```javascript
import * as ti from 'tulip-rs-node';

const close = [81.59, 81.06, 82.87, 83.00, 83.61,
               83.15, 82.84, 83.99, 84.55, 84.36];

const [outputs, state] = ti.sma.indicator([close], [5]);
const smaValues = outputs[0]; // number[]
```

**MACD — three outputs:**

```javascript
const [outputs, state] = ti.macd.indicator([close], [12, 26, 9]);

const macdLine  = outputs[0]; // MACD line
const signal    = outputs[1]; // Signal line
const histogram = outputs[2]; // Histogram
```

**ADX — multiple inputs:**

```javascript
const [outputs, state] = ti.adx.indicator([high, low, close], [14]);
const adxValues = outputs[0];
```

**Candlestick pattern detection:**

```javascript
const options = [5, 1, 1]; // candle_period, trend_period, trend_signal_period

const [result, state] = ti.candlestick.indicator(
    [open, high, low, close],
    options
);

result.forEach((patterns, bar) => {
    if (patterns && patterns.length > 0) {
        patterns.forEach(p => {
            console.log(`Bar ${bar}: ${p.fullName} (${p.forecast})`);
        });
    }
});

// Filter to bullish reversals only
const [bullish] = ti.candlestick.indicator(
    [open, high, low, close],
    [5, 1, 1],
    'BullishReversal'
);
```

**SIMD — multiple assets:**

```javascript
const simdInputs = [
    [asset1Close],  // asset 1 — array of input arrays
    [asset2Close],  // asset 2
    [asset3Close],  // asset 3
    [asset4Close],  // asset 4
];

const [results, states] = ti.sma.simdByAssets(simdInputs, [14]);

results.forEach((output, i) => {
    console.log(`Asset ${i + 1} SMA:`, output[0]);
});
```

**SIMD — multiple option sets:**

```javascript
const simdOptions = [[2], [5], [8], [10]]; // 4 period values

const [results, states] = ti.sma.simdByOptions([close], simdOptions);

results.forEach((output, i) => {
    console.log(`Period ${simdOptions[i][0]} SMA:`, output[0]);
});
```

---

### State Object API

The `state` returned by every call to `indicator()` exposes the following API:

| Method / Property | Signature | Description |
|---|---|---|
| `batchIndicator` | `(inputs: number[][], optionalOutputsMask?: boolean[]) => number[][]` | Continue computation on new bars; pass the same optional outputs mask used in `indicator()` |
| `toJson` | `() => string` | Serialise state to a JSON string |
| `toBuffer` | `() => Buffer` | Serialise state to a binary Buffer (faster than JSON) |

**Restoring state:**

```javascript
// From JSON
const json = state.toJson();
const restored = ti.sma.State.fromJson(json);

// From Buffer (faster)
const buf = state.toBuffer();
const restored = ti.sma.State.fromBuffer(buf);

// Continue from restored state
const result = restored.batchIndicator([newBars]);
```

---

### Indicator Info

Every indicator exposes a static `info` property and utility functions:

```javascript
const info = ti.sma.info;
// {
//   name: 'sma',
//   fullName: 'Simple Moving Average',
//   indicatorType: 'Trend',
//   displayType: 'Overlay',
//   inputs: ['real'],
//   options: ['period'],
//   outputs: ['sma'],
//   optionalOutputs: []
// }

ti.sma.minData([5]);            // minimum bars needed to produce output
ti.sma.minDataAccuracy([5], 6); // bars needed for 6-decimal accuracy
```

---

### Optional Outputs

Indicators that expose optional intermediate series accept a boolean mask as the third argument to `indicator()` and `batchIndicator()`:

```javascript
// ADX exposes optional outputs: dx, atr, tr
// Request all three
const [allOut] = ti.adx.indicator([high, low, close], [14], [true, true, true]);
const adx = allOut[0]; // primary
const dx  = allOut[1]; // optional 0: dx
const atr = allOut[2]; // optional 1: atr
const tr  = allOut[3]; // optional 2: tr

// Request only the first optional output (dx)
const [partial] = ti.adx.indicator([high, low, close], [14], [true, false, false]);
const dxOnly = partial[1];

// Pass the same mask to batchIndicator
const continued = state.batchIndicator([newHigh, newLow, newClose], [true, false, false]);
```

Use `ti.adx.info.optionalOutputs` to discover which optional outputs an indicator has and in what order.

---

## Planned Bindings

| Language / Platform | Status | Notes |
|---|---|---|
| **Python** | ✅ Available | `tulip_rs_python` — PyO3 + maturin |
| **R** | 🔜 Planned | `extendr` |
| **Julia** | 🔜 Planned | `CxxWrap.jl` or FFI |

---

## Contributing a Binding

All language bindings share the same Rust calling convention:

1. Inputs arrive as `&[&[f64]]`, one slice per series.
2. Options arrive as `&[f64]`.
3. The function returns `Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError>`.
4. `IndicatorState` implements `serde::Serialize` / `Deserialize` for cross-language state persistence.

A minimal binding only needs to marshal data into `Vec<f64>` slices, call the indicator, and unpack the output. SIMD, state management, and error handling are all handled by the core Rust library — the binding layer stays thin. If you'd like to contribute a binding for another language, open an issue on the [main repository](https://github.com/me60732/tulip_rs) to discuss the approach.
