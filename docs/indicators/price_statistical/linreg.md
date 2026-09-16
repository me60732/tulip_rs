# LinReg — Linear Regression — `linreg`

The end-point of a least-squares linear regression line fitted to the last `period` bars. Often used as a low-lag trend line.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[linreg]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::linreg::{Linreg, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _) = Linreg::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Linreg::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial LinReg: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued LinReg: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[LINREG_OPTIONS] = {14.0}; // period
    const double *inputs[LINREG_INPUTS] = {close};

    /* Request all optional outputs: linregslope, linregintercept (2 optional) */
    bool optional_outputs[2] = {true, true};
    CIndicatorResult r = linreg_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> linreg (primary), length r.output_lens[0] */
    /* r.outputs[1] -> linregslope (optional — requested) */
    /* r.outputs[2] -> linregintercept (optional — requested) */
    tulip_ffi_result_free(r);
    linreg_state_free(r.state);

    /* Partial computation + state continuation */
    const double *partial_inputs[LINREG_INPUTS] = {close};
    CIndicatorResult p = linreg_indicator(partial_inputs, 8, options, NULL, 0);
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[LINREG_INPUTS] = {new_close};
    CBatchResult b = linreg_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> LinReg values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    linreg_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Linreg.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // LinReg(14) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Linreg.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued LinReg values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.linreg.indicator([close], [14.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.linreg.indicator([close], [14]);
    console.log('LinReg(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.linreg.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued LinReg:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.linreg.indicator([close], [14]);
    console.log('LinReg(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.linreg.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued LinReg:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `linreg` exposes 2 optional outputs: `linregslope`, `linregintercept`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::linreg::{Linreg, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true]; // one per optional output
    let (outputs, _state) = Linreg::indicator(&[close.as_slice()], &[14.0], Some(&mask)).unwrap();

    let linreg          = &outputs[0]; // linreg (primary)
    let linregslope     = &outputs[1]; // linregslope (optional — requested)
    let linregintercept = &outputs[2]; // linregintercept (optional — requested)
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[LINREG_OPTIONS] = {14.0}; // period
    const double *inputs[LINREG_INPUTS] = {close};

    /* Request all optional outputs: linregslope, linregintercept (2 optional) */
    bool optional_outputs[2] = {true, true};
    CIndicatorResult r = linreg_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> linreg (primary), length r.output_lens[0] */
    /* r.outputs[1] -> linregslope (optional — requested) */
    /* r.outputs[2] -> linregintercept (optional — requested) */
    tulip_ffi_result_free(r);
    linreg_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0} // period

    mask := []bool{true, true} // one per optional output (linregslope, linregintercept)

    // Full computation with optional outputs.
    res, st, _ := indicators.Linreg.Indicator(close, options, mask)
    fmt.Println(res.Rows[0]) // linreg (primary)
    fmt.Println(res.Rows[1]) // linregslope (optional — requested)
    fmt.Println(res.Rows[2]) // linregintercept (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.linreg.indicator(
        [close], [14.0],
        optional_outputs=[True, True],
    )

    linreg          = outputs[0]  # linreg (primary)
    linregslope     = outputs[1]  # linregslope (optional — requested)
    linregintercept = outputs[2]  # linregintercept (optional — requested)
    ```

=== "Node.js"

    `linreg` exposes 2 optional outputs: `linregslope`, `linregintercept`.

    ```javascript
    const [allOut] = ti.linreg.indicator([close], [14], [true, true]);
    const linreg          = allOut[0]; // primary
    const linregslope     = allOut[1]; // optional 0: linregslope
    const linregintercept = allOut[2]; // optional 1: linregintercept
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.linreg.indicator([close], [14], [true, true]);
    const linreg          = allOut[0]; // primary
    const linregslope     = allOut[1]; // optional 0: linregslope
    const linregintercept = allOut[2]; // optional 1: linregintercept
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::linreg::{Linreg, Indicator};

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

    let results = Linreg::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::linreg::{Linreg, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];

    let results = Linreg::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
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
    const double *asset1[LINREG_INPUTS] = {a1};
    const double *asset2[LINREG_INPUTS] = {a2};
    const double *asset3[LINREG_INPUTS] = {a3};
    const double *asset4[LINREG_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    double options[LINREG_OPTIONS] = {14.0}; // same period for all assets
    bool optional_outputs[2] = {true, true};
    CSimdResult r = linreg_simd_by_assets(simd_inputs, 4, 10, options, optional_outputs, 2);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's LinReg series, length r.output_lens[i][0] */
        linreg_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[LINREG_INPUTS] = {close};

    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = linreg_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for period set i (periods 7/14/21/28) */
    for (uintptr_t i = 0; i < r.num_results; i++) linreg_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    assets := [][indicators.LinregInputs][]float64{{a1}, {b1}, {c1}, {d1}}
    sim, _ := indicators.Linreg.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    sim2, _ := indicators.Linreg.SimdByOptions(a1, [][]float64{{7}, {14}, {21}, {28}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.linreg.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.linreg.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.linreg.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.linreg.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
