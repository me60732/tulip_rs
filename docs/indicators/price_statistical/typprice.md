# Typical Price — `typprice`

`(High + Low + Close) / 3` for each bar. Commonly used as the price input for indicators like CCI.

**Inputs:** `[high, low, close]` | **Options:** none | **Outputs:** `[typprice]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::typprice::{Typprice, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = Typprice::indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = Typprice::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high   = vec![85.90_f64];
    let new_low    = vec![84.03_f64];
    let new_close  = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]   = {82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]    = {81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11};
    double close[]  = {81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36};
    const double *options = NULL; // TYPPRICE has no options

    const double *inputs[TYPPRICE_INPUTS] = {high, low, close};

    /* Full computation */
    CIndicatorResult r = typprice_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> Typical Price series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    typprice_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = typprice_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.90};
    double new_low[]  = {84.03};
    double new_close[] = {85.53};
    const double *new_inputs[TYPPRICE_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = typprice_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> Typical Price for the new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    typprice_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high  := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00}
    low   := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{} // TYPPRICE has no options

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Typprice.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // Typical Price values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Typprice.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued Typical Price values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Typprice;

    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {}; // TYPPRICE has no options

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Typprice.indicator(new double[][] {high, low, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // Typical Price values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Typprice.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10),
            java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued Typical Price values
        }
    }
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.typprice.indicator([high, low, close], [])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.typprice.indicator([high, low, close], []);
    console.log('TypPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.typprice.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued TypPrice:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.typprice.indicator([high, low, close], []);
    console.log('TypPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.typprice.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued TypPrice:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::typprice::{Typprice, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = Typprice::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1_high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double a1_low[]  = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double a1_close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    double a2_high[] = {85.00, 85.50, 86.00, 86.50, 87.00, 87.50, 88.00, 88.50, 89.00, 89.50};
    double a2_low[]  = {83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50, 87.00, 87.50};
    double a2_close[] = {84.00, 84.50, 85.00, 85.50, 86.00, 86.50, 87.00, 87.50, 88.00, 88.50};

    double a3_high[] = {90.00, 91.00, 92.00, 93.00, 94.00, 95.00, 96.00, 97.00, 98.00, 99.00};
    double a3_low[]  = {88.00, 89.00, 90.00, 91.00, 92.00, 93.00, 94.00, 95.00, 96.00, 97.00};
    double a3_close[] = {89.00, 90.00, 91.00, 92.00, 93.00, 94.00, 95.00, 96.00, 97.00, 98.00};

    double a4_high[] = {100.00, 99.00, 98.00, 97.00, 96.00, 95.00, 94.00, 93.00, 92.00, 91.00};
    double a4_low[]  = {98.00, 97.00, 96.00, 95.00, 94.00, 93.00, 92.00, 91.00, 90.00, 89.00};
    double a4_close[] = {99.00, 98.00, 97.00, 96.00, 95.00, 94.00, 93.00, 92.00, 91.00, 90.00};

    const double *asset1[TYPPRICE_INPUTS] = {a1_high, a1_low, a1_close};
    const double *asset2[TYPPRICE_INPUTS] = {a2_high, a2_low, a2_close};
    const double *asset3[TYPPRICE_INPUTS] = {a3_high, a3_low, a3_close};
    const double *asset4[TYPPRICE_INPUTS] = {a4_high, a4_low, a4_close};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    const double *options = NULL; // TYPPRICE has no options

    CSimdResult r = typprice_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's Typical Price series */
        typprice_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    a1_high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    a1_low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    a1_close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    a2_high, a2_low, a2_close := a1_high, a1_low, a1_close
    a3_high, a3_low, a3_close := a1_high, a1_low, a1_close
    a4_high, a4_low, a4_close := a1_high, a1_low, a1_close

    assets := [][indicators.TyppriceInputs][]float64{{a1_high, a1_low, a1_close}, {a2_high, a2_low, a2_close}, {a3_high, a3_low, a3_close}, {a4_high, a4_low, a4_close}}
    sim, _ := indicators.Typprice.SimdByAssets(assets, []float64{}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    _This indicator has no options, so SIMD by-options does not apply._

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Typprice;

    double[] a1_high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] a1_low  = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] a1_close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    // Reuse the same data for assets 2–4 in this example
    double[] a2_high = a1_high;
    double[] a2_low  = a1_low;
    double[] a2_close = a1_close;
    double[] a3_high = a1_high;
    double[] a3_low  = a1_low;
    double[] a3_close = a1_close;
    double[] a4_high = a1_high;
    double[] a4_low  = a1_low;
    double[] a4_close = a1_close;

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1_high, a1_low, a1_close}, {a2_high, a2_low, a2_close}, {a3_high, a3_low, a3_close}, {a4_high, a4_low, a4_close}};
    try (SimdResult sim = Typprice.simdByAssets(assets, new double[] {}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));}
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1, c1], [h2, l2, c2], [h3, l3, c3], [h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.typprice.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.typprice.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
