# SMA Envelope

Three bands around a Simple Moving Average. `middle = SMA(real, period)`, `upper = SMA + SMA × (percentage / 100)`, `lower = SMA − SMA × (percentage / 100)`. The envelope expands and contracts proportionally with the SMA level. Used to identify overbought/oversold conditions relative to the prevailing trend. Rendered as a price overlay.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period, percentage]` &nbsp;|&nbsp; **Outputs:** `[lower, middle, upper]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::smaenvelope::{SmaEnvelope, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // options: [period, percentage]
    let (outputs, _state) = SmaEnvelope::indicator(&[close.as_slice()], &[14.0, 2.5], None).unwrap();
    println!("Lower:  {:?}", outputs[0]);
    println!("Middle: {:?}", outputs[1]);
    println!("Upper:  {:?}", outputs[2]);

    // State continuation
    let (outputs2, mut state) = SmaEnvelope::indicator(&[&close[..8]], &[14.0, 2.5], None).unwrap();
    println!("Partial Lower:  {:?}", outputs2[0]);
    println!("Partial Middle: {:?}", outputs2[1]);
    println!("Partial Upper:  {:?}", outputs2[2]);

    let continued = state.batch_indicator(&[&close[8..]], None).unwrap();
    println!("Continued Lower:  {:?}", continued[0]);
    println!("Continued Middle: {:?}", continued[1]);
    println!("Continued Upper:  {:?}", continued[2]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[SMAENVELOPE_OPTIONS] = {14.0, 2.5}; // period, percentage
    const double *inputs[SMAENVELOPE_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = smaenvelope_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> lower, r.outputs[1] -> middle, r.outputs[2] -> upper */
    tulip_ffi_result_free(r);
    smaenvelope_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = smaenvelope_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[SMAENVELOPE_INPUTS] = {new_close};
    CBatchResult b = smaenvelope_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0/1/2] -> lower/middle/upper for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    smaenvelope_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0, 2.5} // period, percentage

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Smaenvelope.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // lower
    fmt.Println(res.Rows[1]) // middle
    fmt.Println(res.Rows[2]) // upper
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Smaenvelope.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued lower
    fmt.Println(batch.Rows[1]) // continued middle
    fmt.Println(batch.Rows[2]) // continued upper
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Smaenvelope;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {14.0, 2.5}; // period, percentage

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Smaenvelope.indicator(new double[][] {close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // lower
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // middle
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2))); // upper
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Smaenvelope.indicator(
        new double[][] {java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(
            new double[][] {java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued lower
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(1))); // continued middle
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(2))); // continued upper
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # options: [period, percentage]
    outputs, state = tulip_rs.indicators.smaenvelope.indicator([close], [14.0, 2.5])
    print("Lower: ", outputs[0])
    print("Middle:", outputs[1])
    print("Upper: ", outputs[2])

    # State continuation
    outputs2, state = tulip_rs.indicators.smaenvelope.indicator([close[:8]], [14.0, 2.5])
    print("Partial Lower: ", outputs2[0])
    print("Partial Middle:", outputs2[1])
    print("Partial Upper: ", outputs2[2])

    continued = state.batch_indicator([close[8:]])
    print("Continued Lower: ", continued[0])
    print("Continued Middle:", continued[1])
    print("Continued Upper: ", continued[2])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    // options: [period, percentage]
    const [outputs, state] = ti.smaenvelope.indicator([close], [14, 2.5]);
    console.log('SMA Envelope Lower:', outputs[0]);
    console.log('SMA Envelope Middle:', outputs[1]);
    console.log('SMA Envelope Upper:', outputs[2]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.smaenvelope.indicator([close.slice(0, n)], [14, 2.5]);
    const continued = state2.batchIndicator([close.slice(n)]);
    console.log('Continued Lower:', continued[0]);
    console.log('Continued Middle:', continued[1]);
    console.log('Continued Upper:', continued[2]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    // options: [period, percentage]
    const [outputs, state] = ti.smaenvelope.indicator([close], [14, 2.5]);
    console.log('SMA Envelope Lower:', outputs[0]);
    console.log('SMA Envelope Middle:', outputs[1]);
    console.log('SMA Envelope Upper:', outputs[2]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.smaenvelope.indicator([close.slice(0, n)], [14, 2.5]);
    const continued = state2.batchIndicator([close.slice(n)]);
    console.log('Continued Lower:', continued[0]);
    console.log('Continued Middle:', continued[1]);
    console.log('Continued Upper:', continued[2]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::smaenvelope::{SmaEnvelope, Indicator};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = a1.iter().map(|x| x + 5.0).collect::<Vec<_>>();
    let a3 = a1.iter().map(|x| x - 5.0).collect::<Vec<_>>();
    let a4 = a1.iter().map(|x| x * 1.02).collect::<Vec<_>>();

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = SmaEnvelope::indicator_by_assets::<4>(&inputs, &[14.0, 2.5], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Lower:  {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Middle: {:?}", i + 1, asset_outputs[1]);
        println!("Asset {} Upper:  {:?}", i + 1, asset_outputs[2]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::smaenvelope::{SmaEnvelope, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 2]; 4] = [
        &[10.0, 2.0],
        &[14.0, 2.5],
        &[20.0, 3.0],
        &[50.0, 5.0],
    ];

    let results = SmaEnvelope::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Option set {} Lower:  {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Middle: {:?}", i + 1, opt_outputs[1]);
        println!("Option set {} Upper:  {:?}", i + 1, opt_outputs[2]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {76.59, 76.06, 77.87, 78.00, 78.61, 78.15, 77.84, 78.99, 79.55, 79.36};
    double a4[] = {83.22, 82.68, 83.43, 83.66, 83.68, 83.01, 82.80, 83.77, 84.44, 84.05};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[SMAENVELOPE_INPUTS] = {a1};
    const double *asset2[SMAENVELOPE_INPUTS] = {a2};
    const double *asset3[SMAENVELOPE_INPUTS] = {a3};
    const double *asset4[SMAENVELOPE_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = smaenvelope_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> lower, r.outputs[i][1] -> middle, r.outputs[i][2] -> upper */
        smaenvelope_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in one call:

    ```c
    double o10p2[] = {10.0, 2.0};
    double o14p25[] = {14.0, 2.5};
    double o20p3[] = {20.0, 3.0};
    double o50p5[] = {50.0, 5.0};
    const double *const simd_opts[4] = {o10p2, o14p25, o20p3, o50p5};

    CSimdResult r = smaenvelope_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i][0] -> lower, r.outputs[i][1] -> middle, r.outputs[i][2] -> upper */
    for (uintptr_t i = 0; i < r.num_results; i++) smaenvelope_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := a1, a1, a1

    assets := [][indicators.SmaenvelopeInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Smaenvelope.SimdByAssets(assets, []float64{14.0, 2.5}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d Lower:  %v\n", i+1, lanes[0])
        fmt.Printf("Asset %d Middle: %v\n", i+1, lanes[1])
        fmt.Printf("Asset %d Upper:  %v\n", i+1, lanes[2])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    sim2, _ := indicators.Smaenvelope.SimdByOptions(close, [][]float64{{10.0, 2.0}, {14.0, 2.5}, {20.0, 3.0}, {50.0, 5.0}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Option set %d Lower:  %v\n", i+1, lanes[0])
        fmt.Printf("Option set %d Middle: %v\n", i+1, lanes[1])
        fmt.Printf("Option set %d Upper:  %v\n", i+1, lanes[2])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Smaenvelope;

    double[] a1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double[] a2 = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double[] a3 = {76.59, 76.06, 77.87, 78.00, 78.61, 78.15, 77.84, 78.99, 79.55, 79.36};
    double[] a4 = {83.22, 82.68, 83.43, 83.66, 83.68, 83.01, 82.80, 83.77, 84.44, 84.05};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1}, {a2}, {a3}, {a4}};
    try (SimdResult sim = Smaenvelope.simdByAssets(assets, new double[] {14.0, 2.5}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d Lower:  %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
            System.out.printf("Asset %d Middle: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1)));
            System.out.printf("Asset %d Upper:  %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 2)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Smaenvelope;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    try (SimdResult sim = Smaenvelope.simdByOptions(new double[][] {close},
            new double[][] {{10.0, 2.0}, {14.0, 2.5}, {20.0, 3.0}, {50.0, 5.0}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Option set %d Lower:  %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
            System.out.printf("Option set %d Middle: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1)));
            System.out.printf("Option set %d Upper:  %s%n", i + 1,
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

    simd_inputs = [
        [close],
        [close + 5.0],
        [close - 5.0],
        [close * 1.02],
    ]
    outputs_list, states = tulip_rs.indicators.smaenvelope.simd_by_assets(simd_inputs, [14.0, 2.5])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Lower:  {out[0]}")
        print(f"Asset {i + 1} Middle: {out[1]}")
        print(f"Asset {i + 1} Upper:  {out[2]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    simd_options = [
        [10.0, 2.0],
        [14.0, 2.5],
        [20.0, 3.0],
        [50.0, 5.0],
    ]
    outputs_list, states = tulip_rs.indicators.smaenvelope.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} Lower:  {out[0]}")
        print(f"Option set {i + 1} Middle: {out[1]}")
        print(f"Option set {i + 1} Upper:  {out[2]}")
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
    const [results] = ti.smaenvelope.simdByAssets(simdInputs, [14, 2.5]);
    results.forEach((out, i) => {
        console.log(`Asset ${i + 1} Lower:`, out[0]);
        console.log(`Asset ${i + 1} Middle:`, out[1]);
        console.log(`Asset ${i + 1} Upper:`, out[2]);
    });
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[10, 2.0], [14, 2.5], [20, 3.0], [50, 5.0]];
    const [results] = ti.smaenvelope.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0], out[1], out[2]));
    ```
