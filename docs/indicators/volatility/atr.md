# ATR — Average True Range

Measures market volatility by averaging the true range (the greatest of: high-low, |high-prev_close|, |low-prev_close|) over `period` bars.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[atr]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::atr::{Atr, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = Atr::indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // ATR values

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = Atr::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high  = vec![85.90_f64];
    let new_low   = vec![84.03_f64];
    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[ATR_OPTIONS] = {14.0}; // period
    const double *inputs[ATR_INPUTS] = {high, low, close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = atr_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the ATR series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    atr_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult pr = atr_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.90};
    double new_low[] = {84.03};
    double new_close[] = {85.53};
    const double *new_inputs[ATR_INPUTS] = {new_high, new_low, new_close};
    CBatchResult br = atr_batch(pr.state, new_inputs, 1, NULL, 0);
    /* br.outputs[0] -> ATR values for just the new bar */
    tulip_ffi_batch_result_free(br);
    tulip_ffi_result_free(pr);
    atr_state_free(pr.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.atr.indicator([high, low, close], [14.0])
    print(outputs[0])  # ATR values

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.atr.indicator([high, low, close], [14]);
    console.log('ATR(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.atr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued ATR:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.atr.indicator([high, low, close], [14]);
    console.log('ATR(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.atr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued ATR:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `atr` exposes 1 optional output: `"tr"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::atr::{Atr, Indicator, TIndicatorState};

    let high  = vec![82.59, 82.06, 83.87, 84.00, 84.61,
                     84.15, 83.84, 84.99, 85.55, 85.36_f64];
    let low   = vec![80.59, 80.06, 81.87, 82.00, 82.61,
                     82.15, 81.84, 82.99, 83.55, 83.36_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // request tr
    let (outputs, _state) = Atr::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let atr = &outputs[0]; // atr (primary)
    let tr  = &outputs[1]; // tr  (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.59, 82.06, 83.87, 84.00, 84.61,
                      84.15, 83.84, 84.99, 85.55, 85.36], dtype=np.float64)
    low   = np.array([80.59, 80.06, 81.87, 82.00, 82.61,
                      82.15, 81.84, 82.99, 83.55, 83.36], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.atr.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True],
    )

    atr = outputs[0]  # atr (primary)
    tr  = outputs[1]  # tr  (optional — requested)
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.59, 82.06, 83.87, 84.00, 84.61,
                     84.15, 83.84, 84.99, 85.55, 85.36};
    double low[] = {80.59, 80.06, 81.87, 82.00, 82.61,
                    82.15, 81.84, 82.99, 83.55, 83.36};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[ATR_OPTIONS] = {14.0}; // period
    const double *inputs[ATR_INPUTS] = {high, low, close};

    /* request the optional tr output */
    bool optional_outputs[1] = {true};

    CIndicatorResult r = atr_indicator(inputs, 10, options, optional_outputs, 1);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "atr_indicator failed: error=%d\n", r.error);
        return 1;
    }
    /* r.outputs[0] -> atr (primary) */
    /* r.outputs[1] -> tr (optional — requested) */
    tulip_ffi_result_free(r);
    atr_state_free(r.state);
    ```

=== "Node.js"

    `atr` exposes 1 optional output: `tr`.

    ```javascript
    const [allOut] = ti.atr.indicator([high, low, close], [14], [true]);
    const atr = allOut[0]; // primary
    const tr  = allOut[1]; // optional 0: tr
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.atr.indicator([high, low, close], [14], [true]);
    const atr = allOut[0]; // primary
    const tr  = allOut[1]; // optional 0: tr
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::atr::{Atr, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = Atr::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::atr::{Atr, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = Atr::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double h2[] = {72.15, 71.89, 73.03, 73.30, 73.85, 73.90, 73.33, 74.30, 74.84, 75.00};
    double l2[] = {71.29, 70.64, 71.31, 72.65, 73.07, 73.11, 72.49, 72.30, 74.15, 74.11};
    double c2[] = {71.59, 71.06, 72.87, 73.00, 73.61, 73.15, 72.84, 73.99, 74.55, 74.36};
    double h3[] = {52.15, 51.89, 53.03, 53.30, 53.85, 53.90, 53.33, 54.30, 54.84, 55.00};
    double l3[] = {51.29, 50.64, 51.31, 52.65, 53.07, 53.11, 52.49, 52.30, 54.15, 54.11};
    double c3[] = {51.59, 51.06, 52.87, 53.00, 53.61, 53.15, 52.84, 53.99, 54.55, 54.36};
    double h4[] = {102.15, 101.89, 103.03, 103.30, 103.85, 103.90, 103.33, 104.30, 104.84, 105.00};
    double l4[] = {101.29, 100.64, 101.31, 102.65, 103.07, 103.11, 102.49, 102.30, 104.15, 104.11};
    double c4[] = {101.59, 101.06, 102.87, 103.00, 103.61, 103.15, 102.84, 103.99, 104.55, 104.36};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[ATR_INPUTS] = {h1, l1, c1};
    const double *asset2[ATR_INPUTS] = {h2, l2, c2};
    const double *asset3[ATR_INPUTS] = {h3, l3, c3};
    const double *asset4[ATR_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = atr_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's ATR series */
        atr_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o7[] = {7.0};
    double o14[] = {14.0};
    double o21[] = {21.0};
    double o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = atr_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i][0] -> ATR for period set i */
    for (uintptr_t i = 0; i < r.num_results; i++) atr_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.atr.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.atr.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.atr.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.atr.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
