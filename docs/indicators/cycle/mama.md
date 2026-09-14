# MAMA — MESA Adaptive Moving Average

An adaptive moving average that adjusts its smoothing factor in proportion to the instantaneous rate of phase change; FAMA (Following Adaptive Moving Average) is the slower-responding companion line used for crossover signals.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[fast_limit, slow_limit]` &nbsp;|&nbsp; **Outputs:** `[mama, fama]` &nbsp;|&nbsp; **Optional:** `[dc_period, alpha]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::mama::{Mama, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    // Options: [fast_limit, slow_limit]
    let (outputs, _state) = Mama::indicator(&[close.as_slice()], &[0.5, 0.05], None).unwrap();
    println!("MAMA:  {:?}", outputs[0]);
    println!("FAMA:  {:?}", outputs[1]);

    // State continuation
    let n = close.len() - 5;
    let partial = close[..n].to_vec();
    let (outputs2, mut state) = Mama::indicator(&[partial.as_slice()], &[0.5, 0.05], None).unwrap();
    println!("Partial MAMA: {:?}", outputs2[0]);

    let rest = close[n..].to_vec();
    let continued = state.batch_indicator(&[rest.as_slice()], None).unwrap();
    println!("Continued MAMA: {:?}", continued[0]);
    println!("Continued FAMA: {:?}", continued[1]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    # Options: [fast_limit, slow_limit]
    outputs, state = tulip_rs.indicators.mama.indicator([close], [0.5, 0.05])
    print("MAMA: ", outputs[0])
    print("FAMA: ", outputs[1])

    # State continuation
    partial = close[:-5]
    outputs2, state = tulip_rs.indicators.mama.indicator([partial], [0.5, 0.05])
    rest = close[-5:]
    continued = state.batch_indicator([rest])
    print("Continued MAMA: ", continued[0])
    print("Continued FAMA: ", continued[1])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ]);

    const [outputs, state] = ti.mama.indicator([close], [0.5, 0.05]);
    console.log('MAMA:', outputs[0]);
    console.log('FAMA:', outputs[1]);

    // State continuation
    const [, state2] = ti.mama.indicator([close.slice(0, -5)], [0.5, 0.05]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued MAMA:', continued[0]);
    console.log('Continued FAMA:', continued[1]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ];

    const [outputs, state] = ti.mama.indicator([close], [0.5, 0.05]);
    console.log('MAMA:', outputs[0]);
    console.log('FAMA:', outputs[1]);

    // State continuation
    const [, state2] = ti.mama.indicator([close.slice(0, -5)], [0.5, 0.05]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued MAMA:', continued[0]);
    console.log('Continued FAMA:', continued[1]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[MAMA_OPTIONS] = {0.5, 0.05}; // fast_limit, slow_limit
    const double *inputs[MAMA_INPUTS] = {close};

    /* Full computation with optional outputs */
    bool optional_outputs[2] = {true, true}; // dc_period, alpha
    CIndicatorResult r = mama_indicator(inputs, 40, options, optional_outputs, 2);
    /* r.outputs[0] -> mama (primary) */
    /* r.outputs[1] -> fama (primary) */
    /* r.outputs[2] -> dc_period (optional) */
    /* r.outputs[3] -> alpha (optional) */
    tulip_ffi_result_free(r);
    mama_state_free(r.state);

    /* Partial computation without optional outputs + state continuation */
    CIndicatorResult p = mama_indicator(inputs, 35, options, NULL, 0);
    double new_close[] = {89.70, 90.10, 89.50, 90.20, 90.80};
    const double *new_inputs[MAMA_INPUTS] = {new_close};
    CBatchResult b = mama_batch(p.state, new_inputs, 5, NULL, 0);
    /* b.outputs[0] -> mama for new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    mama_state_free(p.state);
    ```

### Optional Outputs

=== "Rust"

    `mama` exposes 2 optional outputs: `dc_period`, `alpha`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::mama::{Mama, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    let mask = [true, true]; // one per optional output
    let (outputs, _state) = Mama::indicator(&[close.as_slice()], &[0.5, 0.05], Some(&mask)).unwrap();

    let mama      = &outputs[0]; // mama (primary)
    let fama      = &outputs[1]; // fama (primary)
    let dc_period = &outputs[2]; // dc_period (optional — requested)
    let alpha     = &outputs[3]; // alpha (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    outputs, state = tulip_rs.indicators.mama.indicator(
        [close], [0.5, 0.05],
        optional_outputs=[True, True],
    )

    mama      = outputs[0]  # mama (primary)
    fama      = outputs[1]  # fama (primary)
    dc_period = outputs[2]  # dc_period (optional — requested)
    alpha     = outputs[3]  # alpha (optional — requested)
    ```

=== "Node.js"

    `mama` exposes 2 optional outputs: `dc_period`, `alpha`.

    ```javascript
    const [allOut] = ti.mama.indicator([close], [0.5, 0.05], [true, true]);
    const mama     = allOut[0]; // primary
    const fama     = allOut[1]; // primary
    const dcPeriod = allOut[2]; // optional 0: dc_period
    const alpha    = allOut[3]; // optional 1: alpha
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.mama.indicator([close], [0.5, 0.05], [true, true]);
    const mama     = allOut[0]; // primary
    const fama     = allOut[1]; // primary
    const dcPeriod = allOut[2]; // optional 0: dc_period
    const alpha    = allOut[3]; // optional 1: alpha
    ```

=== "C"

    `mama` exposes 2 optional outputs: `dc_period`, `alpha`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[MAMA_OPTIONS] = {0.5, 0.05}; // fast_limit, slow_limit
    const double *inputs[MAMA_INPUTS] = {close};

    bool mask[2] = {true, true}; // one per optional output
    CIndicatorResult r = mama_indicator(inputs, 40, options, mask, 2);

    /* r.outputs[0] -> mama (primary) */
    /* r.outputs[1] -> fama (primary) */
    /* r.outputs[2] -> dc_period (optional — requested) */
    /* r.outputs[3] -> alpha (optional — requested) */

    tulip_ffi_result_free(r);
    mama_state_free(r.state);
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::mama::{Mama, Indicator};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36_f64];
    let a3 = vec![78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36_f64];
    let a4 = vec![83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = Mama::indicator_by_assets::<4>(&inputs, &[0.5, 0.05], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} MAMA: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} FAMA: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::mama::{Mama, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 2]; 4] = [
        &[0.3, 0.03],
        &[0.4, 0.04],
        &[0.5, 0.05],
        &[0.6, 0.06],
    ];

    let results = Mama::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Option set {} MAMA: {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} FAMA: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 3.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.mama.simd_by_assets(simd_inputs, [0.5, 0.05])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} MAMA: {out[0]}")
        print(f"Asset {i + 1} FAMA: {out[1]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_options = [
        [0.3, 0.03],
        [0.4, 0.04],
        [0.5, 0.05],
        [0.6, 0.06],
    ]
    outputs_list, states = tulip_rs.indicators.mama.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} MAMA: {out[0]}")
        print(f"Option set {i + 1} FAMA: {out[1]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v + 5.0)],
        [close.map(v => v - 3.0)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.mama.simdByAssets(simdInputs, [0.5, 0.05]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} MAMA:`, out[0], 'FAMA:', out[1]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[0.3, 0.03], [0.4, 0.04], [0.5, 0.05], [0.6, 0.06]];
    const [results] = ti.mama.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} MAMA:`, out[0]));
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[MAMA_INPUTS] = {a1};
    const double *asset2[MAMA_INPUTS] = {a2};
    const double *asset3[MAMA_INPUTS] = {a3};
    const double *asset4[MAMA_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    bool optional_outputs[2] = {true, true}; // dc_period, alpha
    CSimdResult r = mama_simd_by_assets(simd_inputs, 4, 10, options, optional_outputs, 2);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's mama series, length r.output_lens[i][0] */
        /* r.outputs[i][1] -> asset i's fama */
        mama_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in one call:

    ```c
    double o03[] = {0.3}, o003[] = {0.03};
    double o04[] = {0.4}, o004[] = {0.04};
    double o05[] = {0.5}, o005[] = {0.05};
    double o06[] = {0.6}, o006[] = {0.06};

    const double *const simd_opts[4] = {
        (double[]){0.3, 0.03},
        (double[]){0.4, 0.04},
        (double[]){0.5, 0.05},
        (double[]){0.6, 0.06}
    };

    bool optional_outputs[2] = {true, true};
    CSimdResult r = mama_simd_by_options(inputs, 10, simd_opts, 4, optional_outputs, 2);
    /* r.outputs[i][0] -> asset i's mama for option set */
    for (uintptr_t i = 0; i < r.num_results; i++) mama_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```
