# Pivot Point — `pivotpoint`

Classic floor-trader pivot points calculated from the previous bar's high, low, and close. Provides a central pivot level plus two support and two resistance levels.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[s3, s2, s1, pivot, r1, r2, r3]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::pivotpoint::{PivotPoint, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    // options: [period]
    let (outputs, mut state) = PivotPoint::indicator(&inputs, &[5.0], None).unwrap();

    println!("Pivot: {:?}", outputs[0]);
    println!("R1:    {:?}", outputs[1]);
    println!("S1:    {:?}", outputs[2]);
    println!("R2:    {:?}", outputs[3]);
    println!("S2:    {:?}", outputs[4]);

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = PivotPoint::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[5.0], None).unwrap();

    println!("Pivot: {:?}", outputs2[0]);
    println!("R1:    {:?}", outputs2[1]);
    println!("S1:    {:?}", outputs2[2]);

    let new_high   = vec![85.90_f64];
    let new_low    = vec![84.03_f64];
    let new_close  = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()], None).unwrap();
    println!("Continued Pivot: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    double options[PIVOTPOINT_OPTIONS] = {5.0}; // period
    const double *inputs[PIVOTPOINT_INPUTS] = {high, low, close};

    /* Full computation - one output row with 7 values: s3,s2,s1,pp,r1,r2,r3 */
    CIndicatorResult r = pivotpoint_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0][i] -> the i-th value in the single output row */
    tulip_ffi_result_free(r);
    pivotpoint_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = pivotpoint_indicator(inputs, 8, options, NULL, 0);
    double new_high[]   = {85.90};
    double new_low[]    = {84.03};
    double new_close[]  = {85.53};
    const double *new_inputs[PIVOTPOINT_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = pivotpoint_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0][i] -> continued values for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    pivotpoint_state_free(p.state);
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
    options := []float64{5.0}

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Pivotpoint.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // s3,s2,s1,pp,r1,r2,r3 values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Pivotpoint.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued pivot values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Pivotpoint;

    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {5.0};

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Pivotpoint.indicator(new double[][] {high, low, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // s3,s2,s1,pp,r1,r2,r3 values
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1)));
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2)));
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(3)));
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(4)));
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Pivotpoint.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10),
            java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued pivot values
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    // options: [period]
    outputs, state = tulip_rs.indicators.pivotpoint.indicator([high, low, close], [5.0])

    print(f"Pivot: {outputs[0]}")
    print(f"R1:    {outputs[1]}")
    print(f"S1:    {outputs[2]}")
    print(f"R2:    {outputs[3]}")
    print(f"S2:    {outputs[4]}")
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.pivotpoint.indicator([high, low, close], [5.0]);
    console.log('S3:', outputs[0], 'S2:', outputs[1], 'S1:', outputs[2], 'Pivot:', outputs[3], 'R1:', outputs[4]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.pivotpoint.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [5.0]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Pivot:', continued[3]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.pivotpoint.indicator([high, low, close], [5.0]);
    console.log('S3:', outputs[0], 'S2:', outputs[1], 'S1:', outputs[2], 'Pivot:', outputs[3], 'R1:', outputs[4]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.pivotpoint.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [5.0]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Pivot:', continued[3]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```rust
    use tulip_rs::indicators::pivotpoint::{PivotPoint, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = PivotPoint::indicator_by_assets::<4>(&inputs, &[5.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::pivotpoint::{PivotPoint, IndicatorByOptions};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let options: [&[f64]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let results = PivotPoint::indicator_by_options::<4>(&inputs, &options, None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel (N must be 2/4/8/16):

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    // Reuse the same data for assets 2-4 in this example
    double h2[] = h1;
    double l2[] = l1;
    double c2[] = c1;
    double h3[] = h1;
    double l3[] = l1;
    double c3[] = c1;
    double h4[] = h1;
    double l4[] = l1;
    double c4[] = c1;

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[PIVOTPOINT_INPUTS] = {h1, l1, c1};
    const double *asset2[PIVOTPOINT_INPUTS] = {h2, l2, c2};
    const double *asset3[PIVOTPOINT_INPUTS] = {h3, l3, c3};
    const double *asset4[PIVOTPOINT_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    double options[PIVOTPOINT_OPTIONS] = {5.0};
    CSimdResult r = pivotpoint_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        pivotpoint_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double o5[] = {5.0}, o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0};
    const double *const simd_opts[4] = {o5, o10, o14, o20};

    CSimdResult r = pivotpoint_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i][0] -> results for option set i (periods 5/10/14/20) */
    for (uintptr_t i = 0; i < r.num_results; i++) pivotpoint_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, c2 := h1, l1, c1
    h3, l3, c3 := h1, l1, c1
    h4, l4, c4 := h1, l1, c1

    assets := [][indicators.PivotpointInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Pivotpoint.SimdByAssets(assets, []float64{5.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    sim2, _ := indicators.Pivotpoint.SimdByOptions(high, low, close, [][]float64{{5}, {10}, {14}, {20}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Pivotpoint;

    double[] h1 = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] l1 = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] c1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    // Reuse the same data for assets 2-4 in this example
    double[] h2 = h1;
    double[] l2 = l1;
    double[] c2 = c1;
    double[] h3 = h1;
    double[] l3 = l1;
    double[] c3 = c1;
    double[] h4 = h1;
    double[] l4 = l1;
    double[] c4 = c1;

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}};
    try (SimdResult sim = Pivotpoint.simdByAssets(assets, new double[] {5.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Pivotpoint;

    // (high/low/close series as in the Basic tab)
    try (SimdResult sim = Pivotpoint.simdByOptions(new double[][] {high, low, close},
            new double[][] {{5}, {10}, {14}, {20}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period set %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```python
    simd_inputs = [[h1, l1, c1], [h2, l2, c2], [h3, l3, c3], [h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.pivotpoint.simd_by_assets(simd_inputs, [5.0])
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```python
    simd_options = [[5], [10], [14], [20]]
    outputs_list, states = tulip_rs.indicators.pivotpoint.simd_by_options([high, low, close], simd_options)
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
    const [results] = ti.pivotpoint.simdByAssets(simdInputs, [5]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[3]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.pivotpoint.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[3]));
    ```


