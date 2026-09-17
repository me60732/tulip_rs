# Super Smoother — Ehlers Super Smoother

A two-pole Butterworth filter with no phase lag that provides smoother output than a simple moving average with the same length; useful as a drop-in substitute for the SMA where lag is critical.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[supersmoother]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::supersmoother::{SuperSmoother, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    let (outputs, _state) = SuperSmoother::indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("Super Smoother(10): {:?}", outputs[0]);

    // State continuation
    let n = close.len() - 5;
    let partial = close[..n].to_vec();
    let (outputs2, mut state) = SuperSmoother::indicator(&[partial.as_slice()], &[10.0], None).unwrap();
    println!("Partial Super Smoother: {:?}", outputs2[0]);

    let rest = close[n..].to_vec();
    let continued = state.batch_indicator(&[rest.as_slice()], None).unwrap();
    println!("Continued Super Smoother: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[SUPERSMOOTHER_OPTIONS] = {10.0};
    const double *inputs[SUPERSMOOTHER_INPUTS] = {close};

    /* Full computation */
    CIndicatorResult r = supersmoother_indicator(inputs, 40, options, NULL, 0);
    tulip_ffi_result_free(r);
    supersmoother_state_free(r.state);

    /* Partial + continuation */
    CIndicatorResult p = supersmoother_indicator(inputs, 35, options, NULL, 0);
    const double *rest_inputs[SUPERSMOOTHER_INPUTS] = {close + 35};
    CBatchResult b = supersmoother_batch(p.state, rest_inputs, 5, NULL, 0);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    supersmoother_state_free(p.state);
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

    options := []float64{10.0} // period

    res, st, _ := indicators.Supersmoother.Indicator(close, options, nil)
    fmt.Printf("Super Smoother: %v\n", tulip.AsFloat64(res.Rows[0]))
    res.Close()
    st.Close()

    partial := close[:35]
    res2, state2, _ := indicators.Supersmoother.Indicator(partial, options, nil)
    fmt.Printf("Partial Super Smoother: %v\n", tulip.AsFloat64(res2.Rows[0]))

    continued, _ := state2.Batch(close[35:], nil)
    fmt.Printf("Continued Super Smoother: %v\n", tulip.AsFloat64(continued.Rows[0]))
    continued.Close()
    state2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    outputs, state = tulip_rs.indicators.supersmoother.indicator([close], [10.0])
    print("Super Smoother(10):", outputs[0])

    # State continuation
    partial = close[:-5]
    outputs2, state = tulip_rs.indicators.supersmoother.indicator([partial], [10.0])
    rest = close[-5:]
    continued = state.batch_indicator([rest])
    print("Continued Super Smoother:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ]);

    const [outputs, state] = ti.supersmoother.indicator([close], [10]);
    console.log('Super Smoother(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.supersmoother.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Super Smoother:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ];

    const [outputs, state] = ti.supersmoother.indicator([close], [10]);
    console.log('Super Smoother(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.supersmoother.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Super Smoother:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::supersmoother::{SuperSmoother, Indicator, TIndicatorState};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36_f64];
    let a3 = vec![78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36_f64];
    let a4 = vec![83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = SuperSmoother::indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::supersmoother::{SuperSmoother, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = SuperSmoother::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel:

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    const double *asset1[SUPERSMOOTHER_INPUTS] = {a1};
    const double *asset2[SUPERSMOOTHER_INPUTS] = {a2};
    const double *asset3[SUPERSMOOTHER_INPUTS] = {a3};
    const double *asset4[SUPERSMOOTHER_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = supersmoother_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        supersmoother_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double close_expanded[40];
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 10; j++) close_expanded[i * 10 + j] = a1[j];
    }
    const double *expanded_inputs[SUPERSMOOTHER_INPUTS] = {close_expanded};

    double o5[] = {5.0}, o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0};
    const double *const simd_opts[4] = {o5, o10, o14, o20};

    CSimdResult r = supersmoother_simd_by_options(expanded_inputs, 40, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        supersmoother_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{10.0} // period

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := close, close, close

    assets := [][indicators.SupersmootherInputs][]float64{{close}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Supersmoother.SimdByAssets(assets, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close()
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61}
    options := [][]float64{{5}, {10}, {14}, {20}}

    sim, _ := indicators.Supersmoother.SimdByOptions(close, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim.Close()
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 3.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.supersmoother.simd_by_assets(simd_inputs, [10.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.supersmoother.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v + 5.0)],
        [close.map(v => v - 3.0)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.supersmoother.simdByAssets(simdInputs, [10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.supersmoother.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
