# TEMA — Triple Exponential Moving Average

Further reduces lag with three EMA layers: `3 * EMA - 3 * EMA(EMA) + EMA(EMA(EMA))`. Reacts to price changes more quickly than DEMA at the cost of additional sensitivity to noise.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[tema]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::tema::{Tema, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = Tema::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("TEMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Tema::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial TEMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued TEMA: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[TEMA_OPTIONS] = {14.0}; // period
    const double *inputs[TEMA_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = tema_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the TEMA(14) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    tema_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = tema_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[TEMA_INPUTS] = {new_close};
    CBatchResult b = tema_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> TEMA values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    tema_state_free(p.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.tema.indicator([close], [14.0])
    print("TEMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.tema.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued TEMA:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.tema.indicator([close], [14]);
    console.log('TEMA(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.tema.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TEMA:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.tema.indicator([close], [14]);
    console.log('TEMA(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.tema.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TEMA:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `tema` exposes 2 optional outputs: `"dema"`, `"ema"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::tema::{Tema, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Request dema but not ema
    let mask = [true, false]; // one per optional output
    let (outputs, _state) = Tema::indicator(&[close.as_slice()], &[5.0], Some(&mask)).unwrap();

    let tema = &outputs[0]; // tema (primary)
    let dema = &outputs[1]; // "dema" (optional — requested)
                            // "ema" not requested
    ```

=== "C"

    `tema` exposes 2 optional outputs: `dema`, `ema`. Pass a boolean mask — one bool per optional output.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[TEMA_OPTIONS] = {5.0}; // period
    const double *inputs[TEMA_INPUTS] = {close};
    bool optional_outputs[] = {true, false}; // dema requested, ema not requested

    CIndicatorResult r = tema_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> tema (primary) */
    /* r.outputs[1] -> dema (optional — requested) */
    tulip_ffi_result_free(r);
    tema_state_free(r.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Request dema but not ema
    outputs, state = tulip_rs.indicators.tema.indicator(
        [close], [5.0],
        optional_outputs=[True, False],
    )

    tema = outputs[0]  # tema (primary)
    dema = outputs[1]  # "dema" (optional — requested)
                       # "ema" not requested
    ```

=== "Node.js"

    `tema` exposes 2 optional outputs: `dema`, `ema`. Pass a boolean mask — one bool per optional output.

    ```javascript
    // Request all optional outputs
    const [allOut] = ti.tema.indicator([close], [5], [true, true]);
    const tema = allOut[0]; // primary
    const dema = allOut[1]; // optional 0: dema
    const ema  = allOut[2]; // optional 1: ema

    // Request only dema
    const [partial] = ti.tema.indicator([close], [5], [true, false]);
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    // Request all optional outputs
    const [allOut] = ti.tema.indicator([close], [5], [true, true]);
    const tema = allOut[0]; // primary
    const dema = allOut[1]; // optional 0: dema
    const ema  = allOut[2]; // optional 1: ema

    // Request only dema
    const [partial] = ti.tema.indicator([close], [5], [true, false]);
    ```
### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::tema::{Tema, Indicator};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = Tema::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::tema::{Tema, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = Tema::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[TEMA_INPUTS] = {a1};
    const double *asset2[TEMA_INPUTS] = {a2};
    const double *asset3[TEMA_INPUTS] = {a3};
    const double *asset4[TEMA_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = tema_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        tema_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o5[] = {5.0}, o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0};
    const double *const simd_opts[4] = {o5, o10, o14, o20};

    CSimdResult r = tema_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i (periods 5/10/14/20) */
    for (uintptr_t i = 0; i < r.num_results; i++) tema_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.tema.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.tema.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.tema.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.tema.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
