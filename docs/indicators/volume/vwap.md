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

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]   = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                       85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                       89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                       91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10};
    double low[]    = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                       88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                       90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00};
    double close[]  = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double volume[] = {1500, 2000, 1800, 2200, 1700, 2500, 2100, 1900, 2300, 1600,
                       2800, 2400, 2100, 1800, 2600, 2200, 1900, 2400, 2000, 2100,
                       2300, 1700, 2500, 1800, 2000, 2100, 1600, 2200, 2400, 1900,
                       2300, 1800, 2100, 2500, 1700, 2000, 2200, 1900, 2400, 2100};

    /* VWAP has no options (VWAP_OPTIONS=0); the inputs are [high, low, close, volume] */
    const double *inputs[VWAP_INPUTS] = {high, low, close, volume};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = vwap_indicator(inputs, 40, NULL, NULL, 0);
    /* r.outputs[0] -> the VWAP series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    vwap_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = vwap_indicator(inputs, 35, NULL, NULL, 0);
    double new_high[]   = {91.30, 92.00, 92.60, 93.00, 92.30};
    double new_low[]    = {90.20, 90.90, 91.50, 91.80, 91.20};
    double new_close[]  = {90.50, 91.20, 91.80, 92.10, 91.50};
    double new_volume[] = {2300, 1800, 2100, 2500, 1700};
    const double *new_inputs[VWAP_INPUTS] = {new_high, new_low, new_close, new_volume};
    CBatchResult b = vwap_batch(p.state, new_inputs, 5, NULL, 0);
    /* b.outputs[0] -> VWAP values for just the five new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    vwap_state_free(p.state);
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

=== "C"

    `vwap` exposes 1 optional output: `typprice`. This indicator has no options, so the options array is NULL (or empty).

    ```c
    #include "tulip_rs_ffi.h"

    /* ... (same high, low, close, volume data as above) */
    const double *inputs[VWAP_INPUTS] = {high, low, close, volume};

    /* Request the typprice optional output (mask has one bool per optional output) */
    bool mask[] = {true};
    CIndicatorResult r = vwap_indicator(inputs, 40, NULL, mask, 1);
    /* r.outputs[0] -> vwap (primary), r.outputs[1] -> typprice (optional) */
    tulip_ffi_result_free(r);
    vwap_state_free(r.state);
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

=== "C"

    **By assets** — same option applied to 4 assets in one call (N must be 2/4/8/16). This indicator has no options.

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29};
    double v1[] = {1500, 2000, 1800, 2200, 1700, 2500, 2100, 1900, 2300, 1600,
                   2800, 2400, 2100, 1800, 2600};

    const double *const asset1[VWAP_INPUTS] = {h1, l1, c1, v1};

    /* Asset 2 with slightly different prices */
    double h2[] = {82.25, 81.99, 83.13, 83.40, 83.95, 84.00, 83.43, 84.40, 84.94, 85.10,
                   86.00, 86.68, 87.08, 88.10, 87.97};
    double l2[] = {81.39, 80.74, 81.41, 82.75, 83.17, 83.21, 82.59, 82.40, 84.25, 84.21,
                   84.13, 85.49, 85.86, 87.27, 87.11};
    double c2[] = {81.69, 81.16, 82.97, 83.10, 83.71, 83.25, 82.94, 84.09, 84.65, 84.46,
                   85.63, 86.64, 86.99, 87.87, 87.39};
    double v2[] = {1600, 2100, 1900, 2300, 1800, 2600, 2200, 2000, 2400, 1700,
                   2900, 2500, 2200, 1900, 2700};

    const double *const asset2[VWAP_INPUTS] = {h2, l2, c2, v2};

    /* Asset 3 */
    double h3[] = {81.95, 81.69, 82.83, 83.10, 83.65, 83.70, 83.13, 84.10, 84.64, 84.80,
                   85.70, 86.38, 86.78, 87.80, 87.67};
    double l3[] = {81.09, 80.44, 81.11, 82.45, 82.87, 82.91, 82.29, 82.10, 83.95, 83.91,
                   83.83, 85.19, 85.56, 86.87, 86.71};
    double c3[] = {81.39, 80.86, 82.67, 82.80, 83.41, 83.05, 82.64, 83.79, 84.35, 84.16,
                   85.33, 86.34, 86.69, 87.57, 87.09};
    double v3[] = {1400, 1900, 1700, 2100, 1600, 2400, 2000, 1800, 2200, 1500,
                   2700, 2300, 2000, 1700, 2500};

    const double *const asset3[VWAP_INPUTS] = {h3, l3, c3, v3};

    /* Asset 4 */
    double h4[] = {82.35, 82.09, 83.23, 83.50, 84.05, 84.10, 83.53, 84.50, 85.04, 85.20,
                   86.10, 86.78, 87.18, 88.20, 88.07};
    double l4[] = {81.49, 80.84, 81.51, 82.85, 83.27, 83.31, 82.69, 82.50, 84.35, 84.31,
                   84.23, 85.59, 85.96, 87.37, 87.21};
    double c4[] = {81.79, 81.26, 83.07, 83.20, 83.81, 83.35, 83.04, 84.19, 84.75, 84.56,
                   85.73, 86.74, 87.09, 87.97, 87.49};
    double v4[] = {1700, 2200, 2000, 2400, 1900, 2700, 2300, 2100, 2500, 1800,
                   3000, 2600, 2300, 2000, 2800};

    const double *const asset4[VWAP_INPUTS] = {h4, l4, c4, v4};

    /* simd_inputs is indexed by asset (the N=4 SIMD lanes) */
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = vwap_simd_by_assets(simd_inputs, 4, 15, NULL, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's VWAP series */
        vwap_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
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

    _This indicator has no options, so by-options SIMD does not apply.
