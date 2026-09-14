# Node.js

**Repository:** [github.com/me60732/tulip-rs-node](https://github.com/me60732/tulip-rs-node)

The Node.js binding is built with [napi-rs](https://napi.rs/) and distributed as a native addon via npm. It exposes every indicator, both SIMD modes, state management, and candlestick patterns with a clean, idiomatic JavaScript interface. Building from source is recommended for full `target-cpu=native` performance; prebuilt binaries are provided for Linux x64, macOS x64, and macOS arm64.

### Installation

**From source (recommended)** — native CPU codegen (`-C target-cpu=native`) lets LLVM use every instruction set your CPU supports — a substantial speed-up across both the scalar and SIMD indicator paths, well beyond the prebuilt binaries:

```bash
git clone https://github.com/me60732/tulip-rs-node
cd tulip-rs-node
git checkout {latest tag}   # or omit for the bleeding edge — see the repo's tags page
npm install
RUSTFLAGS="-C target-cpu=native" npm run build
```

**Requirements:** Node.js 18+, Rust nightly (pinned by the repo's `rust-toolchain.toml`)

**From npm** — use this only when the deployment target architecture is unknown or a Rust toolchain can't run there. Prebuilt binaries are provided for Linux x64, macOS x64, and macOS arm64:

```bash
npm install tulip-rs-node
```

---

### Quick Examples

**SMA — single input, single output:**

```javascript
import * as ti from 'tulip-rs-node';

const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                                  83.15, 82.84, 83.99, 84.55, 84.36]);

const [outputs, state] = ti.sma.indicator([close], [5]);
const smaValues = outputs[0]; // Float64Array
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
| `batchIndicator` | `(inputs: Float64Array[], optionalOutputsMask?: boolean[]) => Float64Array[]` | Continue computation on new bars; pass the same optional outputs mask used in `indicator()` |
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
const newBars = Float64Array.from([87.10, 88.25]);
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
//   inputs: ['real'],
//   options: ['period'],
//   outputs: ['sma'],
//   optionalOutputs: [],
//   displayGroups: [
//     { id: 'sma', label: 'SMA', displayType: 'Overlay', outputs: ['sma'] }
//   ]
// }

ti.sma.minData([5]);            // minimum bars needed to produce output
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
