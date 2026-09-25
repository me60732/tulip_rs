# Volatility

Annualised historical volatility based on log returns over `period` bars.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[volatility]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::volatility::{Volatility, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, mut state) = Volatility::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // Annualised volatility values

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Volatility::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[VOLATILITY_OPTIONS] = {14.0}; // period
    const double *inputs[VOLATILITY_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = volatility_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the volatility series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    volatility_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = volatility_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.10, 85.72};
    const double *new_inputs[VOLATILITY_INPUTS] = {new_close};
    CBatchResult b = volatility_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> volatility values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    volatility_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Volatility.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // Annualised volatility values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Volatility.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued volatility values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Volatility;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {14.0}; // period

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Volatility.indicator(new double[][] {close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // Annualised volatility values
    }

    // Partial computation + state continuation.
    Outcome p = Volatility.indicator(
        new double[][] {java.util.Arrays.copyOfRange(close, 0, 8)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(
            new double[][] {java.util.Arrays.copyOfRange(close, 8, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued volatility values
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.volatility.indicator([close], [14.0])
    print(outputs[0])  # Annualised volatility values

    # State continuation
    new_close = np.array([85.10, 85.72], dtype=np.float64)
    continued = state.batch_indicator([new_close])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.volatility.indicator([close], [14]);
    console.log('Volatility(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.volatility.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Volatility:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.volatility.indicator([close], [14]);
    console.log('Volatility(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.volatility.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Volatility:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::volatility::{Volatility, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = Volatility::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::volatility::{Volatility, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = Volatility::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same option applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[VOLATILITY_INPUTS] = {a1};
    const double *asset2[VOLATILITY_INPUTS] = {a2};
    const double *asset3[VOLATILITY_INPUTS] = {a3};
    const double *asset4[VOLATILITY_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = volatility_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        volatility_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o30[] = {30.0};
    const double *const simd_opts[4] = {o7, o14, o21, o30};

    CSimdResult r = volatility_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i (periods 7/14/21/30) */
    for (uintptr_t i = 0; i < r.num_results; i++) volatility_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    r1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    r2 := r1
    r3 := r1
    r4 := r1

    assets := [][indicators.VolatilityInputs][]float64{{r1}, {r2}, {r3}, {r4}}
    sim, _ := indicators.Volatility.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Volatility.SimdByOptions(close, [][]float64{{7}, {14}, {21}, {28}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same period applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Volatility;

    double[] r1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    // Reuse the same data for assets 2–4 in this example
    double[] r2 = r1;
    double[] r3 = r1;
    double[] r4 = r1;

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{r1}, {r2}, {r3}, {r4}};
    try (SimdResult sim = Volatility.simdByAssets(assets, new double[] {14.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Volatility;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    try (SimdResult sim = Volatility.simdByOptions(new double[][] {close},
            new double[][] {{7}, {14}, {21}, {28}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period set %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [np.array(asset1_close, dtype=np.float64)],
        [np.array(asset2_close, dtype=np.float64)],
        [np.array(asset3_close, dtype=np.float64)],
        [np.array(asset4_close, dtype=np.float64)],
    ]
    outputs_list, states = tulip_rs.indicators.volatility.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.volatility.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.volatility.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.volatility.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
