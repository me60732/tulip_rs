# INSTANTANEOUSTRENDLINE — Ehlers Instantaneous Trendline

Extracts the underlying trend from price by suppressing cycle-mode components using the dominant cycle period; adapts its smoothing factor dynamically based on instantaneous frequency.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[]` (none) &nbsp;|&nbsp; **Outputs:** `[trendline]` &nbsp;|&nbsp; **Optional:** `[trigger, dc_period, alpha]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::instantaneoustrendline::{InstantaneousTrendline, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    // instantaneoustrendline takes no options — pass an empty slice
    let (outputs, _state) = InstantaneousTrendline::indicator(&[close.as_slice()], &[], None).unwrap();
    println!("Trendline: {:?}", outputs[0]);

    // State continuation
    let partial = close[..35].to_vec();
    let (outputs2, mut state) = InstantaneousTrendline::indicator(&[partial.as_slice()], &[], None).unwrap();
    println!("Partial Trendline: {:?}", outputs2[0]);

    let new_close = close[35..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued Trendline: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    # instantaneoustrendline takes no options — pass an empty list
    outputs, state = tulip_rs.indicators.instantaneoustrendline.indicator([close], [])
    print("Trendline:", outputs[0])

    # State continuation
    partial = close[:35]
    outputs2, state = tulip_rs.indicators.instantaneoustrendline.indicator([partial], [])
    new_close = close[35:]
    continued = state.batch_indicator([new_close])
    print("Continued Trendline:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20]);

    const [outputs, state] = ti.instantaneoustrendline.indicator([close], []);
    console.log('Trendline:', outputs[0]);

    // State continuation
    const [, state2] = ti.instantaneoustrendline.indicator([close.slice(0, -5)], []);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Trendline:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init, instantaneoustrendline } from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20];

    const [outputs, state] = instantaneoustrendline.indicator([close], []);
    console.log('Trendline:', outputs[0]);

    // State continuation
    const [, state2] = instantaneoustrendline.indicator([close.slice(0, -5)], []);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Trendline:', continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    const double options[INSTANTANEOUSTRENDLINE_OPTIONS] = {}; // no options
    const double *inputs[INSTANTANEOUSTRENDLINE_INPUTS] = {close};

    /* Full computation with all optional outputs */
    bool optional_outputs[4] = {true, true, true, true}; // trigger, dc_period, alpha
    CIndicatorResult r = instantaneoustrendline_indicator(inputs, 40, options, optional_outputs, 4);
    /* r.outputs[0] -> trendline (primary) */
    /* r.outputs[1] -> trigger (optional) */
    /* r.outputs[2] -> dc_period (optional) */
    /* r.outputs[3] -> alpha (optional) */
    tulip_ffi_result_free(r);
    instantaneoustrendline_state_free(r.state);

    /* Partial computation without optional outputs + state continuation */
    CIndicatorResult p = instantaneoustrendline_indicator(inputs, 35, options, NULL, 0);
    double new_close[] = {89.70, 90.10, 89.50, 90.20, 90.80};
    const double *new_inputs[INSTANTANEOUSTRENDLINE_INPUTS] = {new_close};
    CBatchResult b = instantaneoustrendline_batch(p.state, new_inputs, 5, NULL, 0);
    /* b.outputs[0] -> trendline for new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    instantaneoustrendline_state_free(p.state);
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}

    // instantaneoustrendline takes no options — pass an empty slice
    res, st, _ := indicators.Instantaneoustrendline.Indicator(close, []float64{}, nil)
    fmt.Printf("Trendline: %v\n", tulip.AsFloat64(res.Rows[0]))
    res.Close()
    st.Close()

    // State continuation
    partial := close[:35]
    res2, state2, _ := indicators.Instantaneoustrendline.Indicator(partial, []float64{}, nil)
    fmt.Printf("Partial Trendline: %v\n", tulip.AsFloat64(res2.Rows[0]))

    continued, _ := state2.Batch(close[35:], nil)
    fmt.Printf("Continued Trendline: %v\n", tulip.AsFloat64(continued.Rows[0]))
    continued.Close()
    state2.Close()
    ```

### Optional Outputs

=== "Rust"

    `instantaneoustrendline` exposes 3 optional outputs: `trigger`, `dc_period`, `alpha`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::instantaneoustrendline::{InstantaneousTrendline, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    let mask = [true, true, true]; // one per optional output
    let (outputs, _state) = InstantaneousTrendline::indicator(&[close.as_slice()], &[], Some(&mask)).unwrap();

    let trendline = &outputs[0]; // trendline (primary)
    let trigger   = &outputs[1]; // trigger (optional — requested)
    let dc_period = &outputs[2]; // dc_period (optional — requested)
    let alpha     = &outputs[3]; // alpha (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    outputs, state = tulip_rs.indicators.instantaneoustrendline.indicator(
        [close], [],
        optional_outputs=[True, True, True],
    )

    trendline = outputs[0]  # trendline (primary)
    trigger   = outputs[1]  # trigger (optional — requested)
    dc_period = outputs[2]  # dc_period (optional — requested)
    alpha     = outputs[3]  # alpha (optional — requested)
    ```

=== "Node.js"

    `instantaneoustrendline` exposes 3 optional outputs: `trigger`, `dc_period`, `alpha`.

    ```javascript
    const [allOut] = ti.instantaneoustrendline.indicator([close], [], [true, true, true]);
    const trendline = allOut[0]; // primary
    const trigger   = allOut[1]; // optional 0: trigger
    const dcPeriod  = allOut[2]; // optional 1: dc_period
    const alpha     = allOut[3]; // optional 2: alpha

    // Request only trigger
    const [partial] = ti.instantaneoustrendline.indicator([close], [], [true, false, false]);
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = instantaneoustrendline.indicator([close], [], [true, true, true]);
    const trendline = allOut[0]; // primary
    const trigger   = allOut[1]; // optional 0: trigger
    const dcPeriod  = allOut[2]; // optional 1: dc_period
    const alpha     = allOut[3]; // optional 2: alpha

    // Request only trigger
    const [partial] = instantaneoustrendline.indicator([close], [], [true, false, false]);
    ```

=== "C"

    `instantaneoustrendline` exposes 3 optional outputs: `trigger`, `dc_period`, `alpha`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    const double options[INSTANTANEOUSTRENDLINE_OPTIONS] = {}; // no options
    const double *inputs[INSTANTANEOUSTRENDLINE_INPUTS] = {close};

    bool mask[4] = {true, true, true, true}; // one per optional output
    CIndicatorResult r = instantaneoustrendline_indicator(inputs, 40, options, mask, 4);

    /* r.outputs[0] -> trendline (primary) */
    /* r.outputs[1] -> trigger (optional — requested) */
    /* r.outputs[2] -> dc_period (optional — requested) */
    /* r.outputs[3] -> alpha (optional — requested) */

    tulip_ffi_result_free(r);
    instantaneoustrendline_state_free(r.state);
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}

    options := []float64{} // no options

    mask := []bool{true, true, true} // trigger, dc_period, alpha

    res, st, _ := indicators.Instantaneoustrendline.Indicator(close, options, mask)
    fmt.Printf("Trendline: %v\n", tulip.AsFloat64(res.Rows[0]))
    fmt.Printf("Trigger: %v\n", tulip.AsFloat64(res.Rows[1]))
    fmt.Printf("dc_period: %v\n", tulip.AsFloat64(res.Rows[2]))
    fmt.Printf("alpha: %v\n", tulip.AsFloat64(res.Rows[3]))
    res.Close()
    st.Close()
    ```

### SIMD

=== "Rust"

    **By assets** — applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::instantaneoustrendline::{InstantaneousTrendline Indicator};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                  85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                  88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                  90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                  77.00, 77.50, 78.00, 78.50, 79.00, 79.50, 80.00, 80.50, 81.00, 81.50,
                  82.00, 82.50, 83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50,
                  87.00, 87.50, 88.00, 88.50, 89.00, 89.50, 90.00, 90.50, 91.00, 91.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                  58.70, 59.00, 59.30, 59.60, 59.90, 60.20, 60.50, 60.80, 61.10, 61.40,
                  61.70, 62.00, 62.30, 62.60, 62.90, 63.20, 63.50, 63.80, 64.10, 64.40,
                  64.70, 65.00, 65.30, 65.60, 65.90, 66.20, 66.50, 66.80, 67.10, 67.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                  104.1, 104.5, 105.0, 105.3, 105.8, 106.0, 106.5, 107.0, 107.3, 107.8,
                  108.1, 108.5, 109.0, 109.3, 109.8, 110.0, 110.5, 111.0, 111.3, 111.8,
                  112.1, 112.5, 113.0, 113.3, 113.8, 114.0, 114.5, 115.0, 115.3, 115.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = InstantaneousTrendline::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.instantaneoustrendline.simd_by_assets(simd_inputs, [])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.instantaneoustrendline.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "C"

    **By assets** — applied to 4 assets in one call (N must be 2/4/8/16). C FFI offers only by-assets here.

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                   77.00, 77.50, 78.00, 78.50, 79.00, 79.50, 80.00, 80.50, 81.00, 81.50,
                   82.00, 82.50, 83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50,
                   87.00, 87.50, 88.00, 88.50, 89.00, 89.50, 90.00, 90.50, 91.00, 91.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                   58.70, 59.00, 59.30, 59.60, 59.90, 60.20, 60.50, 60.80, 61.10, 61.40,
                   61.70, 62.00, 62.30, 62.60, 62.90, 63.20, 63.50, 63.80, 64.10, 64.40,
                   64.70, 65.00, 65.30, 65.60, 65.90, 66.20, 66.50, 66.80, 67.10, 67.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                   104.1, 104.5, 105.0, 105.3, 105.8, 106.0, 106.5, 107.0, 107.3, 107.8,
                   108.1, 108.5, 109.0, 109.3, 109.8, 110.0, 110.5, 111.0, 111.3, 111.8,
                   112.1, 112.5, 113.0, 113.3, 113.8, 114.0, 114.5, 115.0, 115.3, 115.8};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[INSTANTANEOUSTRENDLINE_INPUTS] = {a1};
    const double *asset2[INSTANTANEOUSTRENDLINE_INPUTS] = {a2};
    const double *asset3[INSTANTANEOUSTRENDLINE_INPUTS] = {a3};
    const double *asset4[INSTANTANEOUSTRENDLINE_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    bool optional_outputs[4] = {true, true, true, true}; // trigger, dc_period, alpha
    CSimdResult r = instantaneoustrendline_simd_by_assets(simd_inputs, 4, 40, options, optional_outputs, 4);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's trendline series, length r.output_lens[i][0] */
        instantaneoustrendline_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}

    assets := [][indicators.InstantaneoustrendlineInputs][]float64{
        {close},
        {close + 5.0},
        {close - 5.0},
        {close * 1.02},
    }
    sim, _ := indicators.Instantaneoustrendline.SimdByAssets(assets, []float64{}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close()
    ```
