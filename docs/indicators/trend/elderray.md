# Elder-Ray

Splits market force into two components relative to an EMA of close. Bull power = high − EMA, measuring how far above the average buyers can push price; Bear power = low − EMA, measuring how far below sellers can push it. Positive bull power alongside a rising EMA confirms bullish momentum; negative bear power with a falling EMA confirms bearish pressure. The EMA itself is available as an optional overlay output.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[bull_power, bear_power]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::elderray::{ElderRay, Indicator, TIndicatorState};

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
    let (outputs, mut state) = ElderRay::indicator(&inputs, &[5.0], None).unwrap();
    println!("{:?}", outputs[0]); // bull_power
    println!("{:?}", outputs[1]); // bear_power

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = ElderRay::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[5.0], None).unwrap();
    println!("{:?}", outputs2[0]); // bull_power
    println!("{:?}", outputs2[1]); // bear_power

    let new_high  = vec![86.54_f64];
    let new_low   = vec![85.39_f64];
    let new_close = vec![86.53_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]); // continued bull_power
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29};
    double options[ELDERRAY_OPTIONS] = {5.0}; // period
    const double *inputs[ELDERRAY_INPUTS] = {high, low, close};

    /* Full computation */
    CIndicatorResult r = elderray_indicator(inputs, 15, options, NULL, 0);
    tulip_ffi_result_free(r);
    elderray_state_free(r.state);

    /* Partial + continuation */
    CIndicatorResult p = elderray_indicator(inputs, 8, options, NULL, 0);
    const double *rest_inputs[ELDERRAY_INPUTS] = {high + 8, low + 8, close + 8};
    CBatchResult b = elderray_batch(p.state, rest_inputs, 7, NULL, 0);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    elderray_state_free(p.state);
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
    options := []float64{5.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Elderray.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // bull_power
    fmt.Println(res.Rows[1]) // bear_power
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Elderray.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued bull_power
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Elderray;

    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29};
    double[] options = {5.0};

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Elderray.indicator(new double[][] {high, low, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // bull_power
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // bear_power
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Elderray.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 15),
            java.util.Arrays.copyOfRange(low, n, 15),
            java.util.Arrays.copyOfRange(close, n, 15)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued bull_power
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

    outputs, state = tulip_rs.indicators.elderray.indicator(
        [high, low, close], [5.0]
    )
    print(outputs[0])  # bull_power
    print(outputs[1])  # bear_power

    # State continuation
    new_high  = np.array([86.54], dtype=np.float64)
    new_low   = np.array([85.39], dtype=np.float64)
    new_close = np.array([86.53], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])  # continued bull_power
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.elderray.indicator([high, low, close], [5]);
    console.log('Bull power:', outputs[0]);
    console.log('Bear power:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.elderray.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [5]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued bull power:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.elderray.indicator([high, low, close], [5]);
    console.log('Bull power:', outputs[0]);
    console.log('Bear power:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.elderray.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [5]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued bull power:', continued[0]);
    ```

### Optional Outputs

`elderray` exposes 1 optional output: `ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

=== "Rust"

    ```rust
    use tulip_rs::indicators::elderray::{ElderRay, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let mask = [true];
    let (outputs, _state) = ElderRay::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[5.0],
        Some(&mask),
    ).unwrap();

    let bull_power = &outputs[0]; // bull_power (primary)
    let bear_power = &outputs[1]; // bear_power (primary)
    let ema  = &outputs[2]; // ema (optional — requested)
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[ELDERRAY_OPTIONS] = {5.0}; // period
    const double *inputs[ELDERRAY_INPUTS] = {high, low, close};
    bool optional_outputs[1] = {true}; // ema

    CIndicatorResult r = elderray_indicator(inputs, 10, options, optional_outputs, 1);
    /* r.outputs[0] -> bull_power (primary) */
    /* r.outputs[1] -> bear_power (primary) */
    /* r.outputs[2] -> ema (optional — requested) */
    tulip_ffi_result_free(r);
    elderray_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{5.0} // period
    mask := []bool{true} // ema

    res, st, _ := indicators.Elderray.Indicator(high, low, close, options, mask)
    bull_power := res.Rows[0] // bull_power (primary)
    bear_power := res.Rows[1] // bear_power (primary)
    ema  := res.Rows[2] // ema (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Elderray;

    // (high/low/close series as in the Basic tab)
    boolean[] mask = {true}; // ema
    Outcome oc = Elderray.indicator(new double[][] {high, low, close}, new double[] {5.0}, mask);
    try (Result res = oc.result()) {
        // row 0 = bull_power (primary), row 1 = bear_power (primary), row 2 = ema (optional — requested)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2))); // ema
    }
    oc.state().close();
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

    outputs, state = tulip_rs.indicators.elderray.indicator(
        [high, low, close], [5.0],
        optional_outputs=[True],
    )

    bull_power = outputs[0]  # bull_power (primary)
    bear_power = outputs[1]  # bear_power (primary)
    ema  = outputs[2]  # ema (optional — requested)
    ```

=== "Node.js"

    ```javascript
    const [allOut] = ti.elderray.indicator([high, low, close], [5], [true]);
    const bull_power = allOut[0]; // primary
    const bear_power = allOut[1]; // primary
    const ema  = allOut[2]; // optional 0: ema
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.elderray.indicator([high, low, close], [5], [true]);
    const bull_power = allOut[0]; // primary
    const bear_power = allOut[1]; // primary
    const ema  = allOut[2]; // optional 0: ema
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::elderray::{ElderRay, Indicator};

    let a1_high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                       85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let a1_low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let a1_close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                        85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let a2_high = vec![98.58, 98.27, 99.64, 99.96, 100.62,
                       100.68, 100.00, 101.16, 101.81, 102.00,
                       103.08, 103.90, 104.38, 105.60, 105.44];
    let a2_low  = vec![97.55, 96.77, 97.57, 99.18, 99.68,
                       99.73, 98.99, 98.76, 100.98, 100.93,
                       100.84, 102.47, 102.91, 104.60, 104.41];
    let a2_close = vec![97.91, 97.27, 99.44, 99.60, 100.33,
                        99.78, 99.41, 100.79, 101.46, 101.23,
                        102.64, 103.85, 104.27, 105.32, 104.75];

    let a3_high = vec![92.15, 91.89, 93.03, 93.30, 93.85,
                       93.90, 93.33, 94.30, 94.84, 95.00,
                       95.90, 96.58, 96.98, 98.00, 97.87];
    let a3_low  = vec![91.29, 90.64, 91.31, 92.65, 93.07,
                       93.11, 92.49, 92.30, 94.15, 94.11,
                       94.03, 95.39, 95.76, 97.17, 97.01];
    let a3_close = vec![91.59, 91.06, 92.87, 93.00, 93.61,
                        93.15, 92.84, 93.99, 94.55, 94.36,
                        95.53, 96.54, 96.89, 97.77, 97.29];

    let a4_high = vec![100.15, 99.89, 101.03, 101.30, 101.85,
                       101.90, 101.33, 102.30, 102.84, 103.00,
                       103.90, 104.58, 104.98, 106.00, 105.87];
    let a4_low  = vec![99.29, 98.64, 99.31, 100.65, 101.07,
                       101.11, 100.49, 100.30, 102.15, 102.11,
                       102.03, 103.39, 103.76, 105.17, 105.01];
    let a4_close = vec![99.59, 99.06, 100.87, 101.00, 101.61,
                        101.15, 100.84, 101.99, 102.55, 102.36,
                        103.53, 104.54, 104.89, 105.77, 105.29];

    let inputs: [&[&[f64]; 3]; 4] = [
        &[a1_high.as_slice(), a1_low.as_slice(), a1_close.as_slice()],
        &[a2_high.as_slice(), a2_low.as_slice(), a2_close.as_slice()],
        &[a3_high.as_slice(), a3_low.as_slice(), a3_close.as_slice()],
        &[a4_high.as_slice(), a4_low.as_slice(), a4_close.as_slice()],
    ];

    let results = ElderRay::indicator_by_assets::<4>(&inputs, &[5.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: bull_power={:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::elderray::{ElderRay, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[7.0], &[9.0], &[12.0]];
    let results = ElderRay::indicator_by_options::<4>(&[high.as_slice(), low.as_slice(), close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: bull_power={:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29};

    double h2[15], l2[15], c2[15];
    for (int i = 0; i < 15; i++) { h2[i] = h1[i] * 1.2; l2[i] = l1[i] * 1.2; c2[i] = c1[i] * 1.2; }
    double h3[15], l3[15], c3[15];
    for (int i = 0; i < 15; i++) { h3[i] = 90.0 + i * 0.5 + h1[i] * 0.1; l3[i] = 90.0 + i * 0.5 + l1[i] * 0.1; c3[i] = 90.0 + i * 0.5 + c1[i] * 0.1; }
    double h4[15], l4[15], c4[15];
    for (int i = 0; i < 15; i++) { h4[i] = 100.0 - i * 0.3 + h1[i] * 0.05; l4[i] = 100.0 - i * 0.3 + l1[i] * 0.05; c4[i] = 100.0 - i * 0.3 + c1[i] * 0.05; }

    const double *asset1[ELDERRAY_INPUTS] = {h1, l1, c1};
    const double *asset2[ELDERRAY_INPUTS] = {h2, l2, c2};
    const double *asset3[ELDERRAY_INPUTS] = {h3, l3, c3};
    const double *asset4[ELDERRAY_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    double options[ELDERRAY_OPTIONS] = {5.0};
    bool optional_outputs[1] = {false};

    CSimdResult r = elderray_simd_by_assets(simd_inputs, 4, 15, options, optional_outputs, 1);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's bull_power, length r.output_lens[i][0] */
        elderray_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    /* Tile the series 20x so longer-period option sets have enough data */
    #define EXPANDED_LEN (10 * 20)
    static double high_expanded[EXPANDED_LEN];
    static double low_expanded[EXPANDED_LEN];
    static double close_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 20; i++) {
        for (size_t j = 0; j < 10; j++) {
            high_expanded[i * 10 + j] = high[j];
            low_expanded[i * 10 + j]  = low[j];
            close_expanded[i * 10 + j] = close[j];
        }
    }
    const double *expanded_inputs[ELDERRAY_INPUTS] = {high_expanded, low_expanded, close_expanded};

    static const double o5[ELDERRAY_OPTIONS]   = {5.0};
    static const double o7[ELDERRAY_OPTIONS]  = {7.0};
    static const double o9[ELDERRAY_OPTIONS]  = {9.0};
    static const double o12[ELDERRAY_OPTIONS]  = {12.0};
    const double *const simd_opts[4] = {o5, o7, o9, o12};

    CSimdResult r = elderray_simd_by_options(expanded_inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> option set i's bull_power */
        elderray_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                    85.53, 86.54, 86.89, 87.77, 87.29}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, c2 := h1, l1, c1
    h3, l3, c3 := h1, l1, c1
    h4, l4, c4 := h1, l1, c1

    assets := [][indicators.ElderrayInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Elderray.SimdByAssets(assets, []float64{5.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d bull_power: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}

    periods := []float64{5.0, 7.0, 9.0, 12.0}
    optionSets := make([][]float64, len(periods))
    for i, p := range periods {
        optionSets[i] = []float64{p}
    }
    sim2, _ := indicators.Elderray.SimdByOptions(high, low, close, optionSets, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period %v bull_power: %v\n", periods[i], lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same period applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Elderray;

    // Each entry lists one asset's input series (h1..c4 as in the C tab).
    double[][][] assets = {{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}};
    try (SimdResult sim = Elderray.simdByAssets(assets, new double[] {5.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d bull_power: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Elderray;

    // (high/low/close series as in the Basic tab)
    try (SimdResult sim = Elderray.simdByOptions(new double[][] {high, low, close},
            new double[][] {{5.0}, {7.0}, {9.0}, {12.0}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period %d bull_power: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    a1_high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                        85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    a1_low = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)
    a1_close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                         85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    a2_high = a1_high * 1.2
    a2_low = a1_low * 1.2
    a2_close = a1_close * 1.2

    a3_high = 90.0 + np.arange(15) * 0.5 + a1_high * 0.1
    a3_low = 90.0 + np.arange(15) * 0.5 + a1_low * 0.1
    a3_close = 90.0 + np.arange(15) * 0.5 + a1_close * 0.1

    a4_high = 100.0 - np.arange(15) * 0.3 + a1_high * 0.05
    a4_low = 100.0 - np.arange(15) * 0.3 + a1_low * 0.05
    a4_close = 100.0 - np.arange(15) * 0.3 + a1_close * 0.05

    simd_inputs = [
        [a1_high, a1_low, a1_close],
        [a2_high, a2_low, a2_close],
        [a3_high, a3_low, a3_close],
        [a4_high, a4_low, a4_close],
    ]
    outputs_list, states = tulip_rs.indicators.elderray.simd_by_assets(simd_inputs, [5.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: bull_power={asset_outputs[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    simd_options = [[5.0], [7.0], [9.0], [12.0]]
    outputs_list, states = tulip_rs.indicators.elderray.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: bull_power={out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.2), low.map(v => v * 1.2), close.map(v => v * 1.2)],
        [Array.from({length: 15}, (_, i) => 90 + i * 0.5 + high[i] * 0.1),
         Array.from({length: 15}, (_, i) => 90 + i * 0.5 + low[i] * 0.1),
         Array.from({length: 15}, (_, i) => 90 + i * 0.5 + close[i] * 0.1)],
        [Array.from({length: 15}, (_, i) => 100 - i * 0.3 + high[i] * 0.05),
         Array.from({length: 15}, (_, i) => 100 - i * 0.3 + low[i] * 0.05),
         Array.from({length: 15}, (_, i) => 100 - i * 0.3 + close[i] * 0.05)],
    ];
    const [results] = ti.elderray.simdByAssets(simdInputs, [5]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}: bull_power=${out[0]}`));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [7], [9], [12]];
    const [results] = ti.elderray.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}: bull_power=${out[0]}`));
    ```
