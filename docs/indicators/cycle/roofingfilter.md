# Roofing Filter — Ehlers Roofing Filter

Band-pass filters price by first applying a high-pass filter to remove trend and then a super-smoother to remove high-frequency noise; the result isolates the dominant cycle band.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[ss_period, hp_period]` &nbsp;|&nbsp; **Outputs:** `[roofing]` &nbsp;|&nbsp; **Optional:** `[highpass]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::roofingfilter::{RoofingFilter, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    // Options: [ss_period, hp_period]
    let (outputs, _state) = RoofingFilter::indicator(&[close.as_slice()], &[10.0, 40.0], None).unwrap();
    println!("Roofing Filter: {:?}", outputs[0]);

    // State continuation
    let n = close.len() - 5;
    let partial = close[..n].to_vec();
    let (outputs2, mut state) = RoofingFilter::indicator(&[partial.as_slice()], &[10.0, 40.0], None).unwrap();
    println!("Partial Roofing Filter: {:?}", outputs2[0]);

    let rest = close[n..].to_vec();
    let continued = state.batch_indicator(&[rest.as_slice()], None).unwrap();
    println!("Continued Roofing Filter: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20
    };
    double options[ROOFINGFILTER_OPTIONS] = {10.0, 40.0}; // ss_period, hp_period
    const double *inputs[ROOFINGFILTER_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = roofingfilter_indicator(inputs, 40, options, NULL, 0);
    /* r.outputs[0] -> the roofing filter series, length r.output_lens[0];
       r.outputs[1] -> highpass (optional — not requested here) */
    tulip_ffi_result_free(r);
    roofingfilter_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = roofingfilter_indicator(inputs, 35, options, NULL, 0);
    double new_close[] = {92.80, 93.10, 92.50, 93.20};
    const double *new_inputs[ROOFINGFILTER_INPUTS] = {new_close};
    CBatchResult b = roofingfilter_batch(p.state, new_inputs, 4, NULL, 0);
    /* b.outputs[0] -> roofing filter values for just the four new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    roofingfilter_state_free(p.state);
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

    # Options: [ss_period, hp_period]
    outputs, state = tulip_rs.indicators.roofingfilter.indicator([close], [10.0, 40.0])
    print("Roofing Filter:", outputs[0])

    # State continuation
    partial = close[:-5]
    outputs2, state = tulip_rs.indicators.roofingfilter.indicator([partial], [10.0, 40.0])
    rest = close[-5:]
    continued = state.batch_indicator([rest])
    print("Continued Roofing Filter:", continued[0])
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

    const [outputs, state] = ti.roofingfilter.indicator([close], [10, 40]);
    console.log('Roofing Filter:', outputs[0]);

    // State continuation
    const [, state2] = ti.roofingfilter.indicator([close.slice(0, -5)], [10, 40]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Roofing Filter:', continued[0]);
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

    const [outputs, state] = ti.roofingfilter.indicator([close], [10, 40]);
    console.log('Roofing Filter:', outputs[0]);

    // State continuation
    const [, state2] = ti.roofingfilter.indicator([close.slice(0, -5)], [10, 40]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Roofing Filter:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `roofingfilter` exposes 1 optional output: `highpass`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::roofingfilter::{RoofingFilter, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    let mask = [true]; // one per optional output
    let (outputs, _state) = RoofingFilter::indicator(&[close.as_slice()], &[10.0, 40.0], Some(&mask)).unwrap();

    let roofing  = &outputs[0]; // roofing (primary)
    let highpass = &outputs[1]; // highpass (optional — requested)
    ```

=== "C"

    `roofingfilter` exposes 1 optional output: `highpass`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20
    };
    double options[ROOFINGFILTER_OPTIONS] = {10.0, 40.0}; // ss_period, hp_period
    const double *inputs[ROOFINGFILTER_INPUTS] = {close};
    bool optional_outputs[1] = {true}; // highpass=true

    CIndicatorResult r = roofingfilter_indicator(inputs, 40, options, optional_outputs, 1);
    /* r.outputs[0] -> roofing (primary)
       r.outputs[1] -> highpass (optional — requested) */
    tulip_ffi_result_free(r);
    roofingfilter_state_free(r.state);
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

    outputs, state = tulip_rs.indicators.roofingfilter.indicator(
        [close], [10.0, 40.0],
        optional_outputs=[True],
    )

    roofing  = outputs[0]  # roofing (primary)
    highpass = outputs[1]  # highpass (optional — requested)
    ```

=== "Node.js"

    `roofingfilter` exposes 1 optional output: `highpass`.

    ```javascript
    const [allOut] = ti.roofingfilter.indicator([close], [10, 40], [true]);
    const roofing  = allOut[0]; // primary
    const highpass = allOut[1]; // optional 0: highpass
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.roofingfilter.indicator([close], [10, 40], [true]);
    const roofing  = allOut[0]; // primary
    const highpass = allOut[1]; // optional 0: highpass
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::roofingfilter::{RoofingFilter, Indicator, TIndicatorState};

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

    let results = RoofingFilter::indicator_by_assets::<4>(&inputs, &[10.0, 40.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::roofingfilter::{RoofingFilter, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 2]; 4] = [
        &[5.0,  20.0],
        &[10.0, 40.0],
        &[14.0, 50.0],
        &[20.0, 60.0],
    ];

    let results = RoofingFilter::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to N assets in parallel (N must be 2/4/8/16):

    ```c
    #include "tulip_rs_ffi.h"

    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    const double *asset1[ROOFINGFILTER_INPUTS] = {a1};
    const double *asset2[ROOFINGFILTER_INPUTS] = {a2};
    const double *asset3[ROOFINGFILTER_INPUTS] = {a3};
    const double *asset4[ROOFINGFILTER_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    double options[ROOFINGFILTER_OPTIONS] = {10.0, 40.0};

    CSimdResult r = roofingfilter_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        roofingfilter_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, N different option sets in parallel:

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[ROOFINGFILTER_INPUTS] = {close};

    double o5_20[] = {5.0, 20.0}, o10_40[] = {10.0, 40.0}, o14_50[] = {14.0, 50.0}, o20_60[] = {20.0, 60.0};
    const double *const simd_opts[4] = {o5_20, o10_40, o14_50, o20_60};

    CSimdResult r = roofingfilter_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) roofingfilter_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
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
    outputs_list, states = tulip_rs.indicators.roofingfilter.simd_by_assets(simd_inputs, [10.0, 40.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
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
        [5.0,  20.0],
        [10.0, 40.0],
        [14.0, 50.0],
        [20.0, 60.0],
    ]
    outputs_list, states = tulip_rs.indicators.roofingfilter.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1}: {out[0]}")
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
    const [results] = ti.roofingfilter.simdByAssets(simdInputs, [10, 40]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[5, 20], [10, 40], [14, 50], [20, 60]];
    const [results] = ti.roofingfilter.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```
