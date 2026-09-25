# Fisher Transform

Converts prices into a Gaussian normal distribution. Sharp moves in the Fisher value can signal potential price reversals; the signal line is a one-bar lag of the Fisher line.

**Inputs:** `[high, low]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[fisher, fisher_signal]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::fisher::{Fisher, TIndicatorState, Indicator};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, _state) = Fisher::indicator(&inputs, &[10.0], None).unwrap();
    println!("Fisher:        {:?}", outputs[0]);
    println!("Fisher Signal: {:?}", outputs[1]);

    // State continuation
    let inputs2 = [&high[..10], &low[..10]];
    let (outputs2, mut state) = Fisher::indicator(&inputs2, &[10.0], None).unwrap();
    println!("Partial Fisher: {:?}", outputs2[0]);

    let new_inputs = [&high[10..], &low[10..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued Fisher:        {:?}", continued[0]);
    println!("Continued Fisher Signal: {:?}", continued[1]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01};
    const double *inputs[FISHER_INPUTS] = {high, low};  /* MUST supply both high AND low */
    double options[FISHER_OPTIONS] = {10.0};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = fisher_indicator(inputs, 15, options, NULL, 0);
    /* r.outputs[0] -> the Fisher series, length r.output_lens[0] */
    /* r.outputs[1] -> the Signal series, length r.output_lens[1] */
    tulip_ffi_result_free(r);
    fisher_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = fisher_indicator(inputs, 10, options, NULL, 0);
    double new_high[] = {85.90, 86.58, 86.98, 88.00, 87.87};
    double new_low[]  = {84.03, 85.39, 85.76, 87.17, 87.01};
    const double *new_inputs[FISHER_INPUTS] = {new_high, new_low};
    CBatchResult b = fisher_batch(p.state, new_inputs, 5, NULL, 0);
    /* b.outputs[0] -> Fisher for just the five new bars */
    /* b.outputs[1] -> Signal for just the five new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    fisher_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01}
    options := []float64{10.0}

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Fisher.Indicator(high, low, options, nil)
    fmt.Println(res.Rows[0]) // Fisher values
    fmt.Println(res.Rows[1]) // Signal values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Fisher.Indicator(high[:10], low[:10], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[10:], low[10:], nil)
    fmt.Println(batch.Rows[0]) // continued Fisher values
    fmt.Println(batch.Rows[1]) // continued Signal values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Fisher;

    double[] high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87};
    double[] low = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01};
    double[] options = {10.0};

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Fisher.indicator(new double[][] {high, low}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // Fisher values
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // Signal values
    }

    // Partial computation + state continuation.
    Outcome p = Fisher.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, 10),
        java.util.Arrays.copyOfRange(low, 0, 10)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, 10, 15),
            java.util.Arrays.copyOfRange(low, 10, 15)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued Fisher values
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(1))); // continued Signal values
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    outputs, state = tulip_rs.indicators.fisher.indicator([high, low], [10.0])
    print("Fisher:        ", outputs[0])
    print("Fisher Signal: ", outputs[1])

    # State continuation
    outputs2, state = tulip_rs.indicators.fisher.indicator([high[:10], low[:10]], [10.0])
    continued = state.batch_indicator([high[10:], low[10:]])
    print("Continued Fisher:        ", continued[0])
    print("Continued Fisher Signal: ", continued[1])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low  = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);

    const [outputs, state] = ti.fisher.indicator([high, low], [9]);
    console.log('Fisher:', outputs[0]);
    console.log('Signal:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.fisher.indicator([high.slice(0, n), low.slice(0, n)], [9]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued Fisher:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.fisher.indicator([high, low], [9]);
    console.log('Fisher:', outputs[0]);
    console.log('Signal:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.fisher.indicator([high.slice(0, n), low.slice(0, n)], [9]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued Fisher:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::fisher::{Fisher, Indicator};

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                  85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                  84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let h2 = h1.clone(); let l2 = l1.clone();
    let h3 = h1.clone(); let l3 = l1.clone();
    let h4 = h1.clone(); let l4 = l1.clone();

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];

    let results = Fisher::indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Fisher:        {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Fisher Signal: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::fisher::{Fisher, IndicatorByOptions};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let inputs = [high.as_slice(), low.as_slice()];
    let results = Fisher::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Option set {} Fisher:        {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Fisher Signal: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01};

    double h2[15], l2[15];
    for (size_t i = 0; i < 15; i++) { h2[i] = h1[i] * 1.1; l2[i] = l1[i] * 1.1; }

    double h3[15], l3[15];
    for (size_t i = 0; i < 15; i++) { h3[i] = h1[i] - 0.5; l3[i] = l1[i] - 0.5; }

    double h4[15], l4[15];
    for (size_t i = 0; i < 15; i++) { h4[i] = h1[i] * 1.01; l4[i] = l1[i] * 1.01; }

    const double *const asset1[FISHER_INPUTS] = {h1, l1};
    const double *const asset2[FISHER_INPUTS] = {h2, l2};
    const double *const asset3[FISHER_INPUTS] = {h3, l3};
    const double *const asset4[FISHER_INPUTS] = {h4, l4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = fisher_simd_by_assets(simd_inputs, 4, 15, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> Fisher */
        /* r.outputs[i][1] -> Signal */
        fisher_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o5[] = {5.0}, o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0};
    const double *const simd_opts[4] = {o5, o10, o14, o20};

    CSimdResult r = fisher_simd_by_options(inputs, 15, simd_opts, 4, NULL, 0);
    /* r.outputs[i][0] -> Fisher */
    /* r.outputs[i][1] -> Signal */
    for (uintptr_t i = 0; i < r.num_results; i++) fisher_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01}

    // Reuse the same data for assets 2–4 in this example
    h2, l2 := h1, l1
    h3, l3 := h1, l1
    h4, l4 := h1, l1

    assets := [][indicators.FisherInputs][]float64{{h1, l1}, {h2, l2}, {h3, l3}, {h4, l4}}
    sim, _ := indicators.Fisher.SimdByAssets(assets, []float64{10.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d Fisher: %v\n", i+1, lanes[0])
        fmt.Printf("Asset %d Signal: %v\n", i+1, lanes[1])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01}

    sim2, _ := indicators.Fisher.SimdByOptions(high, low, [][]float64{{5}, {10}, {14}, {20}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Option set %d Fisher: %v\n", i+1, lanes[0])
        fmt.Printf("Option set %d Signal: %v\n", i+1, lanes[1])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same period applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Fisher;

    double[] h1 = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87};
    double[] l1 = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01};

    double[] h2 = h1.clone(); double[] l2 = l1.clone();
    double[] h3 = h1.clone(); double[] l3 = l1.clone();
    double[] h4 = h1.clone(); double[] l4 = l1.clone();

    // One entry per asset; each asset lists its INPUTS series (high, low).
    double[][][] assets = {{h1, l1}, {h2, l2}, {h3, l3}, {h4, l4}};
    try (SimdResult sim = Fisher.simdByAssets(assets, new double[] {10.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d Fisher: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0))); // Fisher values
            System.out.printf("Asset %d Signal: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1))); // Signal values
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Fisher;

    double[] high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87};
    double[] low = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01};

    try (SimdResult sim = Fisher.simdByOptions(new double[][] {high, low},
            new double[][] {{5}, {10}, {14}, {20}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Option set %d Fisher: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0))); // Fisher values
            System.out.printf("Option set %d Signal: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1))); // Signal values
        }
    }
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    simd_inputs = [
        [high,        low],
        [high + 0.5,  low + 0.5],
        [high - 0.5,  low - 0.5],
        [high * 1.01, low * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.fisher.simd_by_assets(simd_inputs, [10.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Fisher:        {out[0]}")
        print(f"Asset {i + 1} Fisher Signal: {out[1]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.fisher.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} Fisher:        {out[0]}")
        print(f"Option set {i + 1} Fisher Signal: {out[1]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.fisher.simdByAssets(simdInputs, [9]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} Fisher:`, out[0], 'Signal:', out[1]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.fisher.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]} Fisher:`, out[0], 'Signal:', out[1]));
    ```
