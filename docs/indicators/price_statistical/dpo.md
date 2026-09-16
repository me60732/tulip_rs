# DPO — Detrended Price Oscillator — `dpo`

Removes the trend from price by comparing it to a displaced moving average, highlighting underlying cycles.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[dpo]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::dpo::{Dpo, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _) = Dpo::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Dpo::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial DPO: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued DPO: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[DPO_OPTIONS] = {14.0}; // period
    const double *inputs[DPO_INPUTS] = {close};

    /* Full computation (with optional outputs: sma) */
    bool optional_outputs[1] = {true}; // sma
    CIndicatorResult r = dpo_indicator(inputs, 10, options, optional_outputs, 1);
    /* r.outputs[0] -> the DPO series, length r.output_lens[0] */
    /* r.outputs[1] -> SMA (optional — requested) */
    tulip_ffi_result_free(r);
    dpo_state_free(r.state);

    /* Partial computation + state continuation */
    const double *partial_inputs[DPO_INPUTS] = {close};
    CIndicatorResult p = dpo_indicator(partial_inputs, 8, options, NULL, 0);
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[DPO_INPUTS] = {new_close};
    CBatchResult b = dpo_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> DPO values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    dpo_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Dpo.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // DPO(14) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Dpo.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued DPO values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.dpo.indicator([close], [14.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.dpo.indicator([close], [14]);
    console.log('DPO(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.dpo.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued DPO:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.dpo.indicator([close], [14]);
    console.log('DPO(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.dpo.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued DPO:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `dpo` exposes 1 optional output: `sma`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::dpo::{Dpo, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // one per optional output
    let (outputs, _state) = Dpo::indicator(&[close.as_slice()], &[14.0], Some(&mask)).unwrap();

    let dpo = &outputs[0]; // dpo (primary)
    let sma = &outputs[1]; // sma (optional — requested)
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[DPO_OPTIONS] = {14.0}; // period
    const double *inputs[DPO_INPUTS] = {close};

    /* Request optional output: sma (1 optional) */
    bool optional_outputs[1] = {true};
    CIndicatorResult r = dpo_indicator(inputs, 10, options, optional_outputs, 1);
    /* r.outputs[0] -> dpo (primary), length r.output_lens[0] */
    /* r.outputs[1] -> sma (optional — requested) */
    tulip_ffi_result_free(r);
    dpo_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0} // period

    mask := []bool{true} // one per optional output (sma)

    // Full computation with optional outputs.
    res, st, _ := indicators.Dpo.Indicator(close, options, mask)
    fmt.Println(res.Rows[0]) // dpo (primary)
    fmt.Println(res.Rows[1]) // sma (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.dpo.indicator(
        [close], [14.0],
        optional_outputs=[True],
    )

    dpo = outputs[0]  # dpo (primary)
    sma = outputs[1]  # sma (optional — requested)
    ```

=== "Node.js"

    `dpo` exposes 1 optional output: `sma`.

    ```javascript
    const [allOut] = ti.dpo.indicator([close], [14], [true]);
    const dpo = allOut[0]; // primary
    const sma = allOut[1]; // optional 0: sma
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.dpo.indicator([close], [14], [true]);
    const dpo = allOut[0]; // primary
    const sma = allOut[1]; // optional 0: sma
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::dpo::{Dpo, Indicator};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = Dpo::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::dpo::{Dpo, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];

    let results = Dpo::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[DPO_INPUTS] = {a1};
    const double *asset2[DPO_INPUTS] = {a2};
    const double *asset3[DPO_INPUTS] = {a3};
    const double *asset4[DPO_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    double options[DPO_OPTIONS] = {14.0}; // same period for all assets
    bool optional_outputs[1] = {true}; // sma
    CSimdResult r = dpo_simd_by_assets(simd_inputs, 4, 10, options, optional_outputs, 1);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's DPO series, length r.output_lens[i][0] */
        dpo_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[DPO_INPUTS] = {close};

    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = dpo_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for period set i (periods 7/14/21/28) */
    for (uintptr_t i = 0; i < r.num_results; i++) dpo_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    assets := [][indicators.DpoInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Dpo.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    sim2, _ := indicators.Dpo.SimdByOptions(a1, [][]float64{{7}, {14}, {21}, {28}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.dpo.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.dpo.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.dpo.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.dpo.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
