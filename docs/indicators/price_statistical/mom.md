# MOM — Momentum — `mom`

The difference between the current price and the price `period` bars ago: `close[i] - close[i - period]`.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[mom]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::mom::{Mom};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let (outputs, mut state) = Mom::indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Mom::indicator(&[partial.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[MOM_OPTIONS] = {5.0}; // period
    const double *inputs[MOM_INPUTS] = {close};

    /* Full computation */
    CIndicatorResult r = mom_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> MOM(5) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    mom_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = mom_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.53};
    const double *new_inputs[MOM_INPUTS] = {new_close};
    CBatchResult b = mom_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> MOM values for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    mom_state_free(p.state);
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.mom.indicator([close], [10.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.mom.indicator([close], [10]);
    console.log('MOM(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.mom.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued MOM:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.mom.indicator([close], [10]);
    console.log('MOM(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.mom.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued MOM:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::mom::{Mom, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = Mom::indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::mom::{Mom, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[20.0], &[50.0]];
    let results = Mom::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
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

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[MOM_INPUTS] = {a1};
    const double *asset2[MOM_INPUTS] = {a2};
    const double *asset3[MOM_INPUTS] = {a3};
    const double *asset4[MOM_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = mom_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        mom_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o5[] = {5.0}, o10[] = {10.0}, o20[] = {20.0}, o50[] = {50.0};
    const double *const simd_opts[4] = {o5, o10, o20, o50};

    CSimdResult r = mom_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i (periods 5/10/20/50) */
    for (uintptr_t i = 0; i < r.num_results; i++) mom_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.mom.simd_by_assets(simd_inputs, [10.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [20.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.mom.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.mom.simdByAssets(simdInputs, [10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [20], [50]];
    const [results] = ti.mom.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
