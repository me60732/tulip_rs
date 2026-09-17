# DEMA — Double Exponential Moving Average

Reduces EMA lag by applying a second EMA and combining the results: `2 * EMA - EMA(EMA)`. Tracks price more closely than a standard EMA of the same period.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[dema]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::dema::{Dema, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = Dema::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("DEMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Dema::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial DEMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued DEMA: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[DEMA_OPTIONS] = {14.0}; // period
    const double *inputs[DEMA_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = dema_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the DEMA(14) series, length r.output_lens[0]; r.outputs[1] -> EMA */
    tulip_ffi_result_free(r);
    dema_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = dema_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[DEMA_INPUTS] = {new_close};
    CBatchResult b = dema_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> DEMA values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    dema_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Dema.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // DEMA(14) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    partial := close[:8]
    res2, st2, _ := indicators.Dema.Indicator(partial, options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued DEMA values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.dema.indicator([close], [14.0])
    print("DEMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.dema.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued DEMA:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.dema.indicator([close], [14]);
    console.log('DEMA(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.dema.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued DEMA:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.dema.indicator([close], [14]);
    console.log('DEMA(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.dema.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued DEMA:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `dema` exposes 1 optional output: `"ema"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::dema::{Dema, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Request the EMA alongside the DEMA
    let mask = [true]; // one per optional output
    let (outputs, _state) = Dema::indicator(&[close.as_slice()], &[5.0], Some(&mask)).unwrap();

    let dema = &outputs[0]; // dema (primary)
    let ema  = &outputs[1]; // "ema" (optional — requested)
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[DEMA_OPTIONS] = {5.0}; // period
    const double *inputs[DEMA_INPUTS] = {close};
    bool optional_outputs[1] = {true}; // request ema

    CIndicatorResult r = dema_indicator(inputs, 10, options, optional_outputs, 1);
    /* r.outputs[0] -> DEMA (primary), r.outputs[1] -> EMA (optional) */
    tulip_ffi_result_free(r);
    dema_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{5.0} // period

    // Request optional output: ema
    mask := []bool{true}
    res, st, _ := indicators.Dema.Indicator(close, options, mask)

    dema := res.Rows[0]  // dema (primary)
    ema  := res.Rows[1]  // "ema" (optional — requested)
    fmt.Println("dema:", dema, "ema:", ema)
    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Request the EMA alongside the DEMA
    outputs, state = tulip_rs.indicators.dema.indicator(
        [close], [5.0],
        optional_outputs=[True],
    )

    dema = outputs[0]  # dema (primary)
    ema  = outputs[1]  # "ema" (optional — requested)
    ```

=== "Node.js"

    `dema` exposes 1 optional output: `ema`. Pass a boolean mask as the third argument.

    ```javascript
    // Request the EMA alongside the DEMA
    const [allOut] = ti.dema.indicator([close], [5], [true]);
    const dema = allOut[0]; // primary
    const ema  = allOut[1]; // optional: ema
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    // Request the EMA alongside the DEMA
    const [allOut] = ti.dema.indicator([close], [5], [true]);
    const dema = allOut[0]; // primary
    const ema  = allOut[1]; // optional: ema
    ```
### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::dema::{Dema, Indicator};

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

    let results = Dema::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::dema::{Dema, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    use tulip_rs::indicators::dema::{Dema, IndicatorByOptions};
    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = Dema::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
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
    const double *asset1[DEMA_INPUTS] = {a1};
    const double *asset2[DEMA_INPUTS] = {a2};
    const double *asset3[DEMA_INPUTS] = {a3};
    const double *asset4[DEMA_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = dema_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's DEMA series, length r.output_lens[i][0] */
        dema_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o5[] = {5.0}, o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0};
    const double *const simd_opts[4] = {o5, o10, o14, o20};

    CSimdResult r = dema_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> DEMA results for option set i (periods 5/10/14/20) */
    for (uintptr_t i = 0; i < r.num_results; i++) dema_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := a1, a1, a1

    assets := [][indicators.DemaInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Dema.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Dema.SimdByOptions(close, [][]float64{{5}, {10}, {14}, {20}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.dema.simd_by_assets(simd_inputs, [14.0])
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
    outputs_list, states = tulip_rs.indicators.dema.simd_by_options([close], simd_options)
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
    const [results] = ti.dema.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.dema.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
