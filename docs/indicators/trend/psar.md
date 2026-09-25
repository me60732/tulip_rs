# PSAR — Parabolic SAR

A trailing stop-and-reverse indicator. The SAR dot flips below or above price to signal trend direction.

**Inputs:** `[high, low]` | **Options:** `[acceleration_factor_step, acceleration_factor_maximum]` | **Outputs:** `[psar]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::psar::{Psar, Indicator, TIndicatorState};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    // options: [acceleration_factor_step, acceleration_factor_maximum]
    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = Psar::indicator(&inputs, &[0.02, 0.2], None).unwrap();
    println!("{:?}", outputs[0]); // PSAR values

    // State continuation — feed new bars without reprocessing history
    let partial_high = high[..8].to_vec();
    let partial_low  = low[..8].to_vec();
    let (outputs2, mut state) = Psar::indicator(&[partial_high.as_slice(), partial_low.as_slice()], &[0.02, 0.2], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high = vec![85.90_f64];
    let new_low  = vec![84.03_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice()],
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
    double options[PSAR_OPTIONS] = {0.02, 0.2}; // acceleration_factor_step, acceleration_factor_maximum
    const double *inputs[PSAR_INPUTS] = {high, low};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = psar_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the PSAR series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    psar_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult pr = psar_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.90};
    double new_low[] = {84.03};
    const double *new_inputs[PSAR_INPUTS] = {new_high, new_low};
    CBatchResult br = psar_batch(pr.state, new_inputs, 1, NULL, 0);
    /* br.outputs[0] -> PSAR values for just the new bar */
    tulip_ffi_batch_result_free(br);
    tulip_ffi_result_free(pr);
    psar_state_free(pr.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    options := []float64{0.02, 0.2} // acceleration_factor_step, acceleration_factor_maximum

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Psar.Indicator(high, low, options, nil)
    fmt.Println(res.Rows[0]) // PSAR values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Psar.Indicator(high[:8], low[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], nil)
    fmt.Println(batch.Rows[0]) // continued PSAR values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Psar;

    double[] high = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double[] options = {0.02, 0.2}; // acceleration_factor_step, acceleration_factor_maximum

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Psar.indicator(new double[][] {high, low}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // PSAR values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Psar.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued PSAR values
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    # options: [acceleration_factor_step, acceleration_factor_maximum]
    outputs, state = tulip_rs.indicators.psar.indicator([high, low], [0.02, 0.2])
    print(outputs[0])  # PSAR values

    # State continuation
    new_high = np.array([85.30], dtype=np.float64)
    new_low  = np.array([84.60], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low  = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);

    const [outputs, state] = ti.psar.indicator([high, low], [0.02, 0.2]);
    console.log('PSAR:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.psar.indicator([high.slice(0, n), low.slice(0, n)], [0.02, 0.2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued PSAR:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.psar.indicator([high, low], [0.02, 0.2]);
    console.log('PSAR:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.psar.indicator([high.slice(0, n), low.slice(0, n)], [0.02, 0.2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued PSAR:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::psar::{Psar, Indicator};

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = Psar::indicator_by_assets::<4>(&inputs, &[0.02, 0.2], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::psar::{Psar, IndicatorByOptions};

    let opts: [&[f64; 2]; 4] = [&[0.01, 0.1], &[0.02, 0.2], &[0.03, 0.3], &[0.04, 0.4]];
    let results = Psar::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Step/Max {}/{}: {:?}", opts[i][0], opts[i][1], out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double h2[] = {72.15, 71.89, 73.03, 73.30, 73.85, 73.90, 73.33, 74.30, 74.84, 75.00};
    double l2[] = {71.29, 70.64, 71.31, 72.65, 73.07, 73.11, 72.49, 72.30, 74.15, 74.11};
    double h3[] = {52.15, 51.89, 53.03, 53.30, 53.85, 53.90, 53.33, 54.30, 54.84, 55.00};
    double l3[] = {51.29, 50.64, 51.31, 52.65, 53.07, 53.11, 52.49, 52.30, 54.15, 54.11};
    double h4[] = {102.15, 101.89, 103.03, 103.30, 103.85, 103.90, 103.33, 104.30, 104.84, 105.00};
    double l4[] = {101.29, 100.64, 101.31, 102.65, 103.07, 103.11, 102.49, 102.30, 104.15, 104.11};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[PSAR_INPUTS] = {h1, l1};
    const double *asset2[PSAR_INPUTS] = {h2, l2};
    const double *asset3[PSAR_INPUTS] = {h3, l3};
    const double *asset4[PSAR_INPUTS] = {h4, l4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = psar_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's PSAR series */
        psar_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 option sets in parallel:

    ```c
    #define EXPANDED_LEN (15 * 20)
    static double high_expanded[EXPANDED_LEN];
    static double low_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 20; i++) {
        for (size_t j = 0; j < 15; j++) {
            high_expanded[i * 15 + j] = h1[j];
            low_expanded[i * 15 + j] = l1[j];
        }
    }
    const double *expanded_inputs[PSAR_INPUTS] = {high_expanded, low_expanded};

    static const double opts_1[PSAR_OPTIONS] = {0.1, 1.0};
    static const double opts_2[PSAR_OPTIONS] = {0.2, 2.0};
    static const double opts_3[PSAR_OPTIONS] = {0.3, 3.0};
    static const double opts_4[PSAR_OPTIONS] = {0.4, 4.0};
    const double *const simd_opts[4] = {opts_1, opts_2, opts_3, opts_4};

    CSimdResult r = psar_simd_by_options(expanded_inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> option set i's PSAR series */
        psar_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}

    // Reuse the same data for assets 2–4 in this example
    h2, l2 := h1, l1
    h3, l3 := h1, l1
    h4, l4 := h1, l1

    assets := [][indicators.PsarInputs][]float64{{h1, l1}, {h2, l2}, {h3, l3}, {h4, l4}}
    sim, _ := indicators.Psar.SimdByAssets(assets, []float64{0.02, 0.2}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}

    sim2, _ := indicators.Psar.SimdByOptions(high, low, [][]float64{{0.01, 0.2}, {0.02, 0.2}, {0.03, 0.2}, {0.05, 0.2}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Step/Max %.2f/%.1f: %v\n", lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Psar;

    double[] h1 = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] l1 = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};

    // Reuse the same data for assets 2–4 in this example.
    double[] h2 = h1, l2 = l1;
    double[] h3 = h1, l3 = l1;
    double[] h4 = h1, l4 = l1;

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{h1, l1}, {h2, l2}, {h3, l3}, {h4, l4}};
    try (SimdResult sim = Psar.simdByAssets(assets, new double[] {0.02, 0.2}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Psar;

    double[] high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};

    try (SimdResult sim = Psar.simdByOptions(new double[][] {high, low},
            new double[][] {{0.01, 0.2}, {0.02, 0.2}, {0.03, 0.2}, {0.05, 0.2}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Step/Max %.2f/%.1f: %s%n", optionSets[i][0], optionSets[i][1],
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1],
        [h2, l2],
        [h3, l3],
        [h4, l4],
    ]
    outputs_list, states = tulip_rs.indicators.psar.simd_by_assets(simd_inputs, [0.02, 0.2])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[0.01, 0.1], [0.02, 0.2], [0.03, 0.3], [0.04, 0.4]]
    outputs_list, states = tulip_rs.indicators.psar.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Step/Max {simd_options[i][0]}/{simd_options[i][1]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.psar.simdByAssets(simdInputs, [0.02, 0.2]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[0.01, 0.1], [0.02, 0.2], [0.03, 0.3], [0.04, 0.4]];
    const [results] = ti.psar.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Step ${simdOptions[i][0]}:`, out[0]));
    ```
