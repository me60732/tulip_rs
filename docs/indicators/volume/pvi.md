# PVI — Positive Volume Index

Tracks price changes on days when volume increases. Complements NVI.

**Inputs:** `[real, volume]` | **Options:** `[]` | **Outputs:** `[pvi]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::pvi::{Pvi, Indicator, TIndicatorState};

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    let inputs = [close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = Pvi::indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // PVI values

    // State continuation — feed new bars without reprocessing history
    let new_close  = vec![85.50_f64];
    let new_volume = vec![1900.0_f64];
    let continued = state.batch_indicator(
        &[new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    const double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                            85.53, 86.54, 86.89, 87.77, 87.29};
    const double volume[] = {5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0,
                             3936200.0, 4732000.0, 4841300.0, 3915300.0, 6830800.0, 6694100.0,
                             5293600.0, 7985800.0, 4807900.0};

    const double *inputs[PVI_INPUTS] = {close, volume};
    const double options[PVI_OPTIONS] = {}; // PVI has no options (OPTIONS=0)

    /* Full computation */
    CIndicatorResult r = pvi_indicator(inputs, 15, options, NULL, 0);
    /* r.outputs[0] -> the PVI series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    pvi_state_free(r.state);

    /* Partial computation + batch continuation */
    CIndicatorResult pr = pvi_indicator(inputs, 10, options, NULL, 0);
    const double *rest_inputs[PVI_INPUTS] = {close + 10, volume + 10};
    CBatchResult br = pvi_batch(pr.state, rest_inputs, 5, NULL, 0);
    /* br.outputs[0] -> PVI values for the last 5 bars */
    tulip_ffi_batch_result_free(br);
    tulip_ffi_result_free(pr);
    pvi_state_free(pr.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close  := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                        83.15, 82.84, 83.99, 84.55, 84.36}
    volume := []float64{5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                        3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0}
    options := []float64{} // no options

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Pvi.Indicator(close, volume, options, nil)
    fmt.Println(res.Rows[0]) // PVI values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Pvi.Indicator(close[:10], volume[:10], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[10:], volume[10:], nil)
    fmt.Println(batch.Rows[0]) // continued PVI values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.pvi.indicator([close, volume], [])
    print(outputs[0])  # PVI values

    # State continuation
    new_close  = np.array([85.50], dtype=np.float64)
    new_volume = np.array([1900.0], dtype=np.float64)
    continued = state.batch_indicator([new_close, new_volume])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close  = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);
    const volume = Float64Array.from([5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900]);

    const [outputs, state] = ti.pvi.indicator([close, volume], []);
    console.log('PVI:', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.pvi.indicator([close.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([close.slice(n), volume.slice(n)]);
    console.log('Continued PVI:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.pvi.indicator([close, volume], []);
    console.log('PVI:', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.pvi.indicator([close.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([close.slice(n), volume.slice(n)]);
    console.log('Continued PVI:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::pvi::{Pvi, Indicator};

    let inputs: [&[&[f64]; 2]; 4] = [
        &[c1.as_slice(), v1.as_slice()],
        &[c2.as_slice(), v2.as_slice()],
        &[c3.as_slice(), v3.as_slice()],
        &[c4.as_slice(), v4.as_slice()],
    ];
    let results = Pvi::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "C"

    **By assets** — same option applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    const double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    const double v1[] = {5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0,
                         3936200.0, 4732000.0, 4841300.0, 3915300.0};

    const double c2[] = {85.00, 85.50, 86.00, 86.50, 87.00, 87.50, 88.00, 88.50, 89.00, 89.50};
    const double v2[] = {4500000.0, 5500000.0, 6500000.0, 4000000.0, 4500000.0, 3800000.0,
                         4200000.0, 5000000.0, 5100000.0, 4100000.0};

    const double c3[] = {78.00, 79.00, 80.00, 81.00, 82.00, 83.00, 84.00, 85.00, 86.00, 87.00};
    const double v3[] = {3500000.0, 4500000.0, 5500000.0, 3000000.0, 3500000.0, 2800000.0,
                         3200000.0, 4000000.0, 4100000.0, 3100000.0};

    const double c4[] = {95.00, 96.00, 97.00, 98.00, 99.00, 100.00, 101.00, 102.00, 103.00, 104.00};
    const double v4[] = {6500000.0, 7500000.0, 8500000.0, 6000000.0, 6500000.0, 5800000.0,
                         6200000.0, 7000000.0, 7100000.0, 6100000.0};

    const double *const asset1[PVI_INPUTS] = {c1, v1};
    const double *const asset2[PVI_INPUTS] = {c2, v2};
    const double *const asset3[PVI_INPUTS] = {c3, v3};
    const double *const asset4[PVI_INPUTS] = {c4, v4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = pvi_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's PVI series, length r.output_lens[i][0] */
        pvi_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    _C FFI offers only by-assets for this indicator (no options)._

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    v1 := []float64{5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0,
                    3936200.0, 4732000.0, 4841300.0, 3915300.0}

    // Reuse the same data for assets 2–4 in this example
    c2, v2 := c1, v1
    c3, v3 := c1, v1
    c4, v4 := c1, v1

    assets := [][indicators.PviInputs][]float64{
        {c1, v1},
        {c2, v2},
        {c3, v3},
        {c4, v4},
    }
    sim, _ := indicators.Pvi.SimdByAssets(assets, []float64{}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    _This indicator has no options, so by-options SIMD does not offer._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [c1, v1],
        [c2, v2],
        [c3, v3],
        [c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.pvi.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice(), volume.slice()],
        [close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.pvi.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
