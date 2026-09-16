# TR — True Range

The single-bar true range: the greatest of (high-low), |high-prev_close|, |low-prev_close|.

**Inputs:** `[high, low, close]` | **Options:** `[]` | **Outputs:** `[tr]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::tr::{Tr, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = Tr::indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // True Range values

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = Tr::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high  = vec![85.90_f64];
    let new_low   = vec![84.03_f64];
    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    TR has no options (`TR_OPTIONS=0`); pass `NULL` for the options array.

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[TR_INPUTS] = {high, low, close};
    const double *options = NULL; // TR has no options

    /* Full computation */
    CIndicatorResult r = tr_indicator(inputs, 10, options, NULL, 0);
    printf("TR[0]: %.4f\n", r.outputs[0][0]); // outputs[0] is the true range series
    tulip_ffi_result_free(r);
    tr_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = tr_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.90}, new_low[] = {84.03}, new_close[] = {85.53};
    const double *new_inputs[TR_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = tr_batch(p.state, new_inputs, 1, NULL, 0);
    printf("Continued TR[0]: %.4f\n", b.outputs[0][0]);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    tr_state_free(p.state);
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

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Tr.Indicator(high, low, close, []float64{}, nil)
    fmt.Println(res.Rows[0]) // True Range values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Tr.Indicator(high[:8], low[:8], close[:8], []float64{}, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued TR values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.tr.indicator([high, low, close], [])
    print(outputs[0])  # True Range values

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.tr.indicator([high, low, close], []);
    console.log('TR:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.tr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued TR:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.tr.indicator([high, low, close], []);
    console.log('TR:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.tr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued TR:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::tr::{Tr, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = Tr::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "C"

    **By assets** — TR has no options; by-options SIMD is not provided.

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                   83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                   83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36};

    double h2[10], l2[10], c2[10];
    for (uintptr_t i = 0; i < 10; i++) { h2[i] = h1[i] * 1.1; l2[i] = l1[i] * 1.1; c2[i] = c1[i] * 1.1; }
    double h3[10], l3[10], c3[10];
    for (uintptr_t i = 0; i < 10; i++) { h3[i] = 90.0 + (double)i * 0.5 + h1[i] * 0.1;
                                          l3[i] = 90.0 + (double)i * 0.5 + l1[i] * 0.1;
                                          c3[i] = 90.0 + (double)i * 0.5 + c1[i] * 0.1; }
    double h4[10], l4[10], c4[10];
    for (uintptr_t i = 0; i < 10; i++) { h4[i] = 100.0 - (double)i * 0.3 + h1[i] * 0.05;
                                          l4[i] = 100.0 - (double)i * 0.3 + l1[i] * 0.05;
                                          c4[i] = 100.0 - (double)i * 0.3 + c1[i] * 0.05; }

    const double *asset1[TR_INPUTS] = {h1, l1, c1};
    const double *asset2[TR_INPUTS] = {h2, l2, c2};
    const double *asset3[TR_INPUTS] = {h3, l3, c3};
    const double *asset4[TR_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    const double *options = NULL; // TR has no options

    CSimdResult r = tr_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Asset %zu TR[0]: %.4f\n", i + 1, r.outputs[i][0][0]);
        tr_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    assets := [][indicators.TrInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Tr.SimdByAssets(assets, []float64{}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    *This indicator has no options, so by-options SIMD does not offer.*

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.tr.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
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
    const [results] = ti.tr.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
