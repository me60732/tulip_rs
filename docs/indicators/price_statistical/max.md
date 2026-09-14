# Max — Highest Value Over Period — `max`

The highest value in the input series over a rolling `period` window.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[max]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::max::{Max, Indicator, TIndicatorState};

    let close = [81.59_f64, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36];
    let (outputs, mut state) = Max::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Max::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[MAX_INPUTS] = {close};
    double options[MAX_OPTIONS] = {14.0}; // period

    /* Full computation */
    CIndicatorResult r = max_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the MAX(14) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    max_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = max_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.53};
    const double *new_inputs[MAX_INPUTS] = {new_close};
    CBatchResult b = max_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> MAX values for just the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    max_state_free(p.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.max.indicator([close], [14.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.max.indicator([close], [14]);
    console.log('Max(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.max.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Max:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.max.indicator([close], [14]);
    console.log('Max(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.max.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Max:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::max::{Max};

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = Max::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {81.59*1.2, 81.06*1.2, 82.87*1.2, 83.00*1.2, 83.61*1.2, 83.15*1.2, 82.84*1.2, 83.99*1.2, 84.55*1.2, 84.36*1.2};
    double a3[] = {90.0+0.5*0+81.59*0.1, 90.0+0.5*1+81.06*0.1, 90.0+0.5*2+82.87*0.1, 90.0+0.5*3+83.00*0.1, 90.0+0.5*4+83.61*0.1, 90.0+0.5*5+83.15*0.1, 90.0+0.5*6+82.84*0.1, 90.0+0.5*7+83.99*0.1, 90.0+0.5*8+84.55*0.1, 90.0+0.5*9+84.36*0.1};
    double a4[] = {100.0-0.3*0+81.59*0.05, 100.0-0.3*1+81.06*0.05, 100.0-0.3*2+82.87*0.05, 100.0-0.3*3+83.00*0.05, 100.0-0.3*4+83.61*0.05, 100.0-0.3*5+83.15*0.05, 100.0-0.3*6+82.84*0.05, 100.0-0.3*7+83.99*0.05, 100.0-0.3*8+84.55*0.05, 100.0-0.3*9+84.36*0.05};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[MAX_INPUTS] = {a1};
    const double *asset2[MAX_INPUTS] = {a2};
    const double *asset3[MAX_INPUTS] = {a3};
    const double *asset4[MAX_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = max_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        max_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double o5[] = {5.0}, o10[] = {10.0}, o20[] = {20.0}, o50[] = {50.0};
    const double *const simd_opts[4] = {o5, o10, o20, o50};

    CSimdResult r = max_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i (periods 5/10/20/50) */
    for (uintptr_t i = 0; i < r.num_results; i++) max_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.max.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [20.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.max.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.max.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [20], [50]];
    const [results] = ti.max.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
