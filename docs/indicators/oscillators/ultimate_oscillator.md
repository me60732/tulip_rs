# Ultimate Oscillator

Combines momentum from three different time periods (short, medium, and long) to reduce the false signals that arise from using any single timeframe alone.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[short_period, medium_period, long_period]` &nbsp;|&nbsp; **Outputs:** `[ultosc]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::ultosc::{UltOsc, TIndicatorState, Indicator};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Options: [short_period, medium_period, long_period]
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = UltOsc::indicator(&inputs, &[7.0, 14.0, 28.0], None).unwrap();
    println!("Ultimate Oscillator: {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = UltOsc::indicator(&inputs2, &[7.0, 14.0, 28.0], None).unwrap();
    println!("Partial Ultimate Oscillator: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued Ultimate Oscillator: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[ULTOSC_OPTIONS] = {7.0, 14.0, 28.0}; // short_period, medium_period, long_period
    const double *inputs[ULTOSC_INPUTS] = {high, low, close};

    /* Full computation */
    CIndicatorResult r = ultosc_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the Ultimate Oscillator series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    ultosc_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = ultosc_indicator(inputs, 8, options, NULL, 0);
    double new_high[]  = {84.55, 85.00};
    double new_low[]   = {84.15, 84.11};
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[ULTOSC_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = ultosc_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> Ultimate Oscillator values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    ultosc_state_free(p.state);
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
    options := []float64{7.0, 14.0, 28.0} // short_period, medium_period, long_period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Ultosc.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // Ultimate Oscillator values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Ultosc.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued Ultimate Oscillator values
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

    # Options: [short_period, medium_period, long_period]
    outputs, state = tulip_rs.indicators.ultosc.indicator([high, low, close], [7.0, 14.0, 28.0])
    print("Ultimate Oscillator:", outputs[0])

    # State continuation
    outputs2, state = tulip_rs.indicators.ultosc.indicator([high[:8], low[:8], close[:8]], [7.0, 14.0, 28.0])
    continued = state.batch_indicator([high[8:], low[8:], close[8:]])
    print("Continued Ultimate Oscillator:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.ultosc.indicator([high, low, close], [7, 14, 28]);
    console.log('UltOsc:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.ultosc.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [7, 14, 28]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued UltOsc:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.ultosc.indicator([high, low, close], [7, 14, 28]);
    console.log('UltOsc:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.ultosc.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [7, 14, 28]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued UltOsc:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ultosc::{UltOsc, Indicator};

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = UltOsc::indicator_by_assets::<4>(&inputs, &[7.0, 14.0, 28.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::ultosc::{UltOsc, IndicatorByOptions};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[7.0,  14.0, 28.0],
        &[5.0,  10.0, 20.0],
        &[10.0, 20.0, 40.0],
        &[4.0,  8.0,  16.0],
    ];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = UltOsc::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in parallel:

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double h2[] = {h1[0]*1.1, h1[1]*1.1, h1[2]*1.1, h1[3]*1.1, h1[4]*1.1,
                   h1[5]*1.1, h1[6]*1.1, h1[7]*1.1, h1[8]*1.1, h1[9]*1.1};
    double l2[] = {l1[0]*1.1, l1[1]*1.1, l1[2]*1.1, l1[3]*1.1, l1[4]*1.1,
                   l1[5]*1.1, l1[6]*1.1, l1[7]*1.1, l1[8]*1.1, l1[9]*1.1};
    double c2[] = {c1[0]*1.1, c1[1]*1.1, c1[2]*1.1, c1[3]*1.1, c1[4]*1.1,
                   c1[5]*1.1, c1[6]*1.1, c1[7]*1.1, c1[8]*1.1, c1[9]*1.1};
    double h3[] = {h1[0]*0.9, h1[1]*0.9, h1[2]*0.9, h1[3]*0.9, h1[4]*0.9,
                   h1[5]*0.9, h1[6]*0.9, h1[7]*0.9, h1[8]*0.9, h1[9]*0.9};
    double l3[] = {l1[0]*0.9, l1[1]*0.9, l1[2]*0.9, l1[3]*0.9, l1[4]*0.9,
                   l1[5]*0.9, l1[6]*0.9, l1[7]*0.9, l1[8]*0.9, l1[9]*0.9};
    double c3[] = {c1[0]*0.9, c1[1]*0.9, c1[2]*0.9, c1[3]*0.9, c1[4]*0.9,
                   c1[5]*0.9, c1[6]*0.9, c1[7]*0.9, c1[8]*0.9, c1[9]*0.9};
    double h4[] = {h1[0]*1.02, h1[1]*1.02, h1[2]*1.02, h1[3]*1.02, h1[4]*1.02,
                   h1[5]*1.02, h1[6]*1.02, h1[7]*1.02, h1[8]*1.02, h1[9]*1.02};
    double l4[] = {l1[0]*1.02, l1[1]*1.02, l1[2]*1.02, l1[3]*1.02, l1[4]*1.02,
                   l1[5]*1.02, l1[6]*1.02, l1[7]*1.02, l1[8]*1.02, l1[9]*1.02};
    double c4[] = {c1[0]*1.02, c1[1]*1.02, c1[2]*1.02, c1[3]*1.02, c1[4]*1.02,
                   c1[5]*1.02, c1[6]*1.02, c1[7]*1.02, c1[8]*1.02, c1[9]*1.02};

    const double *const asset1[ULTOSC_INPUTS] = {h1, l1, c1};
    const double *const asset2[ULTOSC_INPUTS] = {h2, l2, c2};
    const double *const asset3[ULTOSC_INPUTS] = {h3, l3, c3};
    const double *const asset4[ULTOSC_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = ultosc_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's ultosc series */
        ultosc_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, c2 := h1, l1, c1
    h3, l3, c3 := h1, l1, c1
    h4, l4, c4 := h1, l1, c1

    assets := [][indicators.UltoscInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Ultosc.SimdByAssets(assets, []float64{7.0, 14.0, 28.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```go
    high  := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00}
    low   := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Ultosc.SimdByOptions(high, low, close, [][]float64{{7, 14, 28}, {5, 10, 20}, {10, 20, 40}, {4, 8, 16}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Option set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [
        [high,        low,        close],
        [high + 0.5,  low + 0.5,  close + 0.5],
        [high - 0.5,  low - 0.5,  close - 0.5],
        [high * 1.01, low * 1.01, close * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.ultosc.simd_by_assets(simd_inputs, [7.0, 14.0, 28.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [7.0,  14.0, 28.0],
        [5.0,  10.0, 20.0],
        [10.0, 20.0, 40.0],
        [4.0,  8.0,  16.0],
    ]
    outputs_list, states = tulip_rs.indicators.ultosc.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.ultosc.simdByAssets(simdInputs, [7, 14, 28]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[7, 14, 28], [5, 10, 20], [10, 20, 40], [4, 8, 16]];
    const [results] = ti.ultosc.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```
