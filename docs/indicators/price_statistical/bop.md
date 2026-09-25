# BOP — Balance of Power — `bop`

Measures the strength of buyers vs sellers: `(Close - Open) / (High - Low)`.

**Inputs:** `[open, high, low, close]` | **Options:** none | **Outputs:** `[bop]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::bop::{Bop, TIndicatorState, Indicator};

    let open  = vec![81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25_f64];
    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [open.as_slice(), high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _) = Bop::indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation
    let partial_open  = open[..8].to_vec();
    let partial_high  = high[..8].to_vec();
    let partial_low   = low[..8].to_vec();
    let partial_close = close[..8].to_vec();
    let inputs2 = [partial_open.as_slice(), partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()];
    let (outputs2, mut state) = Bop::indicator(&inputs2, &[], None).unwrap();
    println!("Partial BOP: {:?}", outputs2[0]);

    let new_open  = open[8..].to_vec();
    let new_high  = high[8..].to_vec();
    let new_low   = low[8..].to_vec();
    let new_close = close[8..].to_vec();
    let new_inputs = [new_open.as_slice(), new_high.as_slice(), new_low.as_slice(), new_close.as_slice()];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued BOP: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double open[] = {81.85, 81.20, 81.55, 82.91, 83.10,
                     83.41, 82.71, 82.70, 84.20, 84.25};
    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[BOP_INPUTS] = {open, high, low, close};

    /* Full computation (no options for bop) */
    CIndicatorResult r = bop_indicator(inputs, 10, NULL, NULL, 0);
    /* r.outputs[0] -> the BOP series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    bop_state_free(r.state);

    /* Partial computation + state continuation */
    const double *partial_inputs[BOP_INPUTS] = {open, high, low, close};
    CIndicatorResult p = bop_indicator(partial_inputs, 8, NULL, NULL, 0);
    double new_open[] = {84.20, 84.25};
    double new_high[] = {84.84, 85.00};
    double new_low[] = {84.15, 84.11};
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[BOP_INPUTS] = {new_open, new_high, new_low, new_close};
    CBatchResult b = bop_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> BOP values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    bop_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    open  := []float64{81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25}
    high  := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    low   := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{} // no options for bop

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Bop.Indicator(open, high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // BOP values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Bop.Indicator(open[:8], high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(open[8:], high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued BOP values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Bop;

    double[] open  = {81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25};
    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {};

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Bop.indicator(new double[][] {open, high, low, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // BOP values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Bop.indicator(new double[][] {
        java.util.Arrays.copyOfRange(open, 0, n),
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(open, n, 10),
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10),
            java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued BOP
        }
    }
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.bop.indicator([open_, high, low, close], [])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.bop.indicator([open_, high, low, close], []);
    console.log('BOP:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.bop.indicator([open_.slice(0, n), high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([open_.slice(n), high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued BOP:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.bop.indicator([open_, high, low, close], []);
    console.log('BOP:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.bop.indicator([open_.slice(0, n), high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([open_.slice(n), high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued BOP:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::bop::{Bop, Indicator};

    let o1 = vec![81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25_f64];
    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let o2 = o1.clone(); let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let o3 = o1.clone(); let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let o4 = o1.clone(); let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 4]; 4] = [
        &[o1.as_slice(), h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[o2.as_slice(), h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[o3.as_slice(), h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[o4.as_slice(), h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = Bop::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "C"

    **By assets** — same options (none), N assets in one call:

    ```c
    double o1[] = {81.85, 81.20, 81.55, 82.91, 83.10,
                   83.41, 82.71, 82.70, 84.20, 84.25};
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                   83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                   83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36};

    double o2[] = {82.85, 82.20, 82.55, 83.91, 84.10,
                   84.41, 83.71, 83.70, 85.20, 85.25};
    double h2[] = {83.15, 82.89, 84.03, 84.30, 84.85,
                   84.90, 84.33, 84.30, 85.84, 86.00};
    double l2[] = {82.29, 81.64, 82.31, 83.65, 84.07,
                   84.11, 83.49, 83.30, 85.15, 85.11};
    double c2[] = {82.59, 82.06, 83.87, 84.00, 84.61,
                   84.15, 83.84, 84.99, 85.55, 85.36};

    double o3[] = {83.85, 83.20, 83.55, 84.91, 85.10,
                   85.41, 84.71, 84.70, 86.20, 86.25};
    double h3[] = {84.15, 83.89, 85.03, 85.30, 85.85,
                   85.90, 85.33, 85.30, 86.84, 87.00};
    double l3[] = {83.29, 82.64, 83.31, 84.65, 85.07,
                   85.11, 84.49, 84.30, 86.15, 86.11};
    double c3[] = {83.59, 83.06, 84.87, 85.00, 85.61,
                   85.15, 84.84, 85.99, 86.55, 86.36};

    double o4[] = {84.85, 84.20, 84.55, 85.91, 86.10,
                   86.41, 85.71, 85.70, 87.20, 87.25};
    double h4[] = {85.15, 84.89, 86.03, 86.30, 86.85,
                   86.90, 86.33, 86.30, 87.84, 88.00};
    double l4[] = {84.29, 83.64, 84.31, 85.65, 86.07,
                   86.11, 85.49, 85.30, 87.15, 87.11};
    double c4[] = {84.59, 84.06, 85.87, 86.00, 86.61,
                   86.15, 85.84, 86.99, 87.55, 87.36};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[BOP_INPUTS] = {o1, h1, l1, c1};
    const double *asset2[BOP_INPUTS] = {o2, h2, l2, c2};
    const double *asset3[BOP_INPUTS] = {o3, h3, l3, c3};
    const double *asset4[BOP_INPUTS] = {o4, h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    /* bop has 0 options (BOP_OPTIONS = 0) */
    CSimdResult r = bop_simd_by_assets(simd_inputs, 4, 10, NULL, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's BOP series, length r.output_lens[i][0] */
        bop_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    _This indicator has 0 options (BOP_OPTIONS = 0), so simd_by_options does not exist._

=== "Go"

    **By assets** — same options (none), N assets in parallel:

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    o1 := []float64{81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25}
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    o2, h2, l2, c2 := o1, h1, l1, c1
    o3, h3, l3, c3 := o1, h1, l1, c1
    o4, h4, l4, c4 := o1, h1, l1, c1

    assets := [][indicators.BopInputs][]float64{{o1, h1, l1, c1}, {o2, h2, l2, c2}, {o3, h3, l3, c3}, {o4, h4, l4, c4}}
    sim, _ := indicators.Bop.SimdByAssets(assets, nil, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

=== "Java"

    **By assets** — same options (none), N assets in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Bop;

    double[] o1 = {81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25};
    double[] h1 = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] l1 = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] c1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    // Reuse the same data for assets 2–4 in this example
    double[] o2 = o1; double[] h2 = h1; double[] l2 = l1; double[] c2 = c1;
    double[] o3 = o1; double[] h3 = h1; double[] l3 = l1; double[] c3 = c1;
    double[] o4 = o1; double[] h4 = h1; double[] l4 = l1; double[] c4 = c1;

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{o1, h1, l1, c1}, {o2, h2, l2, c2}, {o3, h3, l3, c3}, {o4, h4, l4, c4}};
    try (SimdResult sim = Bop.simdByAssets(assets, new double[] {}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[o1, h1, l1, c1], [o2, h2, l2, c2], [o3, h3, l3, c3], [o4, h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.bop.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...open_], high.slice(), low.slice(), close.slice()],
        [open_.map(v => v * 1.1), high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [open_.map(v => v * 0.9), high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [open_.map(v => v * 1.02), high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.bop.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
