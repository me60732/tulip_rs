# TRENDMODE — Ehlers TrendMode

Detects whether price is in trend mode or cycle mode; output is `1.0` in trend mode and `0.0` in cycle mode, with `cycle` and `peak` optional outputs exposing the internal Cyber Cycle value and its peak used for mode detection.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[alpha]` &nbsp;|&nbsp; **Outputs:** `[trendmode]` &nbsp;|&nbsp; **Optional:** `[cycle, peak]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::trendmode::{TrendMode, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    // Options: [alpha] — default 0.07
    let (outputs, _state) = TrendMode::indicator(&[close.as_slice()], &[0.07], None).unwrap();
    println!("TrendMode: {:?}", outputs[0]);

    // State continuation
    let partial = close[..35].to_vec();
    let (outputs2, mut state) = TrendMode::indicator(&[partial.as_slice()], &[0.07], None).unwrap();
    println!("Partial TrendMode: {:?}", outputs2[0]);

    let new_close = close[35..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued TrendMode: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[TRENDMODE_OPTIONS] = {0.07}; // alpha
    const double *inputs[TRENDMODE_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = trendmode_indicator(inputs, 40, options, NULL, 0);
    /* r.outputs[0] -> the trend mode series, length r.output_lens[0];
       r.outputs[1] -> cycle (optional — not requested here) */
    tulip_ffi_result_free(r);
    trendmode_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = trendmode_indicator(inputs, 35, options, NULL, 0);
    double new_close[] = {92.80, 93.10, 92.50, 93.20};
    const double *new_inputs[TRENDMODE_INPUTS] = {new_close};
    CBatchResult b = trendmode_batch(p.state, new_inputs, 4, NULL, 0);
    /* b.outputs[0] -> trend mode values for just the four new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    trendmode_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{0.07} // alpha

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Trendmode.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // trend mode values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    partial := close[:35]
    res2, st2, _ := indicators.Trendmode.Indicator(partial, options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[35:], nil)
    fmt.Println(batch.Rows[0]) // continued trend mode values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Trendmode;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double[] options = {0.07}; // alpha

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Trendmode.indicator(new double[][] {close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // trend mode values
    }

    // Partial computation + state continuation.
    Outcome p = Trendmode.indicator(
        new double[][] {java.util.Arrays.copyOfRange(close, 0, 35)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(
            new double[][] {java.util.Arrays.copyOfRange(close, 35, 40)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued trend mode values
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    # Options: [alpha] — default 0.07
    outputs, state = tulip_rs.indicators.trendmode.indicator([close], [0.07])
    print("TrendMode:", outputs[0])

    # State continuation
    partial = close[:35]
    outputs2, state = tulip_rs.indicators.trendmode.indicator([partial], [0.07])
    new_close = close[35:]
    continued = state.batch_indicator([new_close])
    print("Continued TrendMode:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20]);

    const [outputs, state] = ti.trendmode.indicator([close], [0.07]);
    console.log('TrendMode:', outputs[0]);

    // State continuation
    const [, state2] = ti.trendmode.indicator([close.slice(0, -5)], [0.07]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TrendMode:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init, trendmode } from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20];

    const [outputs, state] = trendmode.indicator([close], [0.07]);
    console.log('TrendMode:', outputs[0]);

    // State continuation
    const [, state2] = trendmode.indicator([close.slice(0, -5)], [0.07]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TrendMode:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `trendmode` exposes 2 optional outputs: `cycle`, `peak`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::trendmode::{TrendMode, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    let mask = [true, true]; // one per optional output
    let (outputs, _state) = TrendMode::indicator(&[close.as_slice()], &[0.07], Some(&mask)).unwrap();

    let trendmode = &outputs[0]; // trendmode (primary)
    let cycle     = &outputs[1]; // cycle (optional — requested)
    let peak      = &outputs[2]; // peak (optional — requested)
    ```

=== "C"

    `trendmode` exposes 2 optional outputs: `cycle`, `peak`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[TRENDMODE_OPTIONS] = {0.07}; // alpha
    const double *inputs[TRENDMODE_INPUTS] = {close};
    bool optional_outputs[2] = {true, true}; // cycle=true, peak=true

    CIndicatorResult r = trendmode_indicator(inputs, 40, options, optional_outputs, 2);
    /* r.outputs[0] -> trendmode (primary)
       r.outputs[1] -> cycle (optional — requested)
       r.outputs[2] -> peak (optional — requested) */
    tulip_ffi_result_free(r);
    trendmode_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{0.07} // alpha

    // Request optional outputs: cycle and peak
    mask := []bool{true, true}
    res, st, _ := indicators.Trendmode.Indicator(close, options, mask)

    trendMode := res.Rows[0]  // trendmode (primary)
    cycle     := res.Rows[1]  // cycle (optional — requested)
    peak      := res.Rows[2]  // peak (optional — requested)
    fmt.Println("trendMode:", trendMode, "cycle:", cycle, "peak:", peak)
    res.Close()
    st.Close()
    ```

=== "Java"

    `trendmode` exposes 2 optional outputs: `cycle`, `peak`. Pass a boolean mask as the third argument — one `boolean` per optional output, in order.

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Trendmode;

    // (close series as in the Basic tab)
    boolean[] mask = {true, true}; // cycle=true, peak=true
    Outcome oc = Trendmode.indicator(new double[][] {close}, new double[] {0.07}, mask);
    try (Result res = oc.result()) {
        // row 0 = trendmode (primary), row 1 = cycle, row 2 = peak
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // cycle
    }
    oc.state().close();
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    outputs, state = tulip_rs.indicators.trendmode.indicator(
        [close], [0.07],
        optional_outputs=[True, True],
    )

    trendmode = outputs[0]  # trendmode (primary)
    cycle     = outputs[1]  # cycle (optional — requested)
    peak      = outputs[2]  # peak (optional — requested)
    ```

=== "Node.js"

    `trendmode` exposes 2 optional outputs: `cycle`, `peak`.

    ```javascript
    const [allOut] = ti.trendmode.indicator([close], [0.07], [true, true]);
    const trendMode = allOut[0]; // primary
    const cycle     = allOut[1]; // optional 0: cycle
    const peak      = allOut[2]; // optional 1: peak
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = trendmode.indicator([close], [0.07], [true, true]);
    const trendMode = allOut[0]; // primary
    const cycle     = allOut[1]; // optional 0: cycle
    const peak      = allOut[2]; // optional 1: peak
    ```

### SIMD

=== "Rust"

    **By assets** — same alpha applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::trendmode::{TrendMode, Indicator, TIndicatorState};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                  85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                  88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                  90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                  77.00, 77.50, 78.00, 78.50, 79.00, 79.50, 80.00, 80.50, 81.00, 81.50,
                  82.00, 82.50, 83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50,
                  87.00, 87.50, 88.00, 88.50, 89.00, 89.50, 90.00, 90.50, 91.00, 91.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                  58.70, 59.00, 59.30, 59.60, 59.90, 60.20, 60.50, 60.80, 61.10, 61.40,
                  61.70, 62.00, 62.30, 62.60, 62.90, 63.20, 63.50, 63.80, 64.10, 64.40,
                  64.70, 65.00, 65.30, 65.60, 65.90, 66.20, 66.50, 66.80, 67.10, 67.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                  104.1, 104.5, 105.0, 105.3, 105.8, 106.0, 106.5, 107.0, 107.3, 107.8,
                  108.1, 108.5, 109.0, 109.3, 109.8, 110.0, 110.5, 111.0, 111.3, 111.8,
                  112.1, 112.5, 113.0, 113.3, 113.8, 114.0, 114.5, 115.0, 115.3, 115.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = TrendMode::indicator_by_assets::<4>(&inputs, &[0.07], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```rust
    use tulip_rs::indicators::trendmode::{TrendMode, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    let opts: [&[f64; 1]; 4] = [&[0.05], &[0.07], &[0.10], &[0.15]];

    let results = TrendMode::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Alpha set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same alpha applied to N assets in parallel (N must be 2/4/8/16):

    ```c
    #include "tulip_rs_ffi.h"

    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                   77.00, 77.50, 78.00, 78.50, 79.00, 79.50, 80.00, 80.50, 81.00, 81.50,
                   82.00, 82.50, 83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50,
                   87.00, 87.50, 88.00, 88.50, 89.00, 89.50, 90.00, 90.50, 91.00, 91.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                   58.70, 59.00, 59.30, 59.60, 59.90, 60.20, 60.50, 60.80, 61.10, 61.40,
                   61.70, 62.00, 62.30, 62.60, 62.90, 63.20, 63.50, 63.80, 64.10, 64.40,
                   64.70, 65.00, 65.30, 65.60, 65.90, 66.20, 66.50, 66.80, 67.10, 67.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                   104.1, 104.5, 105.0, 105.3, 105.8, 106.0, 106.5, 107.0, 107.3, 107.8,
                   108.1, 108.5, 109.0, 109.3, 109.8, 110.0, 110.5, 111.0, 111.3, 111.8,
                   112.1, 112.5, 113.0, 113.3, 113.8, 114.0, 114.5, 115.0, 115.3, 115.8};

    const double *asset1[TRENDMODE_INPUTS] = {a1};
    const double *asset2[TRENDMODE_INPUTS] = {a2};
    const double *asset3[TRENDMODE_INPUTS] = {a3};
    const double *asset4[TRENDMODE_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    double options[TRENDMODE_OPTIONS] = {0.07};

    CSimdResult r = trendmode_simd_by_assets(simd_inputs, 4, 40, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        trendmode_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, N different alpha values in parallel:

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[TRENDMODE_INPUTS] = {close};

    double o05[] = {0.05}, o07[] = {0.07}, o10[] = {0.10}, o15[] = {0.15};
    const double *const simd_opts[4] = {o05, o07, o10, o15};

    CSimdResult r = trendmode_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) trendmode_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same alpha applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                    85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                    88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                    90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := a1, a1, a1

    assets := [][indicators.TrendmodeInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Trendmode.SimdByAssets(assets, []float64{0.07}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Trendmode.SimdByOptions(close, [][]float64{{0.05}, {0.07}, {0.10}, {0.15}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Alpha set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same alpha applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Trendmode;

    double[] a1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double[] a2 = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                   77.00, 77.50, 78.00, 78.50, 79.00, 79.50, 80.00, 80.50, 81.00, 81.50,
                   82.00, 82.50, 83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50,
                   87.00, 87.50, 88.00, 88.50, 89.00, 89.50, 90.00, 90.50, 91.00, 91.50};
    double[] a3 = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                   58.70, 59.00, 59.30, 59.60, 59.90, 60.20, 60.50, 60.80, 61.10, 61.40,
                   61.70, 62.00, 62.30, 62.60, 62.90, 63.20, 63.50, 63.80, 64.10, 64.40,
                   64.70, 65.00, 65.30, 65.60, 65.90, 66.20, 66.50, 66.80, 67.10, 67.40};
    double[] a4 = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                   104.1, 104.5, 105.0, 105.3, 105.8, 106.0, 106.5, 107.0, 107.3, 107.8,
                   108.1, 108.5, 109.0, 109.3, 109.8, 110.0, 110.5, 111.0, 111.3, 111.8,
                   112.1, 112.5, 113.0, 113.3, 113.8, 114.0, 114.5, 115.0, 115.3, 115.8};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1}, {a2}, {a3}, {a4}};
    try (SimdResult sim = Trendmode.simdByAssets(assets, new double[] {0.07}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Trendmode;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    try (SimdResult sim = Trendmode.simdByOptions(new double[][] {close},
            new double[][] {{0.05}, {0.07}, {0.10}, {0.15}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Alpha set %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same alpha applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.trendmode.simd_by_assets(simd_inputs, [0.07])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different alpha values in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    simd_options = [[0.05], [0.07], [0.10], [0.15]]
    outputs_list, states = tulip_rs.indicators.trendmode.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Alpha set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same alpha applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.trendmode.simdByAssets(simdInputs, [0.07]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```javascript
    const simdOptions = [[0.05], [0.07], [0.10], [0.15]];
    const [results] = ti.trendmode.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Alpha set ${i + 1}:`, out[0]));
    ```
