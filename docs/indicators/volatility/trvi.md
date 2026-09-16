# TRVI — True Range Volatility Indicator

Similar to CVI but uses True Range instead of high − low. Applies an EMA to TR then outputs the percentage rate of change: (ema_now − ema_old) / ema_old × 100. More responsive to overnight gaps than CVI. Optionally emits the raw True Range series (tr) and the EMA-of-TR series (ema).

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[trvi]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::trvi::{Trvi, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = Trvi::indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // TRVI values

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = Trvi::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high  = vec![85.20_f64];
    let new_low   = vec![84.50_f64];
    let new_close = vec![85.00_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    `trvi` exposes 2 optional outputs: `tr`, `ema`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[TRVI_OPTIONS] = {14.0}; // period
    const double *inputs[TRVI_INPUTS] = {high, low, close};

    /* Full computation */
    CIndicatorResult r = trvi_indicator(inputs, 10, options, NULL, 0);
    printf("trvi[0]: %.4f\n", r.outputs[0][0]); // outputs[0] is the primary trvi series
    tulip_ffi_result_free(r);
    trvi_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = trvi_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.20}, new_low[] = {84.50}, new_close[] = {85.00};
    const double *new_inputs[TRVI_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = trvi_batch(p.state, new_inputs, 1, NULL, 0);
    printf("Continued trvi[0]: %.4f\n", b.outputs[0][0]);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    trvi_state_free(p.state);
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

    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Trvi.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // TRVI values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Trvi.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued TRVI values
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

    outputs, state = tulip_rs.indicators.trvi.indicator([high, low, close], [14.0])
    print(outputs[0])  # TRVI values

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

    const [outputs, state] = ti.trvi.indicator([high, low, close], [14]);
    console.log('TRVI(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.trvi.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued TRVI:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.trvi.indicator([high, low, close], [14]);
    console.log('TRVI(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.trvi.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued TRVI:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `trvi` exposes 2 optional outputs: `tr`, `ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::trvi::{Trvi, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low   = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();

    let mask = [true, true];
    let (outputs, _state) = Trvi::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let trvi = &outputs[0]; // trvi (primary)
    let tr   = &outputs[1]; // tr (optional — requested)
    let ema  = &outputs[2]; // ema (optional — requested)
    ```

=== "C"

    `trvi` exposes 2 optional outputs: `tr`, `ema`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[TRVI_OPTIONS] = {14.0}; // period
    const double *inputs[TRVI_INPUTS] = {high, low, close};
    bool optional_outputs[2] = {true, true}; // tr, ema

    CIndicatorResult r = trvi_indicator(inputs, 10, options, optional_outputs, 2);
    printf("trvi[0]: %.4f\n", r.outputs[0][0]); // primary
    printf("tr[0]:   %.4f\n", r.outputs[1][0]);   // optional 0: tr
    printf("ema[0]:  %.4f\n", r.outputs[2][0]);   // optional 1: ema
    tulip_ffi_result_free(r);
    trvi_state_free(r.state);
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

    options := []float64{14.0} // period

    mask := []bool{true, true} // tr, ema

    res, st, _ := indicators.Trvi.Indicator(high, low, close, options, mask)
    fmt.Println(res.Rows[0]) // trvi (primary)
    fmt.Println(res.Rows[1]) // tr (optional — requested)
    fmt.Println(res.Rows[2]) // ema (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high  = close + 1.0
    low   = close - 1.0

    outputs, state = tulip_rs.indicators.trvi.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, True],
    )

    trvi = outputs[0]  # trvi (primary)
    tr   = outputs[1]  # tr (optional — requested)
    ema  = outputs[2]  # ema (optional — requested)
    ```

=== "Node.js"

    `trvi` exposes 2 optional outputs: `tr`, `ema`.

    ```javascript
    const [allOut] = ti.trvi.indicator([high, low, close], [14], [true, true]);
    const trvi = allOut[0]; // primary
    const tr   = allOut[1]; // optional 0: tr
    const ema  = allOut[2]; // optional 1: ema

    // Request only tr
    const [partial] = ti.trvi.indicator([high, low, close], [14], [true, false]);
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.trvi.indicator([high, low, close], [14], [true, true]);
    const trvi = allOut[0]; // primary
    const tr   = allOut[1]; // optional 0: tr
    const ema  = allOut[2]; // optional 1: ema

    // Request only tr
    const [partial] = ti.trvi.indicator([high, low, close], [14], [true, false]);
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::trvi::{Trvi, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = Trvi::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::trvi::{Trvi, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let results = Trvi::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel:

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

    const double *asset1[TRVI_INPUTS] = {h1, l1, c1};
    const double *asset2[TRVI_INPUTS] = {h2, l2, c2};
    const double *asset3[TRVI_INPUTS] = {h3, l3, c3};
    const double *asset4[TRVI_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    double options[TRVI_OPTIONS] = {14.0}; // period

    CSimdResult r = trvi_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Asset %zu trvi[0]: %.4f\n", i + 1, r.outputs[i][0][0]);
        trvi_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    #define EXPANDED_LEN (10 * 20)
    double h_exp[EXPANDED_LEN], l_exp[EXPANDED_LEN], c_exp[EXPANDED_LEN];
    for (uintptr_t i = 0; i < 20; i++) {
        for (uintptr_t j = 0; j < 10; j++) {
            h_exp[i*10+j] = high[j]; l_exp[i*10+j] = low[j]; c_exp[i*10+j] = close[j];
        }
    }
    const double *inputs[TRVI_INPUTS] = {h_exp, l_exp, c_exp};

    double o5[] = {5.0}, o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0};
    const double *const simd_opts[4] = {o5, o10, o14, o20};

    CSimdResult r = trvi_simd_by_options(inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Period %g: %.4f\n", simd_opts[i][0], r.outputs[i][0][0]);
        trvi_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    assets := [][indicators.TrviInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Trvi.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    sim2, _ := indicators.Trvi.SimdByOptions(high, low, close, [][]float64{{5}, {10}, {14}, {20}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.trvi.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.trvi.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
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
    const [results] = ti.trvi.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.trvi.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
