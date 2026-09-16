# VWMA — Volume Weighted Moving Average

Moving average weighted by trading volume so that high-volume bars have more influence on the average than low-volume bars.

**Inputs:** `[real, volume]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[vwma]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::vwma::{Vwma, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                      3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0_f64];

    let inputs = [close.as_slice(), volume.as_slice()];
    let (outputs, _state) = Vwma::indicator(&inputs, &[14.0], None).unwrap();
    println!("VWMA(14): {:?}", outputs[0]);

    // State continuation
    let partial_close  = close[..8].to_vec();
    let partial_volume = volume[..8].to_vec();
    let inputs2 = [partial_close.as_slice(), partial_volume.as_slice()];
    let (outputs2, mut state) = Vwma::indicator(&inputs2, &[14.0], None).unwrap();
    println!("Partial VWMA: {:?}", outputs2[0]);

    let new_close  = close[8..].to_vec();
    let new_volume = volume[8..].to_vec();
    let new_inputs = [new_close.as_slice(), new_volume.as_slice()];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued VWMA: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double volume[] = {5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                       3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0};
    double options[VWMA_OPTIONS] = {14.0}; // period
    const double *inputs[VWMA_INPUTS] = {close, volume};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = vwma_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the VWMA(14) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    vwma_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = vwma_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {84.55, 84.36};
    double new_volume[] = {4841300.0, 3915300.0};
    const double *new_inputs[VWMA_INPUTS] = {new_close, new_volume};
    CBatchResult b = vwma_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> VWMA values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    vwma_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    volume := []float64{5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                        3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0}
    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Vwma.Indicator(close, volume, options, nil)
    fmt.Println(res.Rows[0]) // VWMA(14) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Vwma.Indicator(close[:8], volume[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], volume[8:], nil)
    fmt.Println(batch.Rows[0]) // continued VWMA values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                       3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.vwma.indicator([close, volume], [14.0])
    print("VWMA(14):", outputs[0])

    # State continuation
    partial_close  = close[:8]
    partial_volume = volume[:8]
    outputs2, state = tulip_rs.indicators.vwma.indicator([partial_close, partial_volume], [14.0])
    new_close  = close[8:]
    new_volume = volume[8:]
    continued = state.batch_indicator([new_close, new_volume])
    print("Continued VWMA:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close  = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                    83.15, 82.84, 83.99, 84.55, 84.36,
                    85.53, 86.54, 86.89, 87.77, 87.29]);
    const volume = Float64Array.from([5653100, 6447400, 7690900, 3831400, 4455100,
                    3798000, 3936200, 4732000, 4841300, 3915300,
                    6830800, 6694100, 5293600, 7985800, 4807900]);

    const [outputs, state] = ti.vwma.indicator([close, volume], [14]);
    console.log('VWMA(14):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.vwma.indicator([close.slice(0, n), volume.slice(0, n)], [14]);
    const continued = state2.batchIndicator([close.slice(n), volume.slice(n)]);
    console.log('Continued VWMA:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close  = [81.59, 81.06, 82.87, 83.00, 83.61,
                    83.15, 82.84, 83.99, 84.55, 84.36,
                    85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100,
                    3798000, 3936200, 4732000, 4841300, 3915300,
                    6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.vwma.indicator([close, volume], [14]);
    console.log('VWMA(14):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.vwma.indicator([close.slice(0, n), volume.slice(0, n)], [14]);
    const continued = state2.batchIndicator([close.slice(n), volume.slice(n)]);
    console.log('Continued VWMA:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets (each with close + volume) in parallel:

    ```rust
    use tulip_rs::indicators::vwma::{Vwma, Indicator};

    let a1_close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a1_vol    = vec![5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                         3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0_f64];
    let a2_close  = a1_close.iter().map(|x| x + 5.0).collect::<Vec<_>>();
    let a2_vol    = a1_vol.clone();
    let a3_close  = a1_close.iter().map(|x| x - 5.0).collect::<Vec<_>>();
    let a3_vol    = a1_vol.clone();
    let a4_close  = a1_close.iter().map(|x| x * 1.02).collect::<Vec<_>>();
    let a4_vol    = a1_vol.clone();

    let inputs: [&[&[f64]; 2]; 4] = [
        &[a1_close.as_slice(), a1_vol.as_slice()],
        &[a2_close.as_slice(), a2_vol.as_slice()],
        &[a3_close.as_slice(), a3_vol.as_slice()],
        &[a4_close.as_slice(), a4_vol.as_slice()],
    ];

    let results = Vwma::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::vwma::{Vwma, IndicatorByOptions};

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                      3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = Vwma::indicator_by_options::<4>(&[close.as_slice(), volume.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets (each with close + volume) in one call (N must be 2/4/8/16):

    ```c
    double a1_close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a1_vol[]   = {5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                         3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0};
    double a2_close[] = {a1_close[0]*1.1, a1_close[1]*1.1, a1_close[2]*1.1,
                         a1_close[3]*1.1, a1_close[4]*1.1, a1_close[5]*1.1,
                         a1_close[6]*1.1, a1_close[7]*1.1, a1_close[8]*1.1,
                         a1_close[9]*1.1};
    double a2_vol[]   = {a1_vol[0]*1.1, a1_vol[1]*1.1, a1_vol[2]*1.1,
                         a1_vol[3]*1.1, a1_vol[4]*1.1, a1_vol[5]*1.1,
                         a1_vol[6]*1.1, a1_vol[7]*1.1, a1_vol[8]*1.1,
                         a1_vol[9]*1.1};
    double a3_close[] = {a1_close[0]*0.9, a1_close[1]*0.9, a1_close[2]*0.9,
                         a1_close[3]*0.9, a1_close[4]*0.9, a1_close[5]*0.9,
                         a1_close[6]*0.9, a1_close[7]*0.9, a1_close[8]*0.9,
                         a1_close[9]*0.9};
    double a3_vol[]   = {a1_vol[0]*0.9, a1_vol[1]*0.9, a1_vol[2]*0.9,
                         a1_vol[3]*0.9, a1_vol[4]*0.9, a1_vol[5]*0.9,
                         a1_vol[6]*0.9, a1_vol[7]*0.9, a1_vol[8]*0.9,
                         a1_vol[9]*0.9};
    double a4_close[] = {a1_close[0]*1.02, a1_close[1]*1.02, a1_close[2]*1.02,
                         a1_close[3]*1.02, a1_close[4]*1.02, a1_close[5]*1.02,
                         a1_close[6]*1.02, a1_close[7]*1.02, a1_close[8]*1.02,
                         a1_close[9]*1.02};
    double a4_vol[]   = {a1_vol[0]*1.02, a1_vol[1]*1.02, a1_vol[2]*1.02,
                         a1_vol[3]*1.02, a1_vol[4]*1.02, a1_vol[5]*1.02,
                         a1_vol[6]*1.02, a1_vol[7]*1.02, a1_vol[8]*1.02,
                         a1_vol[9]*1.02};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[VWMA_INPUTS] = {a1_close, a1_vol};
    const double *asset2[VWMA_INPUTS] = {a2_close, a2_vol};
    const double *asset3[VWMA_INPUTS] = {a3_close, a3_vol};
    const double *asset4[VWMA_INPUTS] = {a4_close, a4_vol};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = vwma_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        vwma_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in one call:

    ```c
    double o5[] = {5.0}, o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0};
    const double *const simd_opts[4] = {o5, o10, o14, o20};

    CSimdResult r = vwma_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i (periods 5/10/14/20) */
    for (uintptr_t i = 0; i < r.num_results; i++) vwma_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1_close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    a1_vol := []float64{5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                        3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0}
    a2_close := []float64{a1_close[0]*1.1, a1_close[1]*1.1, a1_close[2]*1.1,
                          a1_close[3]*1.1, a1_close[4]*1.1, a1_close[5]*1.1,
                          a1_close[6]*1.1, a1_close[7]*1.1, a1_close[8]*1.1,
                          a1_close[9]*1.1}
    a2_vol := []float64{a1_vol[0]*1.1, a1_vol[1]*1.1, a1_vol[2]*1.1,
                        a1_vol[3]*1.1, a1_vol[4]*1.1, a1_vol[5]*1.1,
                        a1_vol[6]*1.1, a1_vol[7]*1.1, a1_vol[8]*1.1,
                        a1_vol[9]*1.1}
    a3_close := []float64{a1_close[0]*0.9, a1_close[1]*0.9, a1_close[2]*0.9,
                          a1_close[3]*0.9, a1_close[4]*0.9, a1_close[5]*0.9,
                          a1_close[6]*0.9, a1_close[7]*0.9, a1_close[8]*0.9,
                          a1_close[9]*0.9}
    a3_vol := []float64{a1_vol[0]*0.9, a1_vol[1]*0.9, a1_vol[2]*0.9,
                        a1_vol[3]*0.9, a1_vol[4]*0.9, a1_vol[5]*0.9,
                        a1_vol[6]*0.9, a1_vol[7]*0.9, a1_vol[8]*0.9,
                        a1_vol[9]*0.9}
    a4_close := []float64{a1_close[0]*1.02, a1_close[1]*1.02, a1_close[2]*1.02,
                          a1_close[3]*1.02, a1_close[4]*1.02, a1_close[5]*1.02,
                          a1_close[6]*1.02, a1_close[7]*1.02, a1_close[8]*1.02,
                          a1_close[9]*1.02}
    a4_vol := []float64{a1_vol[0]*1.02, a1_vol[1]*1.02, a1_vol[2]*1.02,
                        a1_vol[3]*1.02, a1_vol[4]*1.02, a1_vol[5]*1.02,
                        a1_vol[6]*1.02, a1_vol[7]*1.02, a1_vol[8]*1.02,
                        a1_vol[9]*1.02}

    assets := [][indicators.VwmaInputs][]float64{{a1_close, a1_vol}, {a2_close, a2_vol}, {a3_close, a3_vol}, {a4_close, a4_vol}}
    sim, _ := indicators.Vwma.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    volume := []float64{5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                        3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0}
    sim2, _ := indicators.Vwma.SimdByOptions(close, volume, [][]float64{{5}, {10}, {14}, {20}}, nil)
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

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                       3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0], dtype=np.float64)

    a1_close, a1_vol = close,          volume
    a2_close, a2_vol = close + 5.0,    volume
    a3_close, a3_vol = close - 5.0,    volume
    a4_close, a4_vol = close * 1.02,   volume

    simd_inputs = [
        [a1_close, a1_vol],
        [a2_close, a2_vol],
        [a3_close, a3_vol],
        [a4_close, a4_vol],
    ]
    outputs_list, states = tulip_rs.indicators.vwma.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                       3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.vwma.simd_by_options([close, volume], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice(), volume.slice()],
        [close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.vwma.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.vwma.simdByOptions([close, volume], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
