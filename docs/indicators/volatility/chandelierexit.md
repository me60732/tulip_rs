# Chandelier Exit

A trailing stop-loss indicator that dynamically adjusts with volatility. The long line sits at the highest high over the look-back period minus a multiple of ATR; the short line sits at the lowest low plus the same multiple. A close crossing below the long line, or above the short line, signals a potential trend reversal or exit.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period, step]` &nbsp;|&nbsp; **Outputs:** `[long, short]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::chandelierexit::{ChandelierExit, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    // Options: [period, step]  — step is the ATR multiplier (default 2)
    let (outputs, mut state) = ChandelierExit::indicator(&inputs, &[14.0, 2.0], None).unwrap();
    println!("{:?}", outputs[0]); // long stop values
    println!("{:?}", outputs[1]); // short stop values

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = ChandelierExit::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[14.0, 2.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high  = vec![86.54_f64];
    let new_low   = vec![85.39_f64];
    let new_close = vec![86.53_f64];
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
                     83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29};
    double options[CHANDELIEREXIT_OPTIONS] = {14.0, 2.0}; // period, multiplier
    const double *inputs[CHANDELIEREXIT_INPUTS] = {high, low, close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = chandelierexit_indicator(inputs, 15, options, NULL, 0);
    /* r.outputs[0] -> long, r.outputs[1] -> short */
    tulip_ffi_result_free(r);
    chandelierexit_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = chandelierexit_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {86.54};
    double new_low[] = {85.39};
    double new_close[] = {86.53};
    const double *new_inputs[CHANDELIEREXIT_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = chandelierexit_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> long for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    chandelierexit_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high  := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00,
                       85.90, 86.58, 86.98, 88.00, 87.87}
    low   := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29}
    options := []float64{14.0, 2.0} // period, step

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Chandelierexit.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // long stop values
    fmt.Println(res.Rows[1]) // short stop values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Chandelierexit.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued long stop
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Chandelierexit;

    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29};
    double[] options = {14.0, 2.0}; // period, step

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Chandelierexit.indicator(new double[][] {high, low, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // long stop values
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // short stop values
    }

    // Partial computation + state continuation.
    Outcome p = Chandelierexit.indicator(
        new double[][] {
            java.util.Arrays.copyOfRange(high, 0, 8),
            java.util.Arrays.copyOfRange(low, 0, 8),
            java.util.Arrays.copyOfRange(close, 0, 8)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, 8, 15),
            java.util.Arrays.copyOfRange(low, 8, 15),
            java.util.Arrays.copyOfRange(close, 8, 15)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued long stop
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    outputs, state = tulip_rs.indicators.chandelierexit.indicator(
        [high, low, close], [14.0, 2.0]
    )
    print(outputs[0])  # long stop values
    print(outputs[1])  # short stop values

    # State continuation
    new_high  = np.array([88.50], dtype=np.float64)
    new_low   = np.array([87.30], dtype=np.float64)
    new_close = np.array([88.10], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.chandelierexit.indicator([high, low, close], [14, 2]);
    console.log('Long stop:', outputs[0]);
    console.log('Short stop:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.chandelierexit.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14, 2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued long stop:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.chandelierexit.indicator([high, low, close], [14, 2]);
    console.log('Long stop:', outputs[0]);
    console.log('Short stop:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.chandelierexit.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14, 2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued long stop:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `chandelierexit` exposes 4 optional outputs: `atr`, `tr`, `min`, `max`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::chandelierexit::{ChandelierExit, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let mask = [true, true, false, false];
    let (outputs, _state) = ChandelierExit::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0, 2.0],
        Some(&mask),
    ).unwrap();

    let long = &outputs[0]; // long (primary)
    let short = &outputs[1]; // short (primary)
    let atr  = &outputs[2]; // atr (optional — requested)
    let tr   = &outputs[3]; // tr (optional — requested)
    // min and max not requested — omitted from outputs
    ```

=== "C"

    `chandelierexit` exposes 4 optional outputs: `atr`, `tr`, `min`, `max`. Pass a boolean mask as the third argument.
    `chandelierexit` exposes 4 optional outputs: `atr`, `tr`, `min`, `max`. The inputs are [high, low, close] and options is [period, step].

    ```c
    #include "tulip_rs_ffi.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29};
    double options[CHANDELIEREXIT_OPTIONS] = {14.0, 2.0}; // period, step
    const double *inputs[CHANDELIEREXIT_INPUTS] = {high, low, close};

    /* Request both atr and tr (skip min and max) */
    bool mask[] = {true, true, false, false};
    CIndicatorResult r = chandelierexit_indicator(inputs, 15, options, mask, 4);
    /* r.outputs[0] -> long, r.outputs[1] -> short,
       r.outputs[2] -> atr (optional 0), r.outputs[3] -> tr (optional 1) */
    tulip_ffi_result_free(r);
    chandelierexit_state_free(r.state);

    // Request all optional outputs
    bool mask_all[] = {true, true, true, true};
    CIndicatorResult full_r = chandelierexit_indicator(inputs, 15, options, mask_all, 4);
    tulip_ffi_result_free(full_r);
    chandelierexit_state_free(full_r.state);
    ```

=== "Go"

    `chandelierexit` exposes 4 optional outputs: `atr`, `tr`, `min`, `max`. Pass a boolean mask as the fourth argument — one `bool` per optional output, in order.

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high  := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                       85.90, 86.58, 86.98, 88.00, 87.87}
    low   := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29}
    options := []float64{14.0, 2.0} // period, step

    // Request atr and tr only (skip min and max)
    mask := []bool{true, true, false, false}
    res, st, _ := indicators.Chandelierexit.Indicator(high, low, close, options, mask)

    long  := res.Rows[0] // long (primary)
    short := res.Rows[1] // short (primary)
    atr   := res.Rows[2] // atr (optional 0 — requested)
    tr    := res.Rows[3] // tr (optional 1 — requested)
    res.Close()
    st.Close()

    // Request all optional outputs
    mask_all := []bool{true, true, true, true}
    fullRes, fullSt, _ := indicators.Chandelierexit.Indicator(high, low, close, options, mask_all)
    fmt.Println(fullRes.Rows[4]) // min (optional 2)
    fmt.Println(fullRes.Rows[5]) // max (optional 3)
    fullRes.Close()
    fullSt.Close()
    ```

=== "Java"

    `chandelierexit` exposes 4 optional outputs: `atr`, `tr`, `min`, `max`. Pass a boolean mask as the third argument — one `boolean` per optional output, in order.

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Chandelierexit;

    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29};
    double[] options = {14.0, 2.0}; // period, step

    // Request atr and tr only (skip min and max)
    boolean[] mask = {true, true, false, false}; // atr, tr, min, max
    Outcome oc = Chandelierexit.indicator(new double[][] {high, low, close}, options, mask);
    try (Result res = oc.result()) {
        // row 0 = long (primary), row 1 = short (primary),
        // row 2 = atr (optional 0 — requested), row 3 = tr (optional 1 — requested)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // long
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // short
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2))); // atr
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(3))); // tr
    }
    oc.state().close();

    // Request all optional outputs
    boolean[] mask_all = {true, true, true, true};
    Outcome fullOc = Chandelierexit.indicator(new double[][] {high, low, close}, options, mask_all);
    try (Result fullRes = fullOc.result()) {
        System.out.println(java.util.Arrays.toString(fullRes.toDoubleArray(4))); // min
        System.out.println(java.util.Arrays.toString(fullRes.toDoubleArray(5))); // max
    }
    fullOc.state().close();
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    outputs, state = tulip_rs.indicators.chandelierexit.indicator(
        [high, low, close], [14.0, 2.0],
        optional_outputs=[True, True, False, False],
    )

    long  = outputs[0]  # long (primary)
    short = outputs[1]  # short (primary)
    atr   = outputs[2]  # atr (optional — requested)
    tr    = outputs[3]  # tr (optional — requested)
    # min and max not requested — omitted from outputs
    ```

=== "Node.js"

    `chandelierexit` exposes 4 optional outputs: `atr`, `tr`, `min`, `max`.

    ```javascript
    const [allOut] = ti.chandelierexit.indicator([high, low, close], [14, 2], [true, true, true, true]);
    const long  = allOut[0]; // primary
    const short = allOut[1]; // primary
    const atr   = allOut[2]; // optional 0: atr
    const tr    = allOut[3]; // optional 1: tr
    const min   = allOut[4]; // optional 2: min (rolling lowest low)
    const max   = allOut[5]; // optional 3: max (rolling highest high)

    // Request only atr and tr
    const [partial] = ti.chandelierexit.indicator([high, low, close], [14, 2], [true, true, false, false]);
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.chandelierexit.indicator([high, low, close], [14, 2], [true, true, true, true]);
    const long  = allOut[0]; // primary
    const short = allOut[1]; // primary
    const atr   = allOut[2]; // optional 0: atr
    const tr    = allOut[3]; // optional 1: tr
    const min   = allOut[4]; // optional 2: min (rolling lowest low)
    const max   = allOut[5]; // optional 3: max (rolling highest high)

    // Request only atr and tr
    const [partial] = ti.chandelierexit.indicator([high, low, close], [14, 2], [true, true, false, false]);
    ```

### SIMD

=== "Rust"

    ```rust
    use tulip_rs::indicators::chandelierexit::{ChandelierExit, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = ChandelierExit::indicator_by_assets::<4>(&inputs, &[14.0, 2.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: long={:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::chandelierexit::{ChandelierExit, IndicatorByOptions};

    let opts: [&[f64; 2]; 4] = [&[10.0, 2.0], &[14.0, 2.0], &[20.0, 2.0], &[30.0, 3.0]];
    let results = ChandelierExit::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period={} step={}: long={:?}", opts[i][0], opts[i][1], out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1_high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                        85.90, 86.58, 86.98, 88.00, 87.87};
    double a1_low[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01};
    double a1_close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                         85.53, 86.54, 86.89, 87.77, 87.29};
    const double *const asset1[CHANDELIEREXIT_INPUTS] = {a1_high, a1_low, a1_close};

    double a2_high[] = {85.0, 84.5, 86.0, 86.3, 86.9, 87.0, 86.5, 87.0, 87.5, 87.8};
    double a2_low[] = {84.0, 83.5, 85.0, 85.3, 85.9, 86.0, 85.5, 86.0, 86.5, 86.8};
    double a2_close[] = {84.5, 84.0, 85.5, 85.8, 86.4, 86.5, 86.0, 86.5, 87.0, 87.3};
    const double *const asset2[CHANDELIEREXIT_INPUTS] = {a2_high, a2_low, a2_close};

    double a3_high[] = {90.0 + (double)i * 0.5 + 82.15 * 0.1 for i in 0..15}; /* simplified */
    double a3_low[] = {90.0 + (double)i * 0.5 + 81.29 * 0.1 for i in 0..15};
    double a3_close[] = {90.0 + (double)i * 0.5 + 81.59 * 0.1 for i in 0..15};
    const double *const asset3[CHANDELIEREXIT_INPUTS] = {a3_high, a3_low, a3_close};

    double a4_high[] = {100.0 - (double)i * 0.3 + 82.15 * 0.05 for i in 0..15};
    double a4_low[] = {100.0 - (double)i * 0.3 + 81.29 * 0.05 for i in 0..15};
    double a4_close[] = {100.0 - (double)i * 0.3 + 81.59 * 0.05 for i in 0..15};
    const double *const asset4[CHANDELIEREXIT_INPUTS] = {a4_high, a4_low, a4_close};

    /* simd_inputs is indexed by asset (the N=4 SIMD lanes) */
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = chandelierexit_simd_by_assets(simd_inputs, 4, 15, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's long */
        chandelierexit_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in one call:

    ```c
    static const double options_1[CHANDELIEREXIT_OPTIONS] = {10.0, 2.0};
    static const double options_2[CHANDELIEREXIT_OPTIONS] = {14.0, 2.0};
    static const double options_3[CHANDELIEREXIT_OPTIONS] = {20.0, 2.0};
    static const double options_4[CHANDELIEREXIT_OPTIONS] = {30.0, 3.0};
    const double *const simd_opts[4] = {options_1, options_2, options_3, options_4};

    CSimdResult r = chandelierexit_simd_by_options(inputs, 15, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i */
    for (uintptr_t i = 0; i < r.num_results; i++) chandelierexit_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1_high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                        85.90, 86.58, 86.98, 88.00, 87.87}
    a1_low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01}
    a1_close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                         85.53, 86.54, 86.89, 87.77, 87.29}

    // Reuse the same data for assets 2–4 in this example
    a2_high, a2_low, a2_close := a1_high, a1_low, a1_close
    a3_high, a3_low, a3_close := a1_high, a1_low, a1_close
    a4_high, a4_low, a4_close := a1_high, a1_low, a1_close

    assets := [][indicators.ChandelierexitInputs][]float64{{a1_high, a1_low, a1_close}, {a2_high, a2_low, a2_close}, {a3_high, a3_low, a3_close}, {a4_high, a4_low, a4_close}}
    sim, _ := indicators.Chandelierexit.SimdByAssets(assets, []float64{14.0, 2.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: long=%v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, N option sets in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29}

    assets2 := [][indicators.ChandelierexitInputs][]float64{{high, low, close}}
    sim2, _ := indicators.Chandelierexit.SimdByOptions(high, low, close, [][]float64{{10.0, 2.0}, {14.0, 2.0}, {20.0, 2.0}, {30.0, 3.0}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period=%d step=%d: long=%v\n", i+1, lanes[0][0], lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Chandelierexit;

    double[] a1_high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                        85.90, 86.58, 86.98, 88.00, 87.87};
    double[] a1_low = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01};
    double[] a1_close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                         85.53, 86.54, 86.89, 87.77, 87.29};
    double[] a2_high = {85.0, 84.5, 86.0, 86.3, 86.9, 87.0, 86.5, 87.0, 87.5, 87.8,
                        88.0, 88.5, 88.9, 89.0, 88.8};
    double[] a2_low = {84.0, 83.5, 85.0, 85.3, 85.9, 86.0, 85.5, 86.0, 86.5, 86.8,
                       87.0, 87.5, 87.9, 88.0, 87.8};
    double[] a2_close = {84.5, 84.0, 85.5, 85.8, 86.4, 86.5, 86.0, 86.5, 87.0, 87.3,
                         88.0, 88.5, 88.9, 89.0, 88.8};
    double[] a3_high = {90.15, 90.89, 92.03, 92.30, 92.85, 92.90, 92.33, 93.30, 93.84, 94.00,
                        94.90, 95.58, 95.98, 96.00, 95.87};
    double[] a3_low = {89.29, 89.64, 90.31, 90.65, 91.07, 91.11, 90.49, 90.30, 92.15, 92.11,
                       92.03, 93.39, 93.76, 94.17, 94.01};
    double[] a3_close = {89.59, 89.06, 90.87, 91.00, 91.61, 91.15, 90.84, 91.99, 92.55, 92.36,
                         93.53, 94.54, 94.89, 95.77, 95.29};
    double[] a4_high = {100.15, 100.89, 102.03, 102.30, 102.85, 102.90, 102.33, 103.30, 103.84, 104.00,
                        104.90, 105.58, 105.98, 106.00, 105.87};
    double[] a4_low = {99.29, 99.64, 100.31, 100.65, 101.07, 101.11, 100.49, 100.30, 102.15, 102.11,
                       102.03, 103.39, 103.76, 104.17, 104.01};
    double[] a4_close = {99.59, 99.06, 100.87, 101.00, 101.61, 101.15, 100.84, 101.99, 102.55, 102.36,
                         103.53, 104.54, 104.89, 105.77, 105.29};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1_high, a1_low, a1_close}, {a2_high, a2_low, a2_close},
                           {a3_high, a3_low, a3_close}, {a4_high, a4_low, a4_close}};
    try (SimdResult sim = Chandelierexit.simdByAssets(assets, new double[] {14.0, 2.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: long=%s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, N option sets in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Chandelierexit;

    double[] high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87};
    double[] low = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29};

    try (SimdResult sim = Chandelierexit.simdByOptions(new double[][] {high, low, close},
            new double[][] {{10.0, 2.0}, {14.0, 2.0}, {20.0, 2.0}, {30.0, 3.0}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period=%d step=%d: long=%s%n", i + 1,
                sim.toDoubleArray(i, 0)[0], java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
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
    outputs_list, states = tulip_rs.indicators.chandelierexit.simd_by_assets(simd_inputs, [14.0, 2.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: long={asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[10.0, 2.0], [14.0, 2.0], [20.0, 2.0], [30.0, 3.0]]
    outputs_list, states = tulip_rs.indicators.chandelierexit.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period={simd_options[i][0]} step={simd_options[i][1]}: long={out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.chandelierexit.simdByAssets(simdInputs, [14, 2]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}: long=`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[10, 2], [14, 2], [20, 2], [30, 3]];
    const [results] = ti.chandelierexit.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period=${simdOptions[i][0]} step=${simdOptions[i][1]}: long=`, out[0]));
    ```
