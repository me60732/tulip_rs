# Elder-Ray

Splits market force into two components relative to an EMA of close. Bull power = high − EMA, measuring how far above the average buyers can push price; Bear power = low − EMA, measuring how far below sellers can push it. Positive bull power alongside a rising EMA confirms bullish momentum; negative bear power with a falling EMA confirms bearish pressure. The EMA itself is available as an optional overlay output.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[bull, bear]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::elderray::{ElderRay, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = ElderRay::indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // bull power
    println!("{:?}", outputs[1]); // bear power

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = ElderRay::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]); // bull power
    println!("{:?}", outputs2[1]); // bear power

    let new_high  = vec![86.54_f64];
    let new_low   = vec![85.39_f64];
    let new_close = vec![86.53_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]); // continued bull power
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29};
    double options[ELDERRAY_OPTIONS] = {14.0}; // period
    const double *inputs[ELDERRAY_INPUTS] = {high, low, close};

    /* Full computation */
    CIndicatorResult r = elderray_indicator(inputs, 15, options, NULL, 0);
    tulip_ffi_result_free(r);
    elderray_state_free(r.state);

    /* Partial + continuation */
    CIndicatorResult p = elderray_indicator(inputs, 8, options, NULL, 0);
    const double *rest_inputs[ELDERRAY_INPUTS] = {high + 8, low + 8, close + 8};
    CBatchResult b = elderray_batch(p.state, rest_inputs, 7, NULL, 0);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    elderray_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high  := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87}
    low   := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29}
    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Elderray.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // bull power
    fmt.Println(res.Rows[1]) // bear power
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Elderray.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued bull power
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    outputs, state = tulip_rs.indicators.elderray.indicator(
        [high, low, close], [14.0]
    )
    print(outputs[0])  # bull power
    print(outputs[1])  # bear power

    # State continuation
    new_high  = np.array([88.50], dtype=np.float64)
    new_low   = np.array([87.30], dtype=np.float64)
    new_close = np.array([88.10], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])  # continued bull power
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.elderray.indicator([high, low, close], [14]);
    console.log('Bull power:', outputs[0]);
    console.log('Bear power:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.elderray.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued bull power:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.elderray.indicator([high, low, close], [14]);
    console.log('Bull power:', outputs[0]);
    console.log('Bear power:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.elderray.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued bull power:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `elderray` exposes 1 optional output: `ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::elderray::{ElderRay, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let mask = [true];
    let (outputs, _state) = ElderRay::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let bull = &outputs[0]; // bull (primary)
    let bear = &outputs[1]; // bear (primary)
    let ema  = &outputs[2]; // ema (optional — requested)
    ```

=== "C"

    `elderray` exposes 1 optional output: `ema`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    // ... (same high, low, close data as above)
    bool optional_outputs[1] = {true}; // ema

    CIndicatorResult r = elderray_indicator(inputs, 15, options, optional_outputs, 1);
    /* r.outputs[0] -> bull (primary) */
    /* r.outputs[1] -> bear (primary) */
    /* r.outputs[2] -> ema (optional — requested) */
    tulip_ffi_result_free(r);
    elderray_state_free(r.state);
    ```

=== "Go"

    `elderray` exposes 1 optional output: `ema`. Pass a boolean mask as the third argument.

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    // ... (same high, low, close data as above)
    mask := []bool{true} // ema

    res, st, _ := indicators.Elderray.Indicator(high, low, close, options, mask)
    bull := res.Rows[0] // bull (primary)
    bear := res.Rows[1] // bear (primary)
    ema  := res.Rows[2] // ema (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    outputs, state = tulip_rs.indicators.elderray.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True],
    )

    bull = outputs[0]  # bull (primary)
    bear = outputs[1]  # bear (primary)
    ema  = outputs[2]  # ema (optional — requested)
    ```

=== "Node.js"

    `elderray` exposes 1 optional output: `ema`.

    ```javascript
    const [allOut] = ti.elderray.indicator([high, low, close], [14], [true]);
    const bull = allOut[0]; // primary
    const bear = allOut[1]; // primary
    const ema  = allOut[2]; // optional 0: ema
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.elderray.indicator([high, low, close], [14], [true]);
    const bull = allOut[0]; // primary
    const bear = allOut[1]; // primary
    const ema  = allOut[2]; // optional 0: ema
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::elderray::{ElderRay, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = ElderRay::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: bull={:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::elderray::{ElderRay, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = ElderRay::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: bull={:?}", opts[i][0], out[0]);
    }
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
    outputs_list, states = tulip_rs.indicators.elderray.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: bull={asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.elderray.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: bull={out[0]}")
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel:

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29};

    double h2[15], l2[15], c2[15];
    for (int i = 0; i < 15; i++) { h2[i] = h1[i] * 1.2; l2[i] = l1[i] * 1.2; c2[i] = c1[i] * 1.2; }
    double h3[15], l3[15], c3[15];
    for (int i = 0; i < 15; i++) { h3[i] = 90.0 + i * 0.5 + h1[i] * 0.1; l3[i] = 90.0 + i * 0.5 + l1[i] * 0.1; c3[i] = 90.0 + i * 0.5 + c1[i] * 0.1; }
    double h4[15], l4[15], c4[15];
    for (int i = 0; i < 15; i++) { h4[i] = 100.0 - i * 0.3 + h1[i] * 0.05; l4[i] = 100.0 - i * 0.3 + l1[i] * 0.05; c4[i] = 100.0 - i * 0.3 + c1[i] * 0.05; }

    const double *asset1[ELDERRAY_INPUTS] = {h1, l1, c1};
    const double *asset2[ELDERRAY_INPUTS] = {h2, l2, c2};
    const double *asset3[ELDERRAY_INPUTS] = {h3, l3, c3};
    const double *asset4[ELDERRAY_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = elderray_simd_by_assets(simd_inputs, 4, 15, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        elderray_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double high_expanded[60], low_expanded[60], close_expanded[60];
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 15; j++) {
            high_expanded[i * 15 + j] = h1[j];
            low_expanded[i * 15 + j] = l1[j];
            close_expanded[i * 15 + j] = c1[j];
        }
    }
    const double *expanded_inputs[ELDERRAY_INPUTS] = {high_expanded, low_expanded, close_expanded};

    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = elderray_simd_by_options(expanded_inputs, 60, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        elderray_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    assets := [][indicators.ElderrayInputs][]float64{
        {high, low, close},
        {demo.Scale(high, 1.2), demo.Scale(low, 1.2), demo.Scale(close, 1.2)},
        {demo.Scale(high, 0.9), demo.Scale(low, 0.9), demo.Scale(close, 0.9)},
        {demo.Scale(high, 1.05), demo.Scale(low, 1.05), demo.Scale(close, 1.05)},
    }
    sim, _ := indicators.Elderray.SimdByAssets(assets, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d bull: %v
", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    sim2, _ := indicators.Elderray.SimdByOptions(high, low, close,
        [][]float64{{7.0}, {14.0}, {21.0}, {28.0}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period %d bull: %v
", []float64{7.0, 14.0, 21.0, 28.0}[i][0], lanes[0])
    }
    sim2.Close()
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
    const [results] = ti.elderray.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}: bull=`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.elderray.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
