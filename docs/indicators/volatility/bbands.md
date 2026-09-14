# BBands — Bollinger Bands

Three bands plotted around a moving average. The width expands and contracts with volatility.

**Inputs:** `[real]` | **Options:** `[period, stddev_multiplier]` | **Outputs:** `[lower, middle, upper]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::bbands::{BBands, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // options: [period, stddev_multiplier]
    let (outputs, mut state) = BBands::indicator(&[close.as_slice()], &[20.0, 2.0], None).unwrap();
    println!("Lower:  {:?}", outputs[0]);
    println!("Middle: {:?}", outputs[1]);
    println!("Upper:  {:?}", outputs[2]);

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = BBands::indicator(&[partial.as_slice()], &[20.0, 2.0], None).unwrap();
    println!("Lower:  {:?}", outputs2[0]);
    println!("Middle: {:?}", outputs2[1]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Lower continued:  {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[BBANDS_OPTIONS] = {20.0, 2.0}; // period, std_dev
    const double *inputs[BBANDS_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = bbands_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> lower_band, r.outputs[1] -> middle_band, r.outputs[2] -> upper_band */
    tulip_ffi_result_free(r);
    bbands_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = bbands_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.53};
    const double *new_inputs[BBANDS_INPUTS] = {new_close};
    CBatchResult b = bbands_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> lower_band for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    bbands_state_free(p.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # options: [period, stddev_multiplier]
    outputs, state = tulip_rs.indicators.bbands.indicator([close], [20.0, 2.0])
    print(outputs[0])  # Lower band
    print(outputs[1])  # Middle band
    print(outputs[2])  # Upper band

    # State continuation
    new_close = np.array([85.10, 85.72], dtype=np.float64)
    continued = state.batch_indicator([new_close])
    print(continued[0])  # Lower continued
    print(continued[1])  # Middle continued
    print(continued[2])  # Upper continued
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.bbands.indicator([close], [20, 2]);
    console.log('Lower:', outputs[0]);
    console.log('Middle:', outputs[1]);
    console.log('Upper:', outputs[2]);

    // State continuation
    const [, state2] = ti.bbands.indicator([close.slice(0, -5)], [20, 2]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Lower:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.bbands.indicator([close], [20, 2]);
    console.log('Lower:', outputs[0]);
    console.log('Middle:', outputs[1]);
    console.log('Upper:', outputs[2]);

    // State continuation
    const [, state2] = ti.bbands.indicator([close.slice(0, -5)], [20, 2]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Lower:', continued[0]);
    ```

### SIMD

=== "Rust"

    ```rust
    use tulip_rs::indicators::bbands::{BBands, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = BBands::indicator_by_assets::<4>(&inputs, &[20.0, 2.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Lower:  {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Middle: {:?}", i + 1, asset_outputs[1]);
        println!("Asset {} Upper:  {:?}", i + 1, asset_outputs[2]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::bbands::{BBands, IndicatorByOptions};

    let opts: [&[f64; 2]; 4] = [&[10.0, 1.5], &[20.0, 2.0], &[30.0, 2.0], &[50.0, 2.5]];
    let results = BBands::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {} Lower:  {:?}", i + 1, out[0]);
        println!("Option set {} Middle: {:?}", i + 1, out[1]);
        println!("Option set {} Upper:  {:?}", i + 1, out[2]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[BBANDS_INPUTS] = {a1};
    const double *asset2[BBANDS_INPUTS] = {a2};
    const double *asset3[BBANDS_INPUTS] = {a3};
    const double *asset4[BBANDS_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = bbands_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's lower_band */
        bbands_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o10[] = {10.0}, o20[] = {20.0}, o30[] = {30.0}, o50[] = {50.0};
    const double *const simd_opts[4] = {o10, o20, o30, o50};

    CSimdResult r = bbands_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i */
    for (uintptr_t i = 0; i < r.num_results; i++) bbands_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [np.array(asset1_close, dtype=np.float64)],
        [np.array(asset2_close, dtype=np.float64)],
        [np.array(asset3_close, dtype=np.float64)],
        [np.array(asset4_close, dtype=np.float64)],
    ]
    outputs_list, states = tulip_rs.indicators.bbands.simd_by_assets(simd_inputs, [20.0, 2.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1} Lower:  {asset_outputs[0]}")
        print(f"Asset {i+1} Middle: {asset_outputs[1]}")
        print(f"Asset {i+1} Upper:  {asset_outputs[2]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[10.0, 1.5], [20.0, 2.0], [30.0, 2.0], [50.0, 2.5]]
    outputs_list, states = tulip_rs.indicators.bbands.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1} Lower:  {out[0]}")
        print(f"Option set {i+1} Middle: {out[1]}")
        print(f"Option set {i+1} Upper:  {out[2]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.bbands.simdByAssets(simdInputs, [20, 2]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} Lower:`, out[0], 'Middle:', out[1], 'Upper:', out[2]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[10, 1.5], [20, 2], [30, 2], [50, 2.5]];
    const [results] = ti.bbands.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} Lower:`, out[0]));
    ```
