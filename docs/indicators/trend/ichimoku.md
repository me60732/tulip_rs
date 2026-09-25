# Ichimoku — Ichimoku Kinkō Hyō

A comprehensive trend-following system that defines support/resistance, trend direction, and momentum in a single glance; the conversion and base lines act like short- and long-period midpoint averages, while the leading spans project a "cloud" forward by `long_period` bars.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[short_period, long_period]` &nbsp;|&nbsp; **Outputs:** `[conversion, base, leading_span_a, leading_span_b]` &nbsp;|&nbsp; **Optional:** `[lagging_span]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::ichimoku::{Ichimoku, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                     89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                     91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                     88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                     90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    // options: [short_period, long_period]
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = Ichimoku::indicator(&inputs, &[9.0, 26.0], None).unwrap();
    println!("Conversion:     {:?}", outputs[0]);
    println!("Base:           {:?}", outputs[1]);
    println!("Leading Span A: {:?}", outputs[2]);
    println!("Leading Span B: {:?}", outputs[3]);

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = Ichimoku::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[9.0, 26.0], None).unwrap();
    println!("Conversion:     {:?}", outputs2[0]);

    let new_high   = vec![85.90_f64];
    let new_low    = vec![84.03_f64];
    let new_close  = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()], None).unwrap();
    println!("Continued Conversion: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                     89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                     91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                    88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                    90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[ICHIMOKU_OPTIONS] = {9.0, 26.0}; // short_period, long_period
    const double *inputs[ICHIMOKU_INPUTS] = {high, low, close};

    /* Full computation */
    CIndicatorResult r = ichimoku_indicator(inputs, 40, options, NULL, 0);
    tulip_ffi_result_free(r);
    ichimoku_state_free(r.state);

    /* Partial + continuation */
    CIndicatorResult p = ichimoku_indicator(inputs, 35, options, NULL, 0);
    const double *rest_inputs[ICHIMOKU_INPUTS] = {high + 35, low + 35, close + 35};
    CBatchResult b = ichimoku_batch(p.state, rest_inputs, 5, NULL, 0);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    ichimoku_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high  := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                      89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                      91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10}
    low   := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                      88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                      90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{9.0, 26.0} // short_period, long_period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Ichimoku.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // conversion
    fmt.Println(res.Rows[1]) // base
    fmt.Println(res.Rows[2]) // leading_span_a
    fmt.Println(res.Rows[3]) // leading_span_b
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Ichimoku.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued conversion
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Ichimoku;

    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                      89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                      91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                      88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                      90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double[] options = {9.0, 26.0}; // short_period, long_period

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Ichimoku.indicator(new double[][] {high, low, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // conversion
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // base
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2))); // leading_span_a
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(3))); // leading_span_b
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Ichimoku.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10),
            java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued conversion
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                      89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                      91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                      88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                      90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.ichimoku.indicator([high, low, close], [9.0, 26.0])
    print("Conversion:    ", outputs[0])
    print("Base:          ", outputs[1])
    print("Leading Span A:", outputs[2])
    print("Leading Span B:", outputs[3])

    # State continuation
    n = len(high) - 5
    outputs2, state = tulip_rs.indicators.ichimoku.indicator(
        [high[:n], low[:n], close[:n]], [9.0, 26.0]
    )
    print("Partial Conversion:", outputs2[0])

    continued = state.batch_indicator([high[n:], low[n:], close[n:]])
    print("Continued Conversion:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                                     85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                                     89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                                     91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                                     84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                                     88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                                     90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20]);

    // options: [short_period, long_period]
    const [outputs, state] = ti.ichimoku.indicator([high, low, close], [9, 26]);
    console.log('Conversion:    ', outputs[0]);
    console.log('Base:          ', outputs[1]);
    console.log('Leading Span A:', outputs[2]);
    console.log('Leading Span B:', outputs[3]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.ichimoku.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [9, 26]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Conversion:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                   89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                   91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                   88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                   90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20];

    const [outputs, state] = ti.ichimoku.indicator([high, low, close], [9, 26]);
    console.log('Conversion:    ', outputs[0]);
    console.log('Base:          ', outputs[1]);
    console.log('Leading Span A:', outputs[2]);
    console.log('Leading Span B:', outputs[3]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.ichimoku.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [9, 26]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Conversion:', continued[0]);
    ```

### Optional Outputs

`ichimoku` exposes 1 optional output: `lagging_span`. The lagging span is the close price shifted back by `long_period` bars, useful for confirming trend signals against historical price. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

=== "Rust"

    ```rust
    use tulip_rs::indicators::ichimoku::{Ichimoku, Indicator, TIndicatorState};

    // ... (same high, low, close data as above)
    let mask = [true];
    let (outputs, _state) = Ichimoku::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[9.0, 26.0],
        Some(&mask),
    ).unwrap();

    let conversion   = &outputs[0]; // conversion (primary)
    let base         = &outputs[1]; // base (primary)
    let leading_a    = &outputs[2]; // leading_span_a (primary)
    let leading_b    = &outputs[3]; // leading_span_b (primary)
    let lagging_span = &outputs[4]; // lagging_span (optional — requested)
    ```

=== "C"

    `ichimoku` exposes 1 optional output: `lagging_span`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    // ... (same high, low, close data as above)
    bool optional_outputs[1] = {true}; // lagging_span

    CIndicatorResult r = ichimoku_indicator(inputs, 40, options, optional_outputs, 1);
    /* r.outputs[0..3] -> primary outputs (conversion, base, leading_a, leading_b) */
    /* r.outputs[4]     -> lagging_span (optional — requested) */
    tulip_ffi_result_free(r);
    ichimoku_state_free(r.state);
    ```

=== "Go"

    `ichimoku` exposes 1 optional output: `lagging_span`. Pass a boolean mask as the third argument.

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    // ... (same high, low, close data as above)
    mask := []bool{true} // lagging_span

    res, st, _ := indicators.Ichimoku.Indicator(high, low, close, options, mask)
    conversion   := res.Rows[0] // conversion (primary)
    base         := res.Rows[1] // base (primary)
    leadingA     := res.Rows[2] // leading_span_a (primary)
    leadingB     := res.Rows[3] // leading_span_b (primary)
    laggingSpan  := res.Rows[4] // lagging_span (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Java"

    `ichimoku` exposes 1 optional output: `lagging_span`. Pass a boolean mask as the third argument.

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Ichimoku;

    // ... (same high, low, close data as above)
    double[] options = {9.0, 26.0};
    boolean[] mask = {true}; // lagging_span

    Outcome oc = Ichimoku.indicator(new double[][] {high, low, close}, options, mask);
    try (Result res = oc.result()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // conversion (primary)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // base (primary)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2))); // leading_span_a (primary)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(3))); // leading_span_b (primary)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(4))); // lagging_span (optional — requested)
    }
    oc.state().close();
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    # ... (same high, low, close data as above)
    outputs, state = tulip_rs.indicators.ichimoku.indicator(
        [high, low, close], [9.0, 26.0],
        optional_outputs=[True],
    )

    conversion   = outputs[0]  # conversion (primary)
    base         = outputs[1]  # base (primary)
    leading_a    = outputs[2]  # leading_span_a (primary)
    leading_b    = outputs[3]  # leading_span_b (primary)
    lagging_span = outputs[4]  # lagging_span (optional — requested)
    ```

=== "Node.js"

    `ichimoku` exposes 1 optional output: `lagging_span`.

    ```javascript
    const [allOut] = ti.ichimoku.indicator([high, low, close], [9, 26], [true]);
    const conversion  = allOut[0]; // primary
    const base        = allOut[1]; // primary
    const leadingA    = allOut[2]; // primary
    const leadingB    = allOut[3]; // primary
    const laggingSpan = allOut[4]; // optional 0: lagging_span
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.ichimoku.indicator([high, low, close], [9, 26], [true]);
    const conversion  = allOut[0]; // primary
    const base        = allOut[1]; // primary
    const leadingA    = allOut[2]; // primary
    const leadingB    = allOut[3]; // primary
    const laggingSpan = allOut[4]; // optional 0: lagging_span
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ichimoku::{Ichimoku, Indicator};

    let h1 = high.clone(); let l1 = low.clone(); let c1 = close.clone();
    let h2 = h1.clone();   let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone();   let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone();   let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = Ichimoku::indicator_by_assets::<4>(&inputs, &[9.0, 26.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Conversion: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Base:       {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::ichimoku::{Ichimoku, IndicatorByOptions};

    let opts: [&[f64; 2]; 4] = [&[7.0, 22.0], &[9.0, 26.0], &[11.0, 30.0], &[13.0, 34.0]];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = Ichimoku::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Short/Long {}/{}: Conversion={:?}", opts[i][0], opts[i][1], out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in parallel:

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                   89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                   91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                   88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                   90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};

    double h2[40], l2[40], c2[40];
    for (int i = 0; i < 40; i++) { h2[i] = h1[i] * 1.2; l2[i] = l1[i] * 1.2; c2[i] = c1[i] * 1.2; }
    double h3[40], l3[40], c3[40];
    for (int i = 0; i < 40; i++) { h3[i] = 90.0 + i * 0.5 + h1[i] * 0.1; l3[i] = 90.0 + i * 0.5 + l1[i] * 0.1; c3[i] = 90.0 + i * 0.5 + c1[i] * 0.1; }
    double h4[40], l4[40], c4[40];
    for (int i = 0; i < 40; i++) { h4[i] = 100.0 - i * 0.3 + h1[i] * 0.05; l4[i] = 100.0 - i * 0.3 + l1[i] * 0.05; c4[i] = 100.0 - i * 0.3 + c1[i] * 0.05; }

    const double *asset1[ICHIMOKU_INPUTS] = {h1, l1, c1};
    const double *asset2[ICHIMOKU_INPUTS] = {h2, l2, c2};
    const double *asset3[ICHIMOKU_INPUTS] = {h3, l3, c3};
    const double *asset4[ICHIMOKU_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = ichimoku_simd_by_assets(simd_inputs, 4, 40, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        ichimoku_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```c
    double high_expanded[400], low_expanded[400], close_expanded[400];
    for (int i = 0; i < 10; i++) {
        for (int j = 0; j < 40; j++) {
            high_expanded[i * 40 + j] = h1[j];
            low_expanded[i * 40 + j] = l1[j];
            close_expanded[i * 40 + j] = c1[j];
        }
    }
    const double *expanded_inputs[ICHIMOKU_INPUTS] = {high_expanded, low_expanded, close_expanded};

    double o5_10[] = {5.0, 10.0}, o7_14[] = {7.0, 14.0}, o9_18[] = {9.0, 18.0}, o9_26[] = {9.0, 26.0};
    const double *const simd_opts[4] = {o5_10, o7_14, o9_18, o9_26};

    CSimdResult r = ichimoku_simd_by_options(expanded_inputs, 400, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        ichimoku_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                    89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                    91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                    88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                    90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                    85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                    88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                    90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{9.0, 26.0}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, c2 := h1, l1, c1
    h3, l3, c3 := h1, l1, c1
    h4, l4, c4 := h1, l1, c1

    assets := [][indicators.IchimokuInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Ichimoku.SimdByAssets(assets, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d Conversion: %v\n", i+1, lanes[0])
        fmt.Printf("Asset %d Base:       %v\n", i+1, lanes[1])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                      89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                      91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                     88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                     90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}

    // Tile to ensure longer-period option sets have enough data.
    tiledLen := len(high) * 3
    highTiled := make([]float64, tiledLen)
    lowTiled := make([]float64, tiledLen)
    closeTiled := make([]float64, tiledLen)
    for i := 0; i < 3; i++ {
        copy(highTiled[i*len(high):(i+1)*len(high)], high)
        copy(lowTiled[i*len(high):(i+1)*len(high)], low)
        copy(closeTiled[i*len(high):(i+1)*len(high)], close)
    }

    optionSets := [][]float64{{5.0, 10.0}, {7.0, 14.0}, {9.0, 18.0}, {9.0, 26.0}}
    sim2, _ := indicators.Ichimoku.SimdByOptions(highTiled, lowTiled, closeTiled, optionSets, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Short/Long %v/%v: Conversion=%v\n", optionSets[i][0], optionSets[i][1], lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Ichimoku;

    double[] h1 = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                   89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                   91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10};
    double[] l1 = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                   88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                   90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00};
    double[] c1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double[] options = {9.0, 26.0};

    // Reuse the same data for assets 2–4 in this example.
    double[] h2 = h1, l2 = l1, c2 = c1;
    double[] h3 = h1, l3 = l1, c3 = c1;
    double[] h4 = h1, l4 = l1, c4 = c1;

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}};
    try (SimdResult sim = Ichimoku.simdByAssets(assets, options, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d Conversion: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
            System.out.printf("Asset %d Base:       %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Ichimoku;

    double[] high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                     89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                     91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10};
    double[] low = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                    88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                    90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};

    // Tile to ensure longer-period option sets have enough data.
    int tiledLen = high.length * 3;
    double[] highTiled = new double[tiledLen];
    double[] lowTiled = new double[tiledLen];
    double[] closeTiled = new double[tiledLen];
    for (int i = 0; i < 3; i++) {
        System.arraycopy(high, 0, highTiled, i * high.length, high.length);
        System.arraycopy(low, 0, lowTiled, i * high.length, high.length);
        System.arraycopy(close, 0, closeTiled, i * high.length, high.length);
    }

    double[][] optionSets = {{5.0, 10.0}, {7.0, 14.0}, {9.0, 18.0}, {9.0, 26.0}};
    try (SimdResult sim = Ichimoku.simdByOptions(new double[][] {highTiled, lowTiled, closeTiled},
            optionSets, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Short/Long %v/%v: Conversion=%s%n", optionSets[i][0], optionSets[i][1],
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    simd_inputs = [
        [high,        low,        close],
        [high + 0.5,  low + 0.5,  close + 0.5],
        [high - 0.5,  low - 0.5,  close - 0.5],
        [high * 1.01, low * 1.01, close * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.ichimoku.simd_by_assets(simd_inputs, [9.0, 26.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Conversion: {out[0]}")
        print(f"Asset {i + 1} Base:       {out[1]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    simd_options = [[7.0, 22.0], [9.0, 26.0], [11.0, 30.0], [13.0, 34.0]]
    outputs_list, states = tulip_rs.indicators.ichimoku.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Short/Long {simd_options[i][0]}/{simd_options[i][1]}: {out[0]}")
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
    const [results] = ti.ichimoku.simdByAssets(simdInputs, [9, 26]);
    results.forEach((out, i) => {
        console.log(`Asset ${i + 1} Conversion:`, out[0]);
        console.log(`Asset ${i + 1} Base:      `, out[1]);
    });
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[7, 22], [9, 26], [11, 30], [13, 34]];
    const [results] = ti.ichimoku.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Short/Long ${simdOptions[i][0]}/${simdOptions[i][1]}:`, out[0]));
    ```
