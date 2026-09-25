# QStick — `qstick`

A moving average of `(Close - Open)` over `period` bars, summarising buying or selling pressure.

**Inputs:** `[open, close]` | **Options:** `[period]` | **Outputs:** `[qstick]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::qstick::{QStick, Indicator, TIndicatorState};

    let open_ = vec![81.85, 81.20, 81.55, 82.91, 83.10,
                     83.41, 82.71, 82.70, 84.20, 84.25_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [open_.as_slice(), close.as_slice()];
    let (outputs, mut state) = QStick::indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial_open  = open_[..8].to_vec();
    let partial_close = close[..8].to_vec();
    let (outputs2, mut state) = QStick::indicator(&[partial_open.as_slice(), partial_close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_open  = vec![84.03_f64];
    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_open.as_slice(), new_close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double open_[] = {81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double options[QSTICK_OPTIONS] = {5.0}; // period
    const double *inputs[QSTICK_INPUTS] = {open_, close};

    /* Full computation */
    CIndicatorResult r = qstick_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> QStick(5) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    qstick_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = qstick_indicator(inputs, 8, options, NULL, 0);
    double new_open[]  = {84.03};
    double new_close[] = {85.53};
    const double *new_inputs[QSTICK_INPUTS] = {new_open, new_close};
    CBatchResult b = qstick_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> QStick for the new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    qstick_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    open := []float64{81.85, 81.20, 81.55, 82.91, 83.10,
                      83.41, 82.71, 82.70, 84.20, 84.25}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{5.0}

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Qstick.Indicator(open, close, options, nil)
    fmt.Println(res.Rows[0]) // QStick(5) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Qstick.Indicator(open[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(open[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued QStick values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Qstick;

    double[] open_ = {81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {5.0};

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Qstick.indicator(new double[][] {open_, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // QStick(5) values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Qstick.indicator(new double[][] {
        java.util.Arrays.copyOfRange(open_, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(open_, n, 10),
            java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued QStick values
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    open_  = np.array([81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.qstick.indicator([open_, close], [14.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.qstick.indicator([open_, close], [14]);
    console.log('QStick(14):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.qstick.indicator([open_.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([open_.slice(n), close.slice(n)]);
    console.log('Continued QStick:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.qstick.indicator([open_, close], [14]);
    console.log('QStick(14):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.qstick.indicator([open_.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([open_.slice(n), close.slice(n)]);
    console.log('Continued QStick:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::qstick::{QStick, Indicator};

    let inputs: [&[&[f64]; 2]; 4] = [
        &[o1.as_slice(), c1.as_slice()],
        &[o2.as_slice(), c2.as_slice()],
        &[o3.as_slice(), c3.as_slice()],
        &[o4.as_slice(), c4.as_slice()],
    ];
    let results = QStick::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::qstick::{QStick, IndicatorByOptions};

    let inputs_single = [open_.as_slice(), close.as_slice()];
    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let results = QStick::indicator_by_options::<4>(&inputs_single, &opts, None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Option {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1_open[] = {81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25};
    double a1_close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    const double *const asset1[QSTICK_INPUTS] = {a1_open, a1_close};

    // Asset 2: scaled up (+20%)
    double a2_open[10], a2_close[10];
    for (size_t i = 0; i < 10; i++) {
        a2_open[i] = a1_open[i] * 1.2;
        a2_close[i] = a1_close[i] * 1.2;
    }
    const double *const asset2[QSTICK_INPUTS] = {a2_open, a2_close};

    // Asset 3: different upward trend
    double a3_open[10], a3_close[10];
    for (size_t i = 0; i < 10; i++) {
        a3_open[i] = 90.0 + (double)i * 0.5 + a1_open[i] * 0.1;
        a3_close[i] = 90.0 + (double)i * 0.5 + a1_close[i] * 0.1;
    }
    const double *const asset3[QSTICK_INPUTS] = {a3_open, a3_close};

    // Asset 4: downward trend
    double a4_open[10], a4_close[10];
    for (size_t i = 0; i < 10; i++) {
        a4_open[i] = 100.0 - (double)i * 0.3 + a1_open[i] * 0.05;
        a4_close[i] = 100.0 - (double)i * 0.3 + a1_close[i] * 0.05;
    }
    const double *const asset4[QSTICK_INPUTS] = {a4_open, a4_close};

    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = qstick_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        qstick_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o3[] = {3.0}, o5[] = {5.0}, o7[] = {7.0}, o10[] = {10.0};
    const double *const simd_opts[4] = {o3, o5, o7, o10};

    CSimdResult r = qstick_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i (periods 3/5/7/10) */
    for (uintptr_t i = 0; i < r.num_results; i++) qstick_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1_open := []float64{81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25}
    a1_close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    a2_open, a2_close := a1_open, a1_close
    a3_open, a3_close := a1_open, a1_close
    a4_open, a4_close := a1_open, a1_close

    assets := [][indicators.QstickInputs][]float64{{a1_open, a1_close}, {a2_open, a2_close},
                                                    {a3_open, a3_close}, {a4_open, a4_close}}
    sim, _ := indicators.Qstick.SimdByAssets(assets, []float64{5.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    open_ := []float64{81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    sim2, _ := indicators.Qstick.SimdByOptions(open_, close, [][]float64{{5}, {10}, {14}, {20}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Qstick;

    double[] a1_open = {81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25};
    double[] a1_close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    // Reuse the same data for assets 2-4 in this example
    double[] a2_open = a1_open;
    double[] a2_close = a1_close;
    double[] a3_open = a1_open;
    double[] a3_close = a1_close;
    double[] a4_open = a1_open;
    double[] a4_close = a1_close;

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1_open, a1_close}, {a2_open, a2_close},
                           {a3_open, a3_close}, {a4_open, a4_close}};
    try (SimdResult sim = Qstick.simdByAssets(assets, new double[] {5.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Qstick;

    double[] open_ = {81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    try (SimdResult sim = Qstick.simdByOptions(new double[][] {open_, close},
            new double[][] {{5}, {10}, {14}, {20}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period set %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[o1, c1], [o2, c2], [o3, c3], [o4, c4]]
    outputs_list, states = tulip_rs.indicators.qstick.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.qstick.simd_by_options([open_, close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...open_], close.slice()],
        [open_.map(v => v * 1.1), close.map(v => v * 1.1)],
        [open_.map(v => v * 0.9), close.map(v => v * 0.9)],
        [open_.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.qstick.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.qstick.simdByOptions([open_, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
