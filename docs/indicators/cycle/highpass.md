# Highpass — Ehlers High Pass Filter

Removes low-frequency trend components from price by applying Ehlers' two-pole high-pass filter; the period controls the cutoff frequency.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[highpass]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::highpass::{Highpass, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    let (outputs, _state) = Highpass::indicator(&[close.as_slice()], &[20.0], None).unwrap();
    println!("Highpass(20): {:?}", outputs[0]);

    // State continuation
    let n = close.len() - 5;
    let partial = close[..n].to_vec();
    let (outputs2, mut state) = Highpass::indicator(&[partial.as_slice()], &[20.0], None).unwrap();
    println!("Partial Highpass: {:?}", outputs2[0]);

    let rest = close[n..].to_vec();
    let continued = state.batch_indicator(&[rest.as_slice()], None).unwrap();
    println!("Continued Highpass: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[HIGHPASS_OPTIONS] = {20.0}; // period
    const double *inputs[HIGHPASS_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = highpass_indicator(inputs, 40, options, NULL, 0);
    /* r.outputs[0] -> the highpass series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    highpass_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = highpass_indicator(inputs, 35, options, NULL, 0);
    double new_close[] = {89.70, 90.10, 89.50, 90.20, 90.80};
    const double *new_inputs[HIGHPASS_INPUTS] = {new_close};
    CBatchResult b = highpass_batch(p.state, new_inputs, 5, NULL, 0);
    /* b.outputs[0] -> highpass values for just the five new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    highpass_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{20.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Highpass.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // highpass values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Highpass.Indicator(close[:35], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[35:], nil)
    fmt.Println(batch.Rows[0]) // continued highpass values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Highpass;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double[] options = {20.0}; // period

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Highpass.indicator(new double[][] {close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // highpass values
    }

    // Partial computation + state continuation.
    int n = 35;
    Outcome p = Highpass.indicator(
        new double[][] {java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {java.util.Arrays.copyOfRange(close, n, 40)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued highpass values
        }
    }
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
    ], dtype=np.float64)

    outputs, state = tulip_rs.indicators.highpass.indicator([close], [20.0])
    print("Highpass(20):", outputs[0])

    # State continuation
    partial = close[:-5]
    outputs2, state = tulip_rs.indicators.highpass.indicator([partial], [20.0])
    rest = close[-5:]
    continued = state.batch_indicator([rest])
    print("Continued Highpass:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ]);

    const [outputs, state] = ti.highpass.indicator([close], [20]);
    console.log('Highpass(20):', outputs[0]);

    // State continuation
    const [, state2] = ti.highpass.indicator([close.slice(0, -5)], [20]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Highpass:', continued[0]);
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
    ];

    const [outputs, state] = ti.highpass.indicator([close], [20]);
    console.log('Highpass(20):', outputs[0]);

    // State continuation
    const [, state2] = ti.highpass.indicator([close.slice(0, -5)], [20]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Highpass:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::highpass::{Highpass, Indicator, TIndicatorState};

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

    let results = Highpass::indicator_by_assets::<4>(&inputs, &[20.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::highpass::{Highpass, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[10.0], &[20.0], &[30.0], &[40.0]];

    let results = Highpass::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[HIGHPASS_INPUTS] = {a1};
    const double *asset2[HIGHPASS_INPUTS] = {a2};
    const double *asset3[HIGHPASS_INPUTS] = {a3};
    const double *asset4[HIGHPASS_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = highpass_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        highpass_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    static const double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                                   83.15, 82.84, 83.99, 84.55, 84.36};
    #define EXPANDED_LEN (sizeof(close) / sizeof(double) * 10)
    static double close_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 10; i++) {
        for (size_t j = 0; j < sizeof(close) / sizeof(double); j++) {
            close_expanded[i * (sizeof(close) / sizeof(double)) + j] = close[j];
        }
    }
    const double *inputs[HIGHPASS_INPUTS] = {close_expanded};

    static const double o1[HIGHPASS_OPTIONS] = {10.0};
    static const double o2[HIGHPASS_OPTIONS] = {20.0};
    static const double o3[HIGHPASS_OPTIONS] = {30.0};
    static const double o4[HIGHPASS_OPTIONS] = {40.0};
    const double *const simd_opts[4] = {o1, o2, o3, o4};

    CSimdResult r = highpass_simd_by_options(inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        highpass_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := a1, a1, a1
    options := []float64{20.0} // period

    assets := [][indicators.HighpassInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Highpass.SimdByAssets(assets, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := [][]float64{{10}, {20}, {30}, {40}}

    sim, _ := indicators.Highpass.SimdByOptions(close, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim.Close()
    ```

=== "Java"

    **By assets** — same period applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Highpass;

    double[] a1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double[] a2 = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double[] a3 = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double[] a4 = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1}, {a2}, {a3}, {a4}};
    try (SimdResult sim = Highpass.simdByAssets(assets, new double[] {20.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));}
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Highpass;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    try (SimdResult sim = Highpass.simdByOptions(new double[][] {close},
            new double[][] {{10}, {20}, {30}, {40}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period set %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 3.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.highpass.simd_by_assets(simd_inputs, [20.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    simd_options = [[10.0], [20.0], [30.0], [40.0]]
    outputs_list, states = tulip_rs.indicators.highpass.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v + 5.0)],
        [close.map(v => v - 3.0)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.highpass.simdByAssets(simdInputs, [20]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[10], [20], [30], [40]];
    const [results] = ti.highpass.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
