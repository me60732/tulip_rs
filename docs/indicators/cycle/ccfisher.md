# CC Fisher — Cyber Cycle Fisher

Applies the Fisher Transform to the normalised Cyber Cycle oscillator, converting cycle measurements into a near-Gaussian probability distribution; `signal` is the one-bar-lagged fisher value.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[alpha]` &nbsp;|&nbsp; **Outputs:** `[fisher, signal]` &nbsp;|&nbsp; **Optional:** `[trendmode, cycle, peak]`

!!! note "Minimum bars"
    This indicator requires approximately 56 bars of input before producing meaningful output. Use at least 80 bars for reliable results. Pass `alpha = 0.0` to let the indicator choose the smoothing factor automatically.

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::ccfisher::{Ccfisher, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
        93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
        98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
        103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
        108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00_f64,
    ];

    // alpha = 0.0 to use automatic smoothing
    let (outputs, _state) = Ccfisher::indicator(&[close.as_slice()], &[0.0], None).unwrap();
    println!("Fisher: {:?}", outputs[0]);
    println!("Signal: {:?}", outputs[1]);

    // State continuation
    let n = close.len() - 5;
    let partial = close[..n].to_vec();
    let (outputs2, mut state) = Ccfisher::indicator(&[partial.as_slice()], &[0.0], None).unwrap();
    println!("Partial Fisher: {:?}", outputs2[0]);

    let rest = close[n..].to_vec();
    let continued = state.batch_indicator(&[rest.as_slice()], None).unwrap();
    println!("Continued Fisher:  {:?}", continued[0]);
    println!("Continued Signal:  {:?}", continued[1]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
                      93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
                      98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
                      103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
                      108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00};
    double options[CCFISHER_OPTIONS] = {0.0}; // alpha
    const double *inputs[CCFISHER_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = ccfisher_indicator(inputs, 70, options, NULL, 0);
    /* r.outputs[0] -> fisher, r.outputs[1] -> signal */
    tulip_ffi_result_free(r);
    ccfisher_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = ccfisher_indicator(inputs, 65, options, NULL, 0);
    double new_close[] = {92.80, 93.10, 92.50, 93.20, 93.50};
    const double *new_inputs[CCFISHER_INPUTS] = {new_close};
    CBatchResult b = ccfisher_batch(p.state, new_inputs, 5, NULL, 0);
    /* b.outputs[0] -> fisher for the 5 new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    ccfisher_state_free(p.state);
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
        93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
        98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
        103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
        108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00,
    ], dtype=np.float64)

    # alpha = 0.0 to use automatic smoothing
    outputs, state = tulip_rs.indicators.ccfisher.indicator([close], [0.0])
    print("Fisher:", outputs[0])
    print("Signal:", outputs[1])

    # State continuation
    partial = close[:-5]
    outputs2, state = tulip_rs.indicators.ccfisher.indicator([partial], [0.0])
    rest = close[-5:]
    continued = state.batch_indicator([rest])
    print("Continued Fisher: ", continued[0])
    print("Continued Signal: ", continued[1])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
        93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
        98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
        103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
        108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00,
    ]);

    // alpha = 0.0 to use automatic smoothing
    const [outputs, state] = ti.ccfisher.indicator([close], [0]);
    console.log('Fisher:', outputs[0]);
    console.log('Signal:', outputs[1]);

    // State continuation
    const [, state2] = ti.ccfisher.indicator([close.slice(0, -5)], [0]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Fisher:', continued[0]);
    console.log('Continued Signal:', continued[1]);
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
        93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
        98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
        103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
        108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00,
    ];

    // alpha = 0.0 to use automatic smoothing
    const [outputs, state] = ti.ccfisher.indicator([close], [0]);
    console.log('Fisher:', outputs[0]);
    console.log('Signal:', outputs[1]);

    // State continuation
    const [, state2] = ti.ccfisher.indicator([close.slice(0, -5)], [0]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Fisher:', continued[0]);
    console.log('Continued Signal:', continued[1]);
    ```

### Optional Outputs

=== "Rust"

    `ccfisher` exposes 3 optional outputs: `trendmode`, `cycle`, `peak`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::ccfisher::{Ccfisher, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
        93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
        98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
        103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
        108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00_f64,
    ];

    let mask = [true, true, true]; // one per optional output
    let (outputs, _state) = Ccfisher::indicator(&[close.as_slice()], &[0.0], Some(&mask)).unwrap();

    let fisher    = &outputs[0]; // fisher (primary)
    let signal    = &outputs[1]; // signal (primary)
    let trendmode = &outputs[2]; // trendmode (optional — requested)
    let cycle     = &outputs[3]; // cycle (optional — requested)
    let peak      = &outputs[4]; // peak (optional — requested)
    ```

=== "C"

    `ccfisher` exposes 3 optional outputs: `trendmode`, `cycle`, `peak`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
                      93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
                      98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
                      103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
                      108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00};
    double options[CCFISHER_OPTIONS] = {0.0}; // alpha
    const double *inputs[CCFISHER_INPUTS] = {close};
    bool optional_outputs[3] = {true, true, true}; // trendmode, cycle, peak

    CIndicatorResult r = ccfisher_indicator(inputs, 70, options, optional_outputs, 3);
    /* r.outputs[0] -> fisher (primary) */
    /* r.outputs[1] -> signal (primary) */
    /* r.outputs[2] -> trendmode (optional — requested) */
    /* r.outputs[3] -> cycle (optional — requested) */
    /* r.outputs[4] -> peak (optional — requested) */
    tulip_ffi_result_free(r);
    ccfisher_state_free(r.state);
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
        93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
        98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
        103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
        108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00,
    ], dtype=np.float64)

    outputs, state = tulip_rs.indicators.ccfisher.indicator(
        [close], [0.0],
        optional_outputs=[True, True, True],
    )

    fisher    = outputs[0]  # fisher (primary)
    signal    = outputs[1]  # signal (primary)
    trendmode = outputs[2]  # trendmode (optional — requested)
    cycle     = outputs[3]  # cycle (optional — requested)
    peak      = outputs[4]  # peak (optional — requested)
    ```

=== "Node.js"

    `ccfisher` exposes 3 optional outputs: `trendmode`, `cycle`, `peak`.

    ```javascript
    const [allOut] = ti.ccfisher.indicator([close], [0], [true, true, true]);
    const fisher    = allOut[0]; // primary
    const signal    = allOut[1]; // primary
    const trendmode = allOut[2]; // optional 0: trendmode
    const cycle     = allOut[3]; // optional 1: cycle
    const peak      = allOut[4]; // optional 2: peak
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.ccfisher.indicator([close], [0], [true, true, true]);
    const fisher    = allOut[0]; // primary
    const signal    = allOut[1]; // primary
    const trendmode = allOut[2]; // optional 0: trendmode
    const cycle     = allOut[3]; // optional 1: cycle
    const peak      = allOut[4]; // optional 2: peak
    ```

### SIMD

=== "Rust"

    **By assets** — same alpha applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ccfisher::{Ccfisher, Indicator, TIndicatorState};

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

    let results = Ccfisher::indicator_by_assets::<4>(&inputs, &[0.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Fisher: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Signal: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```rust
    use tulip_rs::indicators::ccfisher::{Ccfisher, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[0.0], &[0.1], &[0.2], &[0.3]];

    let results = Ccfisher::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Alpha set {} Fisher: {:?}", i + 1, opt_outputs[0]);
        println!("Alpha set {} Signal: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "C"

    **By assets** — same alpha applied to 4 assets in parallel:

    ```c
    #include "tulip_rs_ffi.h"

    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    const double *asset1[CCFISHER_INPUTS] = {a1};
    const double *asset2[CCFISHER_INPUTS] = {a2};
    const double *asset3[CCFISHER_INPUTS] = {a3};
    const double *asset4[CCFISHER_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = ccfisher_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's fisher series */
        ccfisher_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[CCFISHER_INPUTS] = {close};

    static const double opt0[CCFISHER_OPTIONS] = {0.0};
    static const double opt1[CCFISHER_OPTIONS] = {0.1};
    static const double opt2[CCFISHER_OPTIONS] = {0.2};
    static const double opt3[CCFISHER_OPTIONS] = {0.3};
    const double *const simd_opts[4] = {opt0, opt1, opt2, opt3};

    CSimdResult r = ccfisher_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset with alpha set i */
        ccfisher_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same alpha applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
        93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
        98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
        103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
        108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00,
    ], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 3.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.ccfisher.simd_by_assets(simd_inputs, [0.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Fisher: {out[0]}")
        print(f"Asset {i + 1} Signal: {out[1]}")
    ```

    **By options** — same asset, N different alpha values in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
        93.50, 94.10, 94.80, 95.20, 95.70, 96.30, 96.80, 97.10, 97.60, 98.20,
        98.70, 99.10, 99.80, 100.20, 100.70, 101.30, 101.80, 102.10, 102.60, 103.20,
        103.70, 104.10, 104.80, 105.20, 105.70, 106.30, 106.80, 107.10, 107.60, 108.20,
        108.70, 109.10, 109.80, 110.20, 110.70, 111.30, 111.80, 112.10, 112.60, 113.00,
    ], dtype=np.float64)

    simd_options = [[0.0], [0.1], [0.2], [0.3]]
    outputs_list, states = tulip_rs.indicators.ccfisher.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Alpha set {i + 1} Fisher: {out[0]}")
        print(f"Alpha set {i + 1} Signal: {out[1]}")
    ```

=== "Node.js"

    **By assets** — same alpha applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v + 5.0)],
        [close.map(v => v - 3.0)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.ccfisher.simdByAssets(simdInputs, [0]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} Fisher:`, out[0], 'Signal:', out[1]));
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```javascript
    const simdOptions = [[0], [0.1], [0.2], [0.3]];
    const [results] = ti.ccfisher.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Alpha set ${i + 1} Fisher:`, out[0]));
    ```
