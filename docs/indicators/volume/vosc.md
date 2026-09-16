# VOSC — Volume Oscillator

The percentage difference between two volume moving averages. Expanding volume oscillator supports the price trend.

**Inputs:** `[volume]` | **Options:** `[short_period, long_period]` | **Outputs:** `[vosc]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::vosc::{Vosc, Indicator, TIndicatorState};

    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    // options: [short_period, long_period]
    let (outputs, mut state) = Vosc::indicator(&[volume.as_slice()], &[5.0, 10.0], None).unwrap();
    println!("{:?}", outputs[0]); // VOSC values

    // State continuation — feed new bars without reprocessing history
    let new_volume = vec![1600.0, 1250.0_f64];
    let continued = state.batch_indicator(&[new_volume.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double volume[] = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0};
    double options[VOSC_OPTIONS] = {5.0, 10.0}; // short_period, long_period
    const double *inputs[VOSC_INPUTS] = {volume};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = vosc_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the VOSC series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    vosc_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = vosc_indicator(inputs, 8, options, NULL, 0);
    double new_volume[] = {1600.0, 1250.0};
    const double *new_inputs[VOSC_INPUTS] = {new_volume};
    CBatchResult b = vosc_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> VOSC values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    vosc_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    volume := []float64{1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                        900.0, 1500.0, 1800.0, 1000.0, 1700.0}
    options := []float64{5.0, 10.0} // short_period, long_period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Vosc.Indicator(volume, options, nil)
    fmt.Println(res.Rows[0]) // VOSC values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Vosc.Indicator(volume[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(volume[8:], nil)
    fmt.Println(batch.Rows[0]) // continued VOSC values
    batch.Close()
    st2.Close()
    ```



=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.vosc.indicator([volume], [5.0, 10.0])
    print(outputs[0])  # VOSC values

    # State continuation
    new_volume = np.array([1600.0, 1250.0], dtype=np.float64)
    continued = state.batch_indicator([new_volume])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const volume = Float64Array.from([5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900]);

    const [outputs, state] = ti.vosc.indicator([volume], [5, 10]);
    console.log('VOSC:', outputs[0]);

    // State continuation
    const n = volume.length - 5;
    const [, state2] = ti.vosc.indicator([volume.slice(0, n)], [5, 10]);
    const continued = state2.batchIndicator([volume.slice(n)]);
    console.log('Continued VOSC:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.vosc.indicator([volume], [5, 10]);
    console.log('VOSC:', outputs[0]);

    // State continuation
    const n = volume.length - 5;
    const [, state2] = ti.vosc.indicator([volume.slice(0, n)], [5, 10]);
    const continued = state2.batchIndicator([volume.slice(n)]);
    console.log('Continued VOSC:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `vosc` exposes 2 optional outputs: `short_sma`, `long_sma`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::vosc::{Vosc, Indicator, TIndicatorState};

    let volume = vec![10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0_f64];

    let mask = [true, true];
    let (outputs, _state) = Vosc::indicator(&[volume.as_slice()], &[5.0, 20.0], Some(&mask)).unwrap();

    let vosc      = &outputs[0]; // vosc (primary)
    let short_sma = &outputs[1]; // short_sma (optional — requested)
    let long_sma  = &outputs[2]; // long_sma (optional — requested)
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double volume[] = {10000.0, 12000.0, 9500.0, 11000.0, 13000.0,
                       9800.0, 10500.0, 12500.0, 11800.0, 10200.0};
    double options[VOSC_OPTIONS] = {5.0, 20.0};
    const double *inputs[VOSC_INPUTS] = {volume};
    bool optional_outputs[2] = {true, true}; // short_sma, long_sma

    CIndicatorResult r = vosc_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> vosc, r.outputs[1] -> short_sma, r.outputs[2] -> long_sma */
    tulip_ffi_result_free(r);
    vosc_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    volume := []float64{10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0}
    options := []float64{5.0, 20.0} // short_period, long_period
    mask := []bool{true, true} // short_sma, long_sma

    res, st, _ := indicators.Vosc.Indicator(volume, options, mask)
    fmt.Println(res.Rows[0]) // vosc (primary)
    fmt.Println(res.Rows[1]) // short_sma (optional — requested)
    fmt.Println(res.Rows[2]) // long_sma (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    volume = np.array([10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.vosc.indicator(
        [volume], [5.0, 20.0],
        optional_outputs=[True, True],
    )

    vosc      = outputs[0]  # vosc (primary)
    short_sma = outputs[1]  # short_sma (optional — requested)
    long_sma  = outputs[2]  # long_sma (optional — requested)
    ```

=== "Node.js"

    `vosc` exposes 2 optional outputs: `short_sma`, `long_sma`.

    ```javascript
    const [allOut] = ti.vosc.indicator([volume], [5, 10], [true, true]);
    const vosc     = allOut[0]; // primary
    const shortSma = allOut[1]; // optional 0: short_sma
    const longSma  = allOut[2]; // optional 1: long_sma
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.vosc.indicator([volume], [5, 10], [true, true]);
    const vosc     = allOut[0]; // primary
    const shortSma = allOut[1]; // optional 0: short_sma
    const longSma  = allOut[2]; // optional 1: long_sma
    ```
### SIMD

=== "Rust"

    ```rust
    use tulip_rs::indicators::vosc::{Vosc, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [
        &[v1.as_slice()],
        &[v2.as_slice()],
        &[v3.as_slice()],
        &[v4.as_slice()],
    ];
    let results = Vosc::indicator_by_assets::<4>(&inputs, &[5.0, 10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::vosc::{Vosc, IndicatorByOptions};

    let opts: [&[f64; 2]; 4] = [&[3.0, 6.0], &[5.0, 10.0], &[8.0, 16.0], &[12.0, 24.0]];
    let results = Vosc::indicator_by_options::<4>(&[volume.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double v1[] = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0, 900.0, 1500.0, 1800.0, 1000.0, 1700.0};
    double v2[] = {1100.0, 1300.0, 1000.0, 1500.0, 1200.0, 800.0, 1400.0, 1700.0, 900.0, 1600.0};
    double v3[] = {1300.0, 1500.0, 1200.0, 1700.0, 1400.0, 1000.0, 1600.0, 1900.0, 1100.0, 1800.0};
    double v4[] = {1400.0, 1600.0, 1300.0, 1800.0, 1500.0, 1100.0, 1700.0, 2000.0, 1200.0, 1900.0};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[VOSC_INPUTS] = {v1};
    const double *asset2[VOSC_INPUTS] = {v2};
    const double *asset3[VOSC_INPUTS] = {v3};
    const double *asset4[VOSC_INPUTS] = {v4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = vosc_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series */
        vosc_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    assets := [][indicators.VoscInputs][]float64{
        {v1},
        {v2},
        {v3},
        {v4},
    }
    sim, _ := indicators.Vosc.SimdByAssets(assets, []float64{5.0, 10.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```go
    sim2, _ := indicators.Vosc.SimdByOptions(volume,
        [][]float64{{3.0, 7.0}, {5.0, 10.0}, {8.0, 15.0}, {10.0, 20.0}},
        nil)
    for i := range sim2.Results {
        fmt.Printf("Option set %d: %v\n", i+1, sim2.Results[i][0])
    }
    sim2.Close() // frees every lane state, then the SIMD buffers
    ```



=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [v1],
        [v2],
        [v3],
        [v4],
    ]
    outputs_list, states = tulip_rs.indicators.vosc.simd_by_assets(simd_inputs, [5.0, 10.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[3.0, 6.0], [5.0, 10.0], [8.0, 16.0], [12.0, 24.0]]
    outputs_list, states = tulip_rs.indicators.vosc.simd_by_options([volume], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [volume.slice()],
        [volume.map(v => v * 1.1)],
        [volume.map(v => v * 0.9)],
        [volume.map(v => v * 1.02)],
    ];
    const [results] = ti.vosc.simdByAssets(simdInputs, [5, 10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[3, 6], [5, 10], [8, 16], [12, 24]];
    const [results] = ti.vosc.simdByOptions([volume], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```
