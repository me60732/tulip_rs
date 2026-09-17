# StdDev — Standard Deviation

Rolling standard deviation of the price series over `period` bars.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[stddev]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::stddev::{StdDev, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, mut state) = StdDev::indicator(&[close.as_slice()], &[20.0], None).unwrap();
    println!("{:?}", outputs[0]); // StdDev values

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = StdDev::indicator(&[partial.as_slice()], &[20.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[STDDEV_OPTIONS] = {20.0}; // period
    const double *inputs[STDDEV_INPUTS] = {close};

    /* Full computation */
    CIndicatorResult r = stddev_indicator(inputs, 10, options, NULL, 0);
    printf("StdDev[0]: %.4f\n", r.outputs[0][0]); // outputs[0] is the stddev series
    tulip_ffi_result_free(r);
    stddev_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = stddev_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.53};
    const double *new_inputs[STDDEV_INPUTS] = {new_close};
    CBatchResult b = stddev_batch(p.state, new_inputs, 1, NULL, 0);
    printf("Continued StdDev[0]: %.4f\n", b.outputs[0][0]);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    stddev_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{20.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Stddev.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // StdDev values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Stddev.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued StdDev values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.stddev.indicator([close], [20.0])
    print(outputs[0])  # StdDev values

    # State continuation
    new_close = np.array([85.10, 85.72], dtype=np.float64)
    continued = state.batch_indicator([new_close])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.stddev.indicator([close], [20]);
    console.log('StdDev(20):', outputs[0]);

    // State continuation
    const [, state2] = ti.stddev.indicator([close.slice(0, -5)], [20]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued StdDev:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.stddev.indicator([close], [20]);
    console.log('StdDev(20):', outputs[0]);

    // State continuation
    const [, state2] = ti.stddev.indicator([close.slice(0, -5)], [20]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued StdDev:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `stddev` exposes 1 optional output: `"sma"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::stddev::{StdDev, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // request sma
    let (outputs, _state) = StdDev::indicator(&[close.as_slice()], &[5.0], Some(&mask)).unwrap();

    let stddev = &outputs[0]; // stddev (primary)
    let sma    = &outputs[1]; // sma    (optional — requested)
    ```

=== "C"

    `stddev` exposes 1 optional output: `sma`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[STDDEV_OPTIONS] = {5.0}; // period
    const double *inputs[STDDEV_INPUTS] = {close};
    bool optional_outputs[1] = {true}; // request sma

    CIndicatorResult r = stddev_indicator(inputs, 10, options, optional_outputs, 1);
    printf("stddev[0]: %.4f\n", r.outputs[0][0]); // primary
    printf("sma[0]:    %.4f\n", r.outputs[1][0]);  // optional 0: sma
    tulip_ffi_result_free(r);
    stddev_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    options := []float64{5.0} // period

    mask := []bool{true} // request sma

    res, st, _ := indicators.Stddev.Indicator(close, options, mask)
    fmt.Println(res.Rows[0]) // stddev (primary)
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

    outputs, state = tulip_rs.indicators.stddev.indicator(
        [close], [5.0],
        optional_outputs=[True],
    )

    stddev = outputs[0]  # stddev (primary)
    sma    = outputs[1]  # sma    (optional — requested)
    ```

=== "Node.js"

    `stddev` exposes 1 optional output: `sma`.

    ```javascript
    const [allOut] = ti.stddev.indicator([close], [20], [true]);
    const stddev = allOut[0]; // primary
    const sma    = allOut[1]; // optional 0: sma
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.stddev.indicator([close], [20], [true]);
    const stddev = allOut[0]; // primary
    const sma    = allOut[1]; // optional 0: sma
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::stddev::{StdDev, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = StdDev::indicator_by_assets::<4>(&inputs, &[20.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::stddev::{StdDev, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[10.0], &[20.0], &[30.0], &[50.0]];
    let results = StdDev::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in parallel:

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    const double *asset1[STDDEV_INPUTS] = {a1};
    const double *asset2[STDDEV_INPUTS] = {a2};
    const double *asset3[STDDEV_INPUTS] = {a3};
    const double *asset4[STDDEV_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    double options[STDDEV_OPTIONS] = {20.0}; // period

    CSimdResult r = stddev_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Asset %zu stddev[0]: %.4f\n", i + 1, r.outputs[i][0][0]);
        stddev_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    #define EXPANDED_LEN (10 * 20)
    double close_exp[EXPANDED_LEN];
    for (uintptr_t i = 0; i < 20; i++) {
        for (uintptr_t j = 0; j < 10; j++) {
            close_exp[i*10+j] = close[j];
        }
    }
    const double *inputs[STDDEV_INPUTS] = {close_exp};

    double o10[] = {10.0}, o20[] = {20.0}, o30[] = {30.0}, o50[] = {50.0};
    const double *const simd_opts[4] = {o10, o20, o30, o50};

    CSimdResult r = stddev_simd_by_options(inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Period %g: %.4f\n", simd_opts[i][0], r.outputs[i][0][0]);
        stddev_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    real1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    real2 := real1
    real3 := real1
    real4 := real1

    assets := [][indicators.StddevInputs][]float64{{real1}, {real2}, {real3}, {real4}}
    sim, _ := indicators.Stddev.SimdByAssets(assets, []float64{20.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Stddev.SimdByOptions(close, [][]float64{{10}, {20}, {30}, {50}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [np.array(asset1_close, dtype=np.float64)],
        [np.array(asset2_close, dtype=np.float64)],
        [np.array(asset3_close, dtype=np.float64)],
        [np.array(asset4_close, dtype=np.float64)],
    ]
    outputs_list, states = tulip_rs.indicators.stddev.simd_by_assets(simd_inputs, [20.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[10.0], [20.0], [30.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.stddev.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.stddev.simdByAssets(simdInputs, [20]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[10], [20], [30], [50]];
    const [results] = ti.stddev.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
