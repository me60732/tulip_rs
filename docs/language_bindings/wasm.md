# Browser (WebAssembly)

**Package:** [`tulip-rs-wasm`](https://www.npmjs.com/package/tulip-rs-wasm)

The WebAssembly binding is built with [wasm-pack](https://rustwasm.github.io/wasm-pack/) and published to npm. It brings the full indicator set to any modern browser with no server round-trips and no native dependencies. The API mirrors the Node.js binding closely — the same `indicator()` / `batchIndicator()` / `info` patterns apply.

### Installation

```bash
npm install tulip-rs-wasm
```

---

### Initialisation

The WASM module must be compiled and instantiated before any indicator calls. How you do this depends on your build setup:

=== "Vite (vite-plugin-wasm)"

    `vite-plugin-wasm` resolves the `.wasm` asset URL automatically, but you must still call `init()` to trigger async compilation and instantiation:

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // no URL needed — bundler resolves the asset
    ```

    ```javascript
    // vite.config.js
    import wasm from 'vite-plugin-wasm';
    export default { plugins: [wasm()] };
    ```

=== "webpack 5"

    With `asyncWebAssembly: true`, webpack instantiates the WASM module automatically on import — no `init()` call needed:

    ```javascript
    import * as ti from 'tulip-rs-wasm';
    // ready to use immediately — no init() required
    ```

    ```javascript
    // webpack.config.js
    module.exports = { experiments: { asyncWebAssembly: true } };
    ```

=== "CDN / plain HTML"

    Without a bundler, pass the full URL of the `.wasm` binary to `init()`:

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init('https://cdn.jsdelivr.net/npm/tulip-rs-wasm@0.1.10/pkg/tulip_rs_wasm_bg.wasm');
    ```

---

### Quick Examples

**SMA — single input, single output:**

```javascript
import { init } from 'tulip-rs-wasm';
import * as ti from 'tulip-rs-wasm';

await init(); // Vite setup — see Initialisation above

const close = [81.59, 81.06, 82.87, 83.00, 83.61,
               83.15, 82.84, 83.99, 84.55, 84.36];

const [outputs, state] = ti.sma.indicator([close], [5]);
console.log('SMA(5):', outputs[0]);
```

**MACD — three outputs:**

```javascript
const [outputs, state] = ti.macd.indicator([close], [12, 26, 9]);

const macdLine  = outputs[0];
const signal    = outputs[1];
const histogram = outputs[2];
```

**Multi-input (ADX):**

```javascript
const [outputs, state] = ti.adx.indicator([high, low, close], [14]);
console.log('ADX:', outputs[0]);
```

**State continuation — O(1) streaming:**

```javascript
// Initial computation
const [, state] = ti.sma.indicator([close], [5]);

// Feed one new bar at a time — no history reprocessing
const newValues = state.batchIndicator([[newClose]]);
console.log('New SMA value:', newValues[0]);
```

---

### Indicator Info

Every indicator exposes an `info` lazy getter (populated after `init()`) with the same shape as the Node.js binding:

```javascript
import { init, sma } from 'tulip-rs-wasm';

await init();

const info = sma.info;
console.log(info.name);            // 'sma'
console.log(info.fullName);        // 'Simple Moving Average'
console.log(info.indicatorType);   // 'Trend'
console.log(info.inputs);          // ['real']
console.log(info.options);         // ['period']
console.log(info.outputs);         // ['sma']
console.log(info.optionalOutputs); // []
console.log(info.displayGroups);
// [ { id: 'sma', label: 'SMA', displayType: 'Overlay', outputs: ['sma'] } ]

sma.minData([5]);            // minimum bars needed to produce output
```

---

### State Object API

Similar to the Node.js binding. `batchIndicator` returns `Float64Array[]` (zero-copy memory views). Use `Array.from(result[0])` to convert to a plain JS array if needed.

| Method | Signature | Description |
|---|---|---|
| `batchIndicator` | `(inputs: number[][], mask?: boolean[]) => Float64Array[]` | Continue computation on new bars |
| `toJson` | `() => string` | Serialise state to JSON for persistence |

---
