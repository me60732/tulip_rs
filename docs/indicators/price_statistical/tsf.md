# TSF — Time Series Forecast — `tsf`

Projects the linear regression line one bar forward, giving a one-period-ahead price forecast.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[tsf]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::tsf::{Tsf, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let (outputs, mut state) = Tsf::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Tsf::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[TSF_OPTIONS] = {14.0}; // period
    const double *inputs[TSF_INPUTS] = {close};

    /* Full computation */
    CIndicatorResult r = tsf_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> TSF(14) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    tsf_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = tsf_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.53};
    const double *new_inputs[TSF_INPUTS] = {new_close};
    CBatchResult b = tsf_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> TSF for the new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    tsf_state_free(p.state);
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.tsf.indicator([close], [14.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.tsf.indicator([close], [14]);
    console.log('TSF(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.tsf.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TSF:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.tsf.indicator([close], [14]);
    console.log('TSF(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.tsf.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TSF:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `tsf` exposes 3 optional outputs: `linreg`, `linregslope`, `linregintercept`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::tsf::{Tsf, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true, false]; // one per optional output
    let (outputs, _state) = Tsf::indicator(&[close.as_slice()], &[14.0], Some(&mask)).unwrap();

    let tsf         = &outputs[0]; // tsf (primary)
    let linreg      = &outputs[1]; // linreg (optional — requested)
    let linregslope = &outputs[2]; // linregslope (optional — requested)
    // linregintercept not requested
    ```

=== "C"

    `tsf` exposes 3 optional outputs: `linreg`, `linregslope`, `linregintercept`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[TSF_OPTIONS] = {14.0}; // period
    const double *inputs[TSF_INPUTS] = {close};
    bool optional_outputs[3] = {true, true, false}; // linreg, linregslope, linregintercept

    CIndicatorResult r = tsf_indicator(inputs, 10, options, optional_outputs, 3);
    /* r.outputs[0] -> tsf (primary) */
    /* r.outputs[1] -> linreg (optional — requested) */
    /* r.outputs[2] -> linregslope (optional — requested) */
    tulip_ffi_result_free(r);
    tsf_state_free(r.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.tsf.indicator(
        [close], [14.0],
        optional_outputs=[True, True, False],
    )

    tsf         = outputs[0]  # tsf (primary)
    linreg      = outputs[1]  # linreg (optional — requested)
    linregslope = outputs[2]  # linregslope (optional — requested)
    # linregintercept not requested
    ```

=== "Node.js"

    `tsf` exposes 3 optional outputs: `linreg`, `linregslope`, `linregintercept`.

    ```javascript
    const [allOut] = ti.tsf.indicator([close], [14], [true, true, true]);
    const tsf             = allOut[0]; // primary
    const linreg          = allOut[1]; // optional 0: linreg
    const linregslope     = allOut[2]; // optional 1: linregslope
    const linregintercept = allOut[3]; // optional 2: linregintercept
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.tsf.indicator([close], [14], [true, true, true]);
    const tsf             = allOut[0]; // primary
    const linreg          = allOut[1]; // optional 0: linreg
    const linregslope     = allOut[2]; // optional 1: linregslope
    const linregintercept = allOut[3]; // optional 2: linregintercept
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::tsf::{Tsf, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = Tsf::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::tsf::{Tsf, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = Tsf::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Option {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    const double *asset1[TSF_INPUTS] = {a1};
    const double *asset2[TSF_INPUTS] = {a2};
    const double *asset3[TSF_INPUTS] = {a3};
    const double *asset4[TSF_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    double options[TSF_OPTIONS] = {14.0};

    CSimdResult r = tsf_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's TSF series */
        tsf_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = tsf_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> period set i results */
        tsf_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.tsf.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.tsf.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.tsf.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.tsf.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
