# Williams %R

Momentum indicator measuring the current close relative to the highest high over `period` bars, scaled to a range of -100 to 0. Values near 0 indicate overbought conditions; values near -100 indicate oversold conditions.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[willr]` &nbsp;|&nbsp; **Optional:** `[min, max]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::willr::{WillR, TIndicatorState, Indicator};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = WillR::indicator(&inputs, &[14.0], None).unwrap();
    println!("Williams %R(14): {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = WillR::indicator(&inputs2, &[14.0], None).unwrap();
    println!("Partial Williams %R: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued Williams %R: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[WILLR_OPTIONS] = {14.0}; // period
    const double *inputs[WILLR_INPUTS] = {high, low, close};

    /* Full computation */
    CIndicatorResult r = willr_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the Williams %R(14) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    willr_state_free(r.state);

    /* Partial computation + state continuation */
    const double *partial_inputs[WILLR_INPUTS] = {high, low, close};
    CIndicatorResult p = willr_indicator(partial_inputs, 8, options, NULL, 0);
    double new_high[] = {84.84, 85.00};
    double new_low[] = {84.15, 84.11};
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[WILLR_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = willr_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> Williams %R values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    willr_state_free(p.state);
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
    res, st, _ := indicators.Willr.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // Williams %R(14) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Willr.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued Williams %R values
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

    outputs, state = tulip_rs.indicators.willr.indicator([high, low, close], [14.0])
    print("Williams %R(14):", outputs[0])

    # State continuation
    outputs2, state = tulip_rs.indicators.willr.indicator([high[:8], low[:8], close[:8]], [14.0])
    continued = state.batch_indicator([high[8:], low[8:], close[8:]])
    print("Continued Williams %R:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.willr.indicator([high, low, close], [14]);
    console.log('Williams %R(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.willr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued %R:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.willr.indicator([high, low, close], [14]);
    console.log('Williams %R(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.willr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued %R:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `willr` exposes 2 optional outputs: `min`, `max`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::willr::{WillR, TIndicatorState, Indicator};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true];
    let (outputs, _state) = WillR::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let willr = &outputs[0]; // willr (primary)
    let min   = &outputs[1]; // min (optional — requested)
    let max   = &outputs[2]; // max (optional — requested)
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[WILLR_OPTIONS] = {14.0}; // period
    const double *inputs[WILLR_INPUTS] = {high, low, close};

    /* Request all optional outputs: min, max (2 optional) */
    bool optional_outputs[2] = {true, true};
    CIndicatorResult r = willr_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> willr (primary), length r.output_lens[0] */
    /* r.outputs[1] -> min (optional — requested) */
    /* r.outputs[2] -> max (optional — requested) */
    tulip_ffi_result_free(r);
    willr_state_free(r.state);
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

    mask := []bool{true, true} // one per optional output (min, max)

    // Full computation with optional outputs.
    res, st, _ := indicators.Willr.Indicator(high, low, close, options, mask)
    fmt.Println(res.Rows[0]) // willr (primary)
    fmt.Println(res.Rows[1]) // min (optional — requested)
    fmt.Println(res.Rows[2]) // max (optional — requested)
    res.Close()
    st.Close()
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

    outputs, state = tulip_rs.indicators.willr.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, True],
    )

    willr = outputs[0]  # willr (primary)
    min_  = outputs[1]  # min (optional — requested)
    max_  = outputs[2]  # max (optional — requested)
    ```

=== "Node.js"

    `willr` exposes 2 optional outputs: `min`, `max`.

    ```javascript
    const [allOut] = ti.willr.indicator([high, low, close], [14], [true, true]);
    const willr = allOut[0]; // primary
    const min   = allOut[1]; // optional 0: min
    const max   = allOut[2]; // optional 1: max
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.willr.indicator([high, low, close], [14], [true, true]);
    const willr = allOut[0]; // primary
    const min   = allOut[1]; // optional 0: min
    const max   = allOut[2]; // optional 1: max
    ```
### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::willr::{WillR, Indicator};

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

    let results = WillR::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::willr::{WillR, IndicatorByOptions};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = WillR::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    double h2[] = {83.15, 82.89, 84.03, 84.30, 84.85, 84.90, 84.33, 85.30, 85.84, 86.00};
    double l2[] = {82.29, 81.64, 82.31, 83.65, 84.07, 84.11, 83.49, 83.30, 85.15, 85.11};
    double c2[] = {82.59, 82.06, 83.87, 84.00, 84.61, 84.15, 83.84, 84.99, 85.55, 85.36};

    double h3[] = {84.15, 83.89, 85.03, 85.30, 85.85, 85.90, 85.33, 86.30, 86.84, 87.00};
    double l3[] = {83.29, 82.64, 83.31, 84.65, 85.07, 85.11, 84.49, 84.30, 86.15, 86.11};
    double c3[] = {83.59, 83.06, 84.87, 85.00, 85.61, 85.15, 84.84, 85.99, 86.55, 86.36};

    double h4[] = {85.15, 84.89, 86.03, 86.30, 86.85, 86.90, 86.33, 87.30, 87.84, 88.00};
    double l4[] = {84.29, 83.64, 84.31, 85.65, 86.07, 86.11, 85.49, 85.30, 87.15, 87.11};
    double c4[] = {84.59, 84.06, 85.87, 86.00, 86.61, 86.15, 85.84, 86.99, 87.55, 87.36};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[WILLR_INPUTS] = {h1, l1, c1};
    const double *asset2[WILLR_INPUTS] = {h2, l2, c2};
    const double *asset3[WILLR_INPUTS] = {h3, l3, c3};
    const double *asset4[WILLR_INPUTS] = {h4, l4, c4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    double options[WILLR_OPTIONS] = {14.0}; // same period for all assets
    CSimdResult r = willr_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's Williams %R series, length r.output_lens[i][0] */
        willr_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double h[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                  83.90, 83.33, 84.30, 84.84, 85.00};
    double l[] = {81.29, 80.64, 81.31, 82.65, 83.07,
                  83.11, 82.49, 82.30, 84.15, 84.11};
    double c[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                  83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[WILLR_INPUTS] = {h, l, c};

    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = willr_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for period set i (periods 7/14/21/28) */
    for (uintptr_t i = 0; i < r.num_results; i++) willr_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, c2 := h1, l1, c1
    h3, l3, c3 := h1, l1, c1
    h4, l4, c4 := h1, l1, c1

    assets := [][indicators.WillrInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Willr.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    h := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                   83.90, 83.33, 84.30, 84.84, 85.00}
    l := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                   83.11, 82.49, 82.30, 84.15, 84.11}
    c := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Willr.SimdByOptions(h, l, c, [][]float64{{7}, {14}, {21}, {28}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

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
    outputs_list, states = tulip_rs.indicators.willr.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.willr.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
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
    const [results] = ti.willr.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.willr.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
