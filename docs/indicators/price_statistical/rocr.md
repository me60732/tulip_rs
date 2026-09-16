# ROCR — Rate of Change Ratio — `rocr`

The ratio of the current price to the price `period` bars ago (equivalent to `1 + ROC / 100`).

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[rocr]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::rocr::{Rocr, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let (outputs, mut state) = Rocr::indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Rocr::indicator(&[partial.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    **By assets** — same options applied to 4 assets in parallel:

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    const double *asset1[ROCR_INPUTS] = {a1};
    const double *asset2[ROCR_INPUTS] = {a2};
    const double *asset3[ROCR_INPUTS] = {a3};
    const double *asset4[ROCR_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = rocr_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        rocr_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, N option sets in parallel:

    ```c
    double close_expanded[40];
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 10; j++) close_expanded[i * 10 + j] = a1[j];
    }
    const double *expanded_inputs[ROCR_INPUTS] = {close_expanded};

    double o5[] = {5.0}, o10[] = {10.0}, o20[] = {20.0}, o50[] = {50.0};
    const double *const simd_opts[4] = {o5, o10, o20, o50};

    CSimdResult r = rocr_simd_by_options(expanded_inputs, 40, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        rocr_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{10.0}

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Rocr.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // ROCR(10) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Rocr.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued ROCR values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.rocr.indicator([close], [10.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.rocr.indicator([close], [10]);
    console.log('ROCR(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.rocr.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued ROCR:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.rocr.indicator([close], [10]);
    console.log('ROCR(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.rocr.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued ROCR:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::rocr::{Rocr, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = Rocr::indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::rocr::{Rocr, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[20.0], &[50.0]];
    let results = Rocr::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Option {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in parallel:

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    const double *asset1[ROCR_INPUTS] = {a1};
    const double *asset2[ROCR_INPUTS] = {a2};
    const double *asset3[ROCR_INPUTS] = {a3};
    const double *asset4[ROCR_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = rocr_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        rocr_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, N option sets in parallel:

    ```c
    double close_expanded[40];
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 10; j++) close_expanded[i * 10 + j] = a1[j];
    }
    const double *expanded_inputs[ROCR_INPUTS] = {close_expanded};

    double o5[] = {5.0}, o10[] = {10.0}, o20[] = {20.0}, o50[] = {50.0};
    const double *const simd_opts[4] = {o5, o10, o20, o50};

    CSimdResult r = rocr_simd_by_options(expanded_inputs, 40, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        rocr_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    a2 := []float64{86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36}
    a3 := []float64{78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36}
    a4 := []float64{83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05}

    assets := [][indicators.RocrInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Rocr.SimdByAssets(assets, []float64{10.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    sim2, _ := indicators.Rocr.SimdByOptions(close, [][]float64{{5}, {10}, {20}, {50}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.rocr.simd_by_assets(simd_inputs, [10.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [20.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.rocr.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.rocr.simdByAssets(simdInputs, [10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [20], [50]];
    const [results] = ti.rocr.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
