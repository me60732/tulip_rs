# DI — Directional Indicator

Smoothed directional movement expressed as a percentage of ATR. +DI and -DI crossovers are used as trade signals.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[plus_di, minus_di]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::di::{Di, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = Di::indicator(&inputs, &[14.0], None).unwrap();
    println!("+DI: {:?}", outputs[0]);
    println!('-DI: {:?}', outputs[1]);

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = Di::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[14.0], None).unwrap();
    println!("+DI: {:?}", outputs2[0]);
    println!('-DI: {:?}', outputs2[1]);

    let new_high  = vec![85.90_f64];
    let new_low   = vec![84.03_f64];
    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("+DI continued: {:?}", continued[0]);
    println!('-DI continued: {:?}', continued[1]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[DI_OPTIONS] = {14.0}; // period
    const double *inputs[DI_INPUTS] = {high, low, close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = di_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> plus_di, length r.output_lens[0] */
    /* r.outputs[1] -> minus_di, length r.output_lens[1] */
    tulip_ffi_result_free(r);
    di_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = di_indicator(inputs, 8, options, NULL, 0);
    double new_high[]  = {85.90};
    double new_low[]   = {84.03};
    double new_close[] = {85.53};
    const double *new_inputs[DI_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = di_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> plus_di for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    di_state_free(p.state);
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

    outputs, state = tulip_rs.indicators.di.indicator([high, low, close], [14.0])
    print(outputs[0])  # Plus DI
    print(outputs[1])  # Minus DI

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])  # Plus DI continued
    print(continued[1])  # Minus DI continued
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.di.indicator([high, low, close], [14]);
    console.log('+DI:', outputs[0]);
    console.log('-DI:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.di.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued +DI:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.di.indicator([high, low, close], [14]);
    console.log('+DI:', outputs[0]);
    console.log('-DI:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.di.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued +DI:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `di` exposes 2 optional outputs: `atr`, `tr`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::di::{Di, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low   = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();

    let mask = [true, true];
    let (outputs, _state) = Di::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let plus_di  = &outputs[0]; // plus_di (primary)
    let minus_di = &outputs[1]; // minus_di (primary)
    let atr      = &outputs[2]; // atr (optional — requested)
    let tr       = &outputs[3]; // tr (optional — requested)
    ```

=== "C"

    `di` exposes 2 optional outputs: `atr`, `tr`. Pass a boolean mask — one `bool` per optional output, in header order.

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[DI_OPTIONS] = {14.0}; // period
    const double *inputs[DI_INPUTS] = {high, low, close};
    bool optional_outputs[2] = {true, true}; // atr, tr

    CIndicatorResult r = di_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> plus_di (primary), length r.output_lens[0] */
    /* r.outputs[1] -> minus_di (primary), length r.output_lens[1] */
    /* r.outputs[2] -> atr (optional),     length r.output_lens[2] */
    /* r.outputs[3] -> tr (optional),      length r.output_lens[3] */
    tulip_ffi_result_free(r);
    di_state_free(r.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high  = close + 1.0
    low   = close - 1.0

    outputs, state = tulip_rs.indicators.di.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, True],
    )

    plus_di  = outputs[0]  # plus_di (primary)
    minus_di = outputs[1]  # minus_di (primary)
    atr      = outputs[2]  # atr (optional — requested)
    tr       = outputs[3]  # tr (optional — requested)
    ```

=== "Node.js"

    `di` exposes 2 optional outputs: `atr`, `tr`.

    ```javascript
    const [allOut] = ti.di.indicator([high, low, close], [14], [true, true]);
    const plusDI  = allOut[0]; // primary: +di
    const minusDI = allOut[1]; // primary: -di
    const atr     = allOut[2]; // optional 0: atr
    const tr      = allOut[3]; // optional 1: tr
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.di.indicator([high, low, close], [14], [true, true]);
    const plusDI  = allOut[0]; // primary: +di
    const minusDI = allOut[1]; // primary: -di
    const atr     = allOut[2]; // optional 0: atr
    const tr      = allOut[3]; // optional 1: tr
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::di::{Di, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = Di::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} +DI: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} -DI: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::di::{Di, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = Di::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {} +DI: {:?}", opts[i][0], out[0]);
        println!("Period {} -DI: {:?}", opts[i][0], out[1]);
    }
    ```

=== "C"

    **By assets** — same option applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double h2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double l2[] = {71.10, 71.85, 72.40, 72.00, 73.20, 73.85, 74.10, 74.60, 75.00, 75.50};
    double c2[] = {71.59, 71.06, 72.87, 73.00, 73.61, 73.15, 72.84, 73.99, 74.55, 74.36};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[DI_INPUTS] = {h1, l1, c1};
    const double *asset2[DI_INPUTS] = {h2, l2, c2};
    const double *const *const simd_inputs[4] = {asset1, asset2, NULL, NULL}; /* N=4 lanes */
    double options[DI_OPTIONS] = {14.0};
    bool optional_outputs[2] = {true, true};

    CSimdResult r = di_simd_by_assets(simd_inputs, 4, 10, options, optional_outputs, 2);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's plus_di, length r.output_lens[i][0] */
        /* r.outputs[i][1] -> asset i's minus_di, length r.output_lens[i][1] */
        di_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, N different periods in one call:

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[DI_INPUTS] = {high, low, close};

    /* Tile the series 20x so longer-period option sets have enough data */
    #define EXPANDED_LEN (10 * 20)
    static double high_expanded[EXPANDED_LEN];
    static double low_expanded[EXPANDED_LEN];
    static double close_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 20; i++) {
        for (size_t j = 0; j < 10; j++) {
            high_expanded[i * 10 + j]  = high[j];
            low_expanded[i * 10 + j]   = low[j];
            close_expanded[i * 10 + j] = close[j];
        }
    }
    const double *expanded_inputs[DI_INPUTS] = {high_expanded, low_expanded, close_expanded};

    static const double o7[DI_OPTIONS]   = {7.0};
    static const double o14[DI_OPTIONS]  = {14.0};
    static const double o21[DI_OPTIONS]  = {21.0};
    static const double o28[DI_OPTIONS]  = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = di_simd_by_options(expanded_inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) di_state_free(r.states[i]);
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
    outputs_list, states = tulip_rs.indicators.di.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1} +DI: {asset_outputs[0]}")
        print(f"Asset {i+1} -DI: {asset_outputs[1]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.di.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]} +DI: {out[0]}")
        print(f"Period {simd_options[i][0]} -DI: {out[1]}")
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
    const [results] = ti.di.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} +DI:`, out[0], '-DI:', out[1]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.di.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]} +DI:`, out[0], '-DI:', out[1]));
    ```
