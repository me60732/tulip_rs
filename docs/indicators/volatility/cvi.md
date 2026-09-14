# CVI — Chaikin Volatility

Measures the rate of change of the trading range (high minus low) EMA. Rising values indicate increasing volatility.

**Inputs:** `[high, low]` | **Options:** `[period]` | **Outputs:** `[cvi]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::cvi::{Cvi, Indicator, TIndicatorState};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = Cvi::indicator(&inputs, &[10.0], None).unwrap();
    println!("{:?}", outputs[0]); // CVI values

    // State continuation — feed new bars without reprocessing history
    let partial_high = high[..8].to_vec();
    let partial_low  = low[..8].to_vec();
    let (outputs2, mut state) = Cvi::indicator(&[partial_high.as_slice(), partial_low.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high = vec![85.90_f64];
    let new_low  = vec![84.03_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice()],
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
    double options[CVI_OPTIONS] = {10.0}; // period
    const double *inputs[CVI_INPUTS] = {high, low};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = cvi_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> cvi_line */
    tulip_ffi_result_free(r);
    cvi_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = cvi_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.90};
    double new_low[] = {84.03};
    const double *new_inputs[CVI_INPUTS] = {new_high, new_low};
    CBatchResult b = cvi_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> cvi_line for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    cvi_state_free(p.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    outputs, state = tulip_rs.indicators.cvi.indicator([high, low], [10.0])
    print(outputs[0])  # CVI values

    # State continuation
    new_high = np.array([85.30], dtype=np.float64)
    new_low  = np.array([84.60], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low  = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);

    const [outputs, state] = ti.cvi.indicator([high, low], [10]);
    console.log('CVI(10):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.cvi.indicator([high.slice(0, n), low.slice(0, n)], [10]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued CVI:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.cvi.indicator([high, low], [10]);
    console.log('CVI(10):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.cvi.indicator([high.slice(0, n), low.slice(0, n)], [10]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued CVI:', continued[0]);
    ```

### SIMD

=== "Rust"

    ```rust
    use tulip_rs::indicators::cvi::{Cvi, Indicator};

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = Cvi::indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::cvi::{Cvi, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let results = Cvi::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1_high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double a1_low[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    const double *const asset1[CVI_INPUTS] = {a1_high, a1_low};

    double a2_high[] = {90.0, 89.5, 91.0, 91.3, 91.9, 92.0, 91.5, 92.0, 92.5, 92.8};
    double a2_low[] = {89.0, 88.5, 90.0, 90.3, 90.9, 91.0, 90.5, 91.0, 91.5, 91.8};
    const double *const asset2[CVI_INPUTS] = {a2_high, a2_low};

    double a3_high[] = {75.0 + (double)i * 0.2 for i in 0..10}; /* simplified */
    double a3_low[] = {74.0 + (double)i * 0.2 for i in 0..10};
    const double *const asset3[CVI_INPUTS] = {a3_high, a3_low};

    double a4_high[] = {85.0 - (double)i * 0.1 for i in 0..10};
    double a4_low[] = {84.0 - (double)i * 0.1 for i in 0..10};
    const double *const asset4[CVI_INPUTS] = {a4_high, a4_low};

    /* simd_inputs is indexed by asset (the N=4 SIMD lanes) */
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = cvi_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's cvi_line */
        cvi_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o5[] = {5.0}, o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0};
    const double *const simd_opts[4] = {o5, o10, o14, o20};

    CSimdResult r = cvi_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i */
    for (uintptr_t i = 0; i < r.num_results; i++) cvi_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1], [h2, l2], [h3, l3], [h4, l4]]
    outputs_list, states = tulip_rs.indicators.cvi.simd_by_assets(simd_inputs, [10.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.cvi.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.cvi.simdByAssets(simdInputs, [10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.cvi.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
