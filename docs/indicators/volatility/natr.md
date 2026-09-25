# NATR — Normalized Average True Range

ATR expressed as a percentage of the closing price, making it comparable across different price levels.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[natr]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::natr::{Natr, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = Natr::indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // NATR values (as percentage)

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = Natr::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high  = vec![85.90_f64];
    let new_low   = vec![84.03_f64];
    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[NATR_OPTIONS] = {14.0}; // period
    const double *inputs[NATR_INPUTS] = {high, low, close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = natr_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> natr */
    tulip_ffi_result_free(r);
    natr_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = natr_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.90};
    double new_low[] = {84.03};
    double new_close[] = {85.53};
    const double *new_inputs[NATR_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = natr_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> natr for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    natr_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high  := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00}
    low   := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Natr.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // NATR values (as percentage)
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Natr.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued NATR
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Natr;

    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {14.0}; // period

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Natr.indicator(new double[][] {high, low, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // NATR values (as percentage)
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Natr.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10),
            java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued NATR
        }
    }
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

    outputs, state = tulip_rs.indicators.natr.indicator([high, low, close], [14.0])
    print(outputs[0])  # NATR values (as percentage)

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.natr.indicator([high, low, close], [14]);
    console.log('NATR(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.natr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued NATR:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.natr.indicator([high, low, close], [14]);
    console.log('NATR(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.natr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued NATR:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `natr` exposes 2 optional outputs: `"atr"`, `"tr"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::natr::{Natr, Indicator, TIndicatorState};

    let high  = vec![82.59, 82.06, 83.87, 84.00, 84.61,
                     84.15, 83.84, 84.99, 85.55, 85.36_f64];
    let low   = vec![80.59, 80.06, 81.87, 82.00, 82.61,
                     82.15, 81.84, 82.99, 83.55, 83.36_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true]; // request atr, tr
    let (outputs, _state) = Natr::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let natr = &outputs[0]; // natr (primary)
    let atr  = &outputs[1]; // atr  (optional — requested)
    let tr   = &outputs[2]; // tr   (optional — requested)
    ```

=== "C"

    `natr` exposes 2 optional outputs: `atr`, `tr`. Pass a boolean mask as the third argument.
    `natr` exposes 2 optional outputs: `atr`, `tr`. The inputs are [high, low, close] and options is [period].

    ```c
    #include "tulip_rs_ffi.h"

    double high[]  = {82.59, 82.06, 83.87, 84.00, 84.61,
                      84.15, 83.84, 84.99, 85.55, 85.36};
    double low[]   = {80.59, 80.06, 81.87, 82.00, 82.61,
                      82.15, 81.84, 82.99, 83.55, 83.36};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[NATR_OPTIONS] = {14.0}; // period
    const double *inputs[NATR_INPUTS] = {high, low, close};

    /* Request both optional outputs (atr and tr) */
    bool mask[] = {true, true};
    CIndicatorResult r = natr_indicator(inputs, 10, options, mask, 2);
    /* r.outputs[0] -> natr (primary), r.outputs[1] -> atr (optional 0), r.outputs[2] -> tr (optional 1) */
    tulip_ffi_result_free(r);
    natr_state_free(r.state);
    ```

=== "Go"

    `natr` exposes 2 optional outputs: `atr`, `tr`. Pass a boolean mask as the fourth argument — one `bool` per optional output, in order.

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high  := []float64{82.59, 82.06, 83.87, 84.00, 84.61,
                       84.15, 83.84, 84.99, 85.55, 85.36}
    low   := []float64{80.59, 80.06, 81.87, 82.00, 82.61,
                       82.15, 81.84, 82.99, 83.55, 83.36}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0} // period

    // Request both optional outputs (atr and tr)
    mask := []bool{true, true}
    res, st, _ := indicators.Natr.Indicator(high, low, close, options, mask)

    natr := res.Rows[0] // natr (primary)
    atr  := res.Rows[1] // atr  (optional 0 — requested)
    tr   := res.Rows[2] // tr   (optional 1 — requested)
    res.Close()
    st.Close()

    // Request only atr
    mask_atr_only := []bool{true, false}
    partialRes, partialSt, _ := indicators.Natr.Indicator(high, low, close, options, mask_atr_only)
    fmt.Println(partialRes.Rows[1]) // atr (optional 0)
    partialRes.Close()
    partialSt.Close()
    ```

=== "Java"

    `natr` exposes 2 optional outputs: `atr`, `tr`. Pass a boolean mask as the third argument — one `boolean` per optional output, in order.

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Natr;

    // (high/low/close series as in the Basic tab)
    boolean[] mask = {true, true}; // atr, tr
    Outcome oc = Natr.indicator(new double[][] {high, low, close}, new double[] {14.0}, mask);
    try (Result res = oc.result()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // natr (primary)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // atr  (optional 0 — requested)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2))); // tr   (optional 1 — requested)
    }
    oc.state().close();
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.59, 82.06, 83.87, 84.00, 84.61,
                      84.15, 83.84, 84.99, 85.55, 85.36], dtype=np.float64)
    low   = np.array([80.59, 80.06, 81.87, 82.00, 82.61,
                      82.15, 81.84, 82.99, 83.55, 83.36], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.natr.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, True],
    )

    natr = outputs[0]  # natr (primary)
    atr  = outputs[1]  # atr  (optional — requested)
    tr   = outputs[2]  # tr   (optional — requested)
    ```

=== "Node.js"

    `natr` exposes 2 optional outputs: `atr`, `tr`.

    ```javascript
    const [allOut] = ti.natr.indicator([high, low, close], [14], [true, true]);
    const natr = allOut[0]; // primary
    const atr  = allOut[1]; // optional 0: atr
    const tr   = allOut[2]; // optional 1: tr
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.natr.indicator([high, low, close], [14], [true, true]);
    const natr = allOut[0]; // primary
    const atr  = allOut[1]; // optional 0: atr
    const tr   = allOut[2]; // optional 1: tr
    ```

### SIMD

=== "Rust"

    ```rust
    use tulip_rs::indicators::natr::{Natr, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = Natr::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::natr::{Natr, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = Natr::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1_high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double a1_low[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double a1_close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    const double *const asset1[NATR_INPUTS] = {a1_high, a1_low, a1_close};

    double a2_high[] = {90.0, 89.5, 91.0, 91.3, 91.9, 92.0, 91.5, 92.0, 92.5, 92.8};
    double a2_low[] = {89.0, 88.5, 90.0, 90.3, 90.9, 91.0, 90.5, 91.0, 91.5, 91.8};
    double a2_close[] = {89.5, 89.0, 90.5, 90.8, 91.4, 91.5, 91.0, 91.5, 92.0, 92.3};
    const double *const asset2[NATR_INPUTS] = {a2_high, a2_low, a2_close};

    double a3_high[] = {75.0 + (double)i * 0.2 for i in 0..10}; /* simplified */
    double a3_low[] = {74.0 + (double)i * 0.2 for i in 0..10};
    double a3_close[] = {74.5 + (double)i * 0.2 for i in 0..10};
    const double *const asset3[NATR_INPUTS] = {a3_high, a3_low, a3_close};

    double a4_high[] = {85.0 - (double)i * 0.1 for i in 0..10};
    double a4_low[] = {84.0 - (double)i * 0.1 for i in 0..10};
    double a4_close[] = {84.5 - (double)i * 0.1 for i in 0..10};
    const double *const asset4[NATR_INPUTS] = {a4_high, a4_low, a4_close};

    /* simd_inputs is indexed by asset (the N=4 SIMD lanes) */
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = natr_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's natr */
        natr_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = natr_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i */
    for (uintptr_t i = 0; i < r.num_results; i++) natr_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, c2 := h1, l1, c1
    h3, l3, c3 := h1, l1, c1
    h4, l4, c4 := h1, l1, c1

    assets := [][indicators.NatrInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Natr.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, N option sets in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    assets2 := [][indicators.NatrInputs][]float64{{high, low, close}}
    sim2, _ := indicators.Natr.SimdByOptions(high, low, close, [][]float64{{7.0}, {14.0}, {21.0}, {28.0}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Natr;

    // Each entry lists one asset's input series (h1..c4 as in the C tab).
    double[][][] assets = {{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}};
    try (SimdResult sim = Natr.simdByAssets(assets, new double[] {14.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Natr;

    // (high/low/close series as in the Basic tab)
    try (SimdResult sim = Natr.simdByOptions(new double[][] {high, low, close},
            new double[][] {{7.0}, {14.0}, {21.0}, {28.0}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.natr.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.natr.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.natr.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.natr.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
