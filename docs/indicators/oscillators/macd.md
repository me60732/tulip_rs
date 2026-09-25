# MACD — Moving Average Convergence Divergence

Shows the relationship between two EMAs of different periods. The histogram visualises the difference between the MACD line and its signal line, highlighting momentum shifts.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[fast_period, slow_period, signal_period]` &nbsp;|&nbsp; **Outputs:** `[macd_line, signal_line, histogram]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::macd::{Macd, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Options: [fast_period, slow_period, signal_period]
    let (outputs, _state) = Macd::indicator(&[close.as_slice()], &[12.0, 26.0, 9.0], None).unwrap();
    println!("MACD line:  {:?}", outputs[0]);
    println!("Signal:     {:?}", outputs[1]);
    println!("Histogram:  {:?}", outputs[2]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Macd::indicator(&[partial.as_slice()], &[12.0, 26.0, 9.0], None).unwrap();
    println!("Partial MACD: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued MACD:      {:?}", continued[0]);
    println!("Continued Signal:    {:?}", continued[1]);
    println!("Continued Histogram: {:?}", continued[2]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[MACD_INPUTS] = {close};
    double options[MACD_OPTIONS] = {12.0, 26.0, 9.0};

    /* Full computation with all optional outputs */
    bool optional_outputs[2] = {true, true};  /* short_ema, long_ema */
    CIndicatorResult r = macd_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> macd_line (primary) */
    /* r.outputs[1] -> signal_line (primary) */
    /* r.outputs[2] -> histogram (primary) */
    /* r.outputs[3] -> short_ema (optional) */
    /* r.outputs[4] -> long_ema (optional) */
    tulip_ffi_result_free(r);
    macd_state_free(r.state);

    /* Partial computation + state continuation (no optional outputs) */
    CIndicatorResult p = macd_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[MACD_INPUTS] = {new_close};
    CBatchResult b = macd_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> MACD line */
    /* b.outputs[1] -> Signal */
    /* b.outputs[2] -> Histogram */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    macd_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{12.0, 26.0, 9.0} // fastperiod,slowperiod,signalperiod

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Macd.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // MACD line values
    fmt.Println(res.Rows[1]) // Signal line values
    fmt.Println(res.Rows[2]) // Histogram values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Macd.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued MACD line values
    fmt.Println(batch.Rows[1]) // continued Signal line values
    fmt.Println(batch.Rows[2]) // continued Histogram values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Macd;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {12.0, 26.0, 9.0}; // fastperiod,slowperiod,signalperiod

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Macd.indicator(new double[][] {close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // MACD line values
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // Signal line values
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2))); // Histogram values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Macd.indicator(new double[][] {
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued MACD line values
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(1))); // continued Signal line values
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(2))); // continued Histogram values
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Options: [fast_period, slow_period, signal_period]
    outputs, state = tulip_rs.indicators.macd.indicator([close], [12.0, 26.0, 9.0])
    print("MACD line: ", outputs[0])
    print("Signal:    ", outputs[1])
    print("Histogram: ", outputs[2])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.macd.indicator([partial], [12.0, 26.0, 9.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued MACD:      ", continued[0])
    print("Continued Signal:    ", continued[1])
    print("Continued Histogram: ", continued[2])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.macd.indicator([close], [12, 26, 9]);
    console.log('MACD line:', outputs[0]);
    console.log('Signal:',    outputs[1]);
    console.log('Histogram:', outputs[2]);

    // State continuation
    const [, state2] = ti.macd.indicator([close.slice(0, -1)], [12, 26, 9]);
    const continued = state2.batchIndicator([close.slice(-1)]);
    console.log('Continued MACD:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.macd.indicator([close], [12, 26, 9]);
    console.log('MACD line:', outputs[0]);
    console.log('Signal:',    outputs[1]);
    console.log('Histogram:', outputs[2]);

    // State continuation
    const [, state2] = ti.macd.indicator([close.slice(0, -1)], [12, 26, 9]);
    const continued = state2.batchIndicator([close.slice(-1)]);
    console.log('Continued MACD:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `macd` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::macd::{Macd, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true]; // one per optional output
    let (outputs, _state) = Macd::indicator(&[close.as_slice()], &[12.0, 26.0, 9.0], Some(&mask)).unwrap();

    let macd_line   = &outputs[0]; // macd_line (primary)
    let signal_line = &outputs[1]; // signal_line (primary)
    let histogram   = &outputs[2]; // histogram (primary)
    let short_ema   = &outputs[3]; // short_ema (optional — requested)
    let long_ema    = &outputs[4]; // long_ema (optional — requested)
    ```

=== "C"

    `macd` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask in header order.

    ```c
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[MACD_INPUTS] = {close};
    double options[MACD_OPTIONS] = {12.0, 26.0, 9.0};

    bool mask[2] = {true, true};  /* one per optional output */
    CIndicatorResult r = macd_indicator(inputs, 10, options, mask, 2);
    /* r.outputs[0] -> macd_line (primary) */
    /* r.outputs[1] -> signal_line (primary) */
    /* r.outputs[2] -> histogram (primary) */
    /* r.outputs[3] -> short_ema (optional — requested) */
    /* r.outputs[4] -> long_ema (optional — requested) */
    tulip_ffi_result_free(r);
    macd_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{12.0, 26.0, 9.0} // fastperiod,slowperiod,signalperiod

    // Full computation with optional outputs — Rows are zero-copy views.
    res, st, _ := indicators.Macd.Indicator(close, options, []bool{true, true})
    fmt.Println(res.Rows[0]) // macd_line (primary)
    fmt.Println(res.Rows[1]) // signal_line (primary)
    fmt.Println(res.Rows[2]) // histogram (primary)
    fmt.Println(res.Rows[3]) // short_ema (optional — requested)
    fmt.Println(res.Rows[4]) // long_ema (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Java"

    `macd` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask as the third argument — one `boolean` per optional output, in order.

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Macd;

    // (close series and options as in the Basic tab)
    boolean[] mask = {true, true}; // short_ema, long_ema
    Outcome oc = Macd.indicator(new double[][] {close}, new double[] {12.0, 26.0, 9.0}, mask);
    try (Result res = oc.result()) {
        // row 0 = macd_line, row 1 = signal_line, row 2 = histogram;
        // rows 3 = short_ema, row 4 = long_ema (optional — requested)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // macd_line
    }
    oc.state().close();
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.macd.indicator(
        [close], [12.0, 26.0, 9.0],
        optional_outputs=[True, True],
    )

    macd_line   = outputs[0]  # macd_line (primary)
    signal_line = outputs[1]  # signal_line (primary)
    histogram   = outputs[2]  # histogram (primary)
    short_ema   = outputs[3]  # short_ema (optional — requested)
    long_ema    = outputs[4]  # long_ema (optional — requested)
    ```

=== "Node.js"

    `macd` exposes 2 optional outputs: `short_ema`, `long_ema`.

    ```javascript
    const [allOut] = ti.macd.indicator([close], [12, 26, 9], [true, true]);
    const macdLine = allOut[0]; // primary
    const signal   = allOut[1]; // primary
    const hist     = allOut[2]; // primary
    const shortEma = allOut[3]; // optional 0: short_ema
    const longEma  = allOut[4]; // optional 1: long_ema
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.macd.indicator([close], [12, 26, 9], [true, true]);
    const macdLine = allOut[0]; // primary
    const signal   = allOut[1]; // primary
    const hist     = allOut[2]; // primary
    const shortEma = allOut[3]; // optional 0: short_ema
    const longEma  = allOut[4]; // optional 1: long_ema
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::macd::{Macd, Indicator};

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

    let results = Macd::indicator_by_assets::<4>(&inputs, &[12.0, 26.0, 9.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} MACD: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Signal: {:?}", i + 1, asset_outputs[1]);
        println!("Asset {} Histogram: {:?}", i + 1, asset_outputs[2]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::macd::{Macd, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[6.0,  13.0,  5.0],
        &[12.0, 26.0,  9.0],
        &[19.0, 39.0, 14.0],
        &[24.0, 52.0, 18.0],
    ];

    let results = Macd::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Option set {} MACD:      {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Signal:    {:?}", i + 1, opt_outputs[1]);
        println!("Option set {} Histogram: {:?}", i + 1, opt_outputs[2]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    const double *const asset1[MACD_INPUTS] = {a1};
    const double *const asset2[MACD_INPUTS] = {a2};
    const double *const asset3[MACD_INPUTS] = {a3};
    const double *const asset4[MACD_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = macd_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> MACD line */
        /* r.outputs[i][1] -> Signal */
        /* r.outputs[i][2] -> Histogram */
        macd_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in one call:

    ```c
    double o12[] = {12.0, 26.0, 9.0}, o19[] = {19.0, 39.0, 14.0};
    double o24[] = {24.0, 52.0, 18.0}, o6[] = {6.0, 13.0, 5.0};
    const double *const simd_opts[4] = {o12, o19, o24, o6};

    CSimdResult r = macd_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i][0] -> MACD line */
    /* r.outputs[i][1] -> Signal */
    /* r.outputs[i][2] -> Histogram */
    for (uintptr_t i = 0; i < r.num_results; i++) macd_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 2 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for asset 2 in this example
    a2 := a1

    assets := [][indicators.MacdInputs][]float64{{a1}, {a2}}
    sim, _ := indicators.Macd.SimdByAssets(assets, []float64{12.0, 26.0, 9.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d MACD: %v\n", i+1, lanes[0])
        fmt.Printf("Asset %d Signal: %v\n", i+1, lanes[1])
        fmt.Printf("Asset %d Histogram: %v\n", i+1, lanes[2])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Macd.SimdByOptions(close, [][]float64{{10, 20, 5}, {12, 26, 9}, {15, 30, 8}, {20, 40, 10}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Option set %d MACD: %v\n", i+1, lanes[0])
        fmt.Printf("Option set %d Signal: %v\n", i+1, lanes[1])
        fmt.Printf("Option set %d Histogram: %v\n", i+1, lanes[2])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same options applied to 2 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Macd;

    // One entry per asset; each asset lists its INPUTS series.
    double[] a1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double[] a2 = a1; // reuse the same data for asset 2

    double[][][] assets = {{a1}, {a2}};
    try (SimdResult sim = Macd.simdByAssets(assets, new double[] {12.0, 26.0, 9.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d MACD: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
            System.out.printf("Asset %d Signal: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1)));
            System.out.printf("Asset %d Histogram: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 2)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Macd;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    try (SimdResult sim = Macd.simdByOptions(new double[][] {close},
            new double[][] {{10, 20, 5}, {12, 26, 9}, {15, 30, 8}, {20, 40, 10}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Option set %d MACD: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
            System.out.printf("Option set %d Signal: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1)));
            System.out.printf("Option set %d Histogram: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 2)));
        }
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.macd.simd_by_assets(simd_inputs, [12.0, 26.0, 9.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} MACD:      {out[0]}")
        print(f"Asset {i + 1} Signal:    {out[1]}")
        print(f"Asset {i + 1} Histogram: {out[2]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [6.0,  13.0,  5.0],
        [12.0, 26.0,  9.0],
        [19.0, 39.0, 14.0],
        [24.0, 52.0, 18.0],
    ]
    outputs_list, states = tulip_rs.indicators.macd.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} MACD:      {out[0]}")
        print(f"Option set {i + 1} Signal:    {out[1]}")
        print(f"Option set {i + 1} Histogram: {out[2]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.macd.simdByAssets(simdInputs, [12, 26, 9]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} MACD:`, out[0], 'Signal:', out[1]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[6, 13, 5], [12, 26, 9], [19, 39, 14], [24, 52, 18]];
    const [results] = ti.macd.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} MACD:`, out[0]));
    ```
