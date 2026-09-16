# VHF — Vertical Horizontal Filter

Identifies whether the market is trending or ranging. Higher values indicate a trend; lower values suggest consolidation.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[vhf]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::vhf::{Vhf, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, mut state) = Vhf::indicator(&[close.as_slice()], &[28.0], None).unwrap();
    println!("{:?}", outputs[0]); // VHF values

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Vhf::indicator(&[partial.as_slice()], &[28.0], None).unwrap();
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
    double options[VHF_OPTIONS] = {28.0}; // period
    const double *inputs[VHF_INPUTS] = {close};

    /* Full computation */
    CIndicatorResult r = vhf_indicator(inputs, 10, options, NULL, 0);
    printf("VHF[0]: %.4f\n", r.outputs[0][0]); // outputs[0] is the VHF series
    tulip_ffi_result_free(r);
    vhf_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = vhf_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.53};
    const double *new_inputs[VHF_INPUTS] = {new_close};
    CBatchResult b = vhf_batch(p.state, new_inputs, 1, NULL, 0);
    printf("Continued VHF[0]: %.4f\n", b.outputs[0][0]);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    vhf_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{28.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Vhf.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // VHF values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Vhf.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued VHF values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.vhf.indicator([close], [28.0])
    print(outputs[0])  # VHF values

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

    const [outputs, state] = ti.vhf.indicator([close], [28]);
    console.log('VHF(28):', outputs[0]);

    // State continuation
    const [, state2] = ti.vhf.indicator([close.slice(0, -5)], [28]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued VHF:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.vhf.indicator([close], [28]);
    console.log('VHF(28):', outputs[0]);

    // State continuation
    const [, state2] = ti.vhf.indicator([close.slice(0, -5)], [28]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued VHF:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::vhf::{Vhf, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = Vhf::indicator_by_assets::<4>(&inputs, &[28.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::vhf::{Vhf, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[14.0], &[21.0], &[28.0], &[55.0]];
    let results = Vhf::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel:

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    const double *asset1[VHF_INPUTS] = {a1};
    const double *asset2[VHF_INPUTS] = {a2};
    const double *asset3[VHF_INPUTS] = {a3};
    const double *asset4[VHF_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    double options[VHF_OPTIONS] = {28.0}; // period

    CSimdResult r = vhf_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Asset %zu VHF[0]: %.4f\n", i + 1, r.outputs[i][0][0]);
        vhf_state_free(r.states[i]);
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
    const double *inputs[VHF_INPUTS] = {close_exp};

    double o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0}, o55[] = {55.0};
    const double *const simd_opts[4] = {o14, o21, o28, o55};

    CSimdResult r = vhf_simd_by_options(inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Period %g: %.4f\n", simd_opts[i][0], r.outputs[i][0][0]);
        vhf_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    assets := [][indicators.VhfInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Vhf.SimdByAssets(assets, []float64{28.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    sim2, _ := indicators.Vhf.SimdByOptions(close, [][]float64{{14}, {21}, {28}, {55}}, nil)
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
    outputs_list, states = tulip_rs.indicators.vhf.simd_by_assets(simd_inputs, [28.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[14.0], [21.0], [28.0], [55.0]]
    outputs_list, states = tulip_rs.indicators.vhf.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.vhf.simdByAssets(simdInputs, [28]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[14], [21], [28], [55]];
    const [results] = ti.vhf.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
