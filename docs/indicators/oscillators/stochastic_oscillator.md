# Stochastic Oscillator

Compares a security's closing price to its price range over a given period. %K is the raw stochastic value; %D is a smoothed moving average of %K.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[k_period, k_slowing_period, d_period]` &nbsp;|&nbsp; **Outputs:** `[stoch_k, stoch_d]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::stoch::{Stoch, TIndicatorState, Indicator};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    // Options: [k_period, k_slowing_period, d_period]
    let (outputs, _state) = Stoch::indicator(&inputs, &[14.0, 3.0, 3.0], None).unwrap();
    println!("Stoch %K: {:?}", outputs[0]);
    println!("Stoch %D: {:?}", outputs[1]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = Stoch::indicator(&inputs2, &[14.0, 3.0, 3.0], None).unwrap();
    println!("Partial %K: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued %K: {:?}", continued[0]);
    println!("Continued %D: {:?}", continued[1]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[STOCH_OPTIONS] = {14.0, 3.0, 3.0}; // k_period, k_slowing_period, d_period
    const double *inputs[STOCH_INPUTS] = {high, low, close};

    /* Full computation */
    CIndicatorResult r = stoch_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> slowk (stoch_k), length r.output_lens[0] */
    /* r.outputs[1] -> slowd (stoch_d), length r.output_lens[1] */
    tulip_ffi_result_free(r);
    stoch_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = stoch_indicator(inputs, 8, options, NULL, 0);
    double new_high[]  = {84.55, 85.00};
    double new_low[]   = {84.15, 84.11};
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[STOCH_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = stoch_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> slowk for just the two new bars */
    /* b.outputs[1] -> slowd for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    stoch_state_free(p.state);
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

    # Options: [k_period, k_slowing_period, d_period]
    outputs, state = tulip_rs.indicators.stoch.indicator([high, low, close], [14.0, 3.0, 3.0])
    print("Stoch %K:", outputs[0])
    print("Stoch %D:", outputs[1])

    # State continuation
    outputs2, state = tulip_rs.indicators.stoch.indicator([high[:8], low[:8], close[:8]], [14.0, 3.0, 3.0])
    continued = state.batch_indicator([high[8:], low[8:], close[8:]])
    print("Continued %K:", continued[0])
    print("Continued %D:", continued[1])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.stoch.indicator([high, low, close], [5, 3, 3]);
    console.log('%K:', outputs[0]);
    console.log('%D:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.stoch.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [5, 3, 3]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued %K:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.stoch.indicator([high, low, close], [5, 3, 3]);
    console.log('%K:', outputs[0]);
    console.log('%D:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.stoch.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [5, 3, 3]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued %K:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::stoch::{Stoch, Indicator};

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    // Reuse the same data for assets 2–4 in this example
    let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = Stoch::indicator_by_assets::<4>(&inputs, &[14.0, 3.0, 3.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} %K: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} %D: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::stoch::{Stoch, IndicatorByOptions};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[5.0,  3.0, 3.0],
        &[9.0,  3.0, 3.0],
        &[14.0, 3.0, 3.0],
        &[21.0, 3.0, 3.0],
    ];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = Stoch::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Option set {} %K: {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} %D: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel:

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double h2[] = {h1[0]*1.1, h1[1]*1.1, h1[2]*1.1, h1[3]*1.1, h1[4]*1.1,
                   h1[5]*1.1, h1[6]*1.1, h1[7]*1.1, h1[8]*1.1, h1[9]*1.1};
    double l2[] = {l1[0]*1.1, l1[1]*1.1, l1[2]*1.1, l1[3]*1.1, l1[4]*1.1,
                   l1[5]*1.1, l1[6]*1.1, l1[7]*1.1, l1[8]*1.1, l1[9]*1.1};
    double c2[] = {c1[0]*1.1, c1[1]*1.1, c1[2]*1.1, c1[3]*1.1, c1[4]*1.1,
                   c1[5]*1.1, c1[6]*1.1, c1[7]*1.1, c1[8]*1.1, c1[9]*1.1};
    double h3[] = {h1[0]*0.9, h1[1]*0.9, h1[2]*0.9, h1[3]*0.9, h1[4]*0.9,
                   h1[5]*0.9, h1[6]*0.9, h1[7]*0.9, h1[8]*0.9, h1[9]*0.9};
    double l3[] = {l1[0]*0.9, l1[1]*0.9, l1[2]*0.9, l1[3]*0.9, l1[4]*0.9,
                   l1[5]*0.9, l1[6]*0.9, l1[7]*0.9, l1[8]*0.9, l1[9]*0.9};
    double c3[] = {c1[0]*0.9, c1[1]*0.9, c1[2]*0.9, c1[3]*0.9, c1[4]*0.9,
                   c1[5]*0.9, c1[6]*0.9, c1[7]*0.9, c1[8]*0.9, c1[9]*0.9};
    double h4[] = {h1[0]*1.02, h1[1]*1.02, h1[2]*1.02, h1[3]*1.02, h1[4]*1.02,
                   h1[5]*1.02, h1[6]*1.02, h1[7]*1.02, h1[8]*1.02, h1[9]*1.02};
    double l4[] = {l1[0]*1.02, l1[1]*1.02, l1[2]*1.02, l1[3]*1.02, l1[4]*1.02,
                   l1[5]*1.02, l1[6]*1.02, l1[7]*1.02, l1[8]*1.02, l1[9]*1.02};
    double c4[] = {c1[0]*1.02, c1[1]*1.02, c1[2]*1.02, c1[3]*1.02, c1[4]*1.02,
                   c1[5]*1.02, c1[6]*1.02, c1[7]*1.02, c1[8]*1.02, c1[9]*1.02};

    const double *const asset1[STOCH_INPUTS] = {h1, l1, c1};
    const double *const asset2[STOCH_INPUTS] = {h2, l2, c2};
    const double *const asset3[STOCH_INPUTS] = {h3, l3, c3};
    const double *const asset4[STOCH_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = stoch_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's slowk, length r.output_lens[i][0] */
        /* r.outputs[i][1] -> asset i's slowd, length r.output_lens[i][1] */
        stoch_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double o5[] = {5.0, 3.0, 3.0};
    double o9[] = {9.0, 3.0, 3.0};
    double o14[] = {14.0, 3.0, 3.0};
    double o21[] = {21.0, 3.0, 3.0};

    const double *const simd_opts[4] = {o5, o9, o14, o21};

    CSimdResult r = stoch_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset's slowk for option set i */
        /* r.outputs[i][1] -> asset's slowd for option set i */
        stoch_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    h1, l1, c1 = high,        low,        close
    h2, l2, c2 = high + 0.5,  low + 0.5,  close + 0.5
    h3, l3, c3 = high - 0.5,  low - 0.5,  close - 0.5
    h4, l4, c4 = high * 1.01, low * 1.01, close * 1.01

    simd_inputs = [[h1, l1, c1], [h2, l2, c2], [h3, l3, c3], [h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.stoch.simd_by_assets(simd_inputs, [14.0, 3.0, 3.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} %K: {out[0]}")
        print(f"Asset {i + 1} %D: {out[1]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [5.0,  3.0, 3.0],
        [9.0,  3.0, 3.0],
        [14.0, 3.0, 3.0],
        [21.0, 3.0, 3.0],
    ]
    outputs_list, states = tulip_rs.indicators.stoch.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} %K: {out[0]}")
        print(f"Option set {i + 1} %D: {out[1]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.stoch.simdByAssets(simdInputs, [5, 3, 3]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} %K:`, out[0], '%D:', out[1]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[5, 3, 3], [9, 3, 3], [14, 3, 3], [21, 3, 3]];
    const [results] = ti.stoch.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} %K:`, out[0], '%D:', out[1]));
    ```
