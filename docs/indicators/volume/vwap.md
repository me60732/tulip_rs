# VWAP — Volume Weighted Average Price

The average price weighted by trading volume over the entire input window; commonly used as a benchmark to evaluate execution quality — prices above VWAP are considered bullish, below bearish.

**Inputs:** `[high, low, close, volume]` &nbsp;|&nbsp; **Options:** `[]` &nbsp;|&nbsp; **Outputs:** `[vwap]` &nbsp;|&nbsp; **Optional:** `[typprice]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::vwap::{Vwap, Indicator, TIndicatorState};

    let high   = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                      89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                      91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10_f64];
    let low    = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                      88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                      90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00_f64];
    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];
    let volume = vec![1500.0, 2000.0, 1800.0, 2200.0, 1700.0, 2500.0, 2100.0, 1900.0, 2300.0, 1600.0,
                      2800.0, 2400.0, 2100.0, 1800.0, 2600.0, 2200.0, 1900.0, 2400.0, 2000.0, 2100.0,
                      2300.0, 1700.0, 2500.0, 1800.0, 2000.0, 2100.0, 1600.0, 2200.0, 2400.0, 1900.0,
                      2300.0, 1800.0, 2100.0, 2500.0, 1700.0, 2000.0, 2200.0, 1900.0, 2400.0, 2100.0_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()];
    let (outputs, _state) = Vwap::indicator(&inputs, &[], None).unwrap();
    println!("VWAP: {:?}", outputs[0]);

    // State continuation
    let n = high.len() - 5;
    let partial_inputs = [&high[..n], &low[..n], &close[..n], &volume[..n]];
    let (outputs2, mut state) = indicator(&partial_inputs, &[], None).unwrap();
    println!("Partial VWAP: {:?}", outputs2[0]);

    let rest_inputs = [&high[n..], &low[n..], &close[n..], &volume[n..]];
    let continued = state.batch_indicator(&rest_inputs, None).unwrap();
    println!("Continued VWAP: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                       85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                       89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                       91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                       88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                       90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)
    volume = np.array([1500, 2000, 1800, 2200, 1700, 2500, 2100, 1900, 2300, 1600,
                       2800, 2400, 2100, 1800, 2600, 2200, 1900, 2400, 2000, 2100,
                       2300, 1700, 2500, 1800, 2000, 2100, 1600, 2200, 2400, 1900,
                       2300, 1800, 2100, 2500, 1700, 2000, 2200, 1900, 2400, 2100], dtype=np.float64)

    outputs, state = tulip_rs.indicators.vwap.indicator([high, low, close, volume], [])
    print("VWAP:", outputs[0])

    # State continuation
    n = len(high) - 5
    outputs2, state = tulip_rs.indicators.vwap.indicator(
        [high[:n], low[:n], close[:n], volume[:n]], []
    )
    print("Partial VWAP:", outputs2[0])

    continued = state.batch_indicator([high[n:], low[n:], close[n:], volume[n:]])
    print("Continued VWAP:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                                      85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                                      89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                                      91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10]);
    const low    = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                                      84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                                      88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                                      90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00]);
    const close  = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20]);
    const volume = Float64Array.from([1500, 2000, 1800, 2200, 1700, 2500, 2100, 1900, 2300, 1600,
                                      2800, 2400, 2100, 1800, 2600, 2200, 1900, 2400, 2000, 2100,
                                      2300, 1700, 2500, 1800, 2000, 2100, 1600, 2200, 2400, 1900,
                                      2300, 1800, 2100, 2500, 1700, 2000, 2200, 1900, 2400, 2100]);

    const [outputs, state] = ti.vwap.indicator([high, low, close, volume], []);
    console.log('VWAP:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.vwap.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued VWAP:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                    89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                    91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                    88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                    90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00];
    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                    85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                    88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                    90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20];
    const volume = [1500, 2000, 1800, 2200, 1700, 2500, 2100, 1900, 2300, 1600,
                    2800, 2400, 2100, 1800, 2600, 2200, 1900, 2400, 2000, 2100,
                    2300, 1700, 2500, 1800, 2000, 2100, 1600, 2200, 2400, 1900,
                    2300, 1800, 2100, 2500, 1700, 2000, 2200, 1900, 2400, 2100];

    const [outputs, state] = ti.vwap.indicator([high, low, close, volume], []);
    console.log('VWAP:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.vwap.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued VWAP:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `vwap` exposes 1 optional output: `typprice`. The typical price `(high + low + close) / 3` is the per-bar price used internally when computing the weighted average. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::vwap::{Vwap, Indicator, TIndicatorState};

    // ... (same high, low, close, volume data as above)
    let mask = [true];
    let (outputs, _state) = Vwap::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()],
        &[],
        Some(&mask),
    ).unwrap();

    let vwap     = &outputs[0]; // vwap (primary)
    let typprice = &outputs[1]; // typprice (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    # ... (same high, low, close, volume data as above)
    outputs, state = tulip_rs.indicators.vwap.indicator(
        [high, low, close, volume], [],
        optional_outputs=[True],
    )

    vwap     = outputs[0]  # vwap (primary)
    typprice = outputs[1]  # typprice (optional — requested)
    ```

=== "Node.js"

    `vwap` exposes 1 optional output: `typprice`.

    ```javascript
    const [allOut] = ti.vwap.indicator([high, low, close, volume], [], [true]);
    const vwap     = allOut[0]; // primary
    const typprice = allOut[1]; // optional 0: typprice
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.vwap.indicator([high, low, close, volume], [], [true]);
    const vwap     = allOut[0]; // primary
    const typprice = allOut[1]; // optional 0: typprice
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::vwap::{Vwap, Indicator};

    let h1 = high.clone();   let l1 = low.clone();
    let c1 = close.clone();  let v1 = volume.clone();
    let h2 = h1.clone();     let l2 = l1.clone();
    let c2 = c1.clone();     let v2 = v1.clone();
    let h3 = h1.clone();     let l3 = l1.clone();
    let c3 = c1.clone();     let v3 = v1.clone();
    let h4 = h1.clone();     let l4 = l1.clone();
    let c4 = c1.clone();     let v4 = v1.clone();

    let inputs: [&[&[f64]; 4]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice(), v4.as_slice()],
    ];

    let results = Vwap::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    simd_inputs = [
        [high,        low,        close,        volume],
        [high + 0.5,  low + 0.5,  close + 0.5,  volume * 1.1],
        [high - 0.5,  low - 0.5,  close - 0.5,  volume * 0.9],
        [high * 1.01, low * 1.01, close * 1.01, volume * 1.05],
    ]
    outputs_list, states = tulip_rs.indicators.vwap.simd_by_assets(simd_inputs, [])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice(), volume.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.vwap.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
