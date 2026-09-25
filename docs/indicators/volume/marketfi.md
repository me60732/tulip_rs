# Market Facilitation Index — `marketfi`

`(High - Low) / Volume` — measures the efficiency of price movement per unit of volume traded.

**Inputs:** `[high, low, volume]` | **Options:** none | **Outputs:** `[marketfi]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::marketfi::{Marketfi};

    let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
    let (outputs, _) = Marketfi::indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    const double high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                           85.90, 86.58, 86.98, 88.00, 87.87};
    const double low[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                          84.03, 85.39, 85.76, 87.17, 87.01};
    const double volume[] = {5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0,
                             3936200.0, 4732000.0, 4841300.0, 3915300.0, 6830800.0, 6694100.0,
                             5293600.0, 7985800.0, 4807900.0};

    const double *inputs[MARKETFI_INPUTS] = {high, low, volume};
    const double *options = NULL; // MARKETFI has no options

    /* Full computation */
    CIndicatorResult r = marketfi_indicator(inputs, 15, options, NULL, 0);
    /* r.outputs[0] -> the MARKETFI series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    marketfi_state_free(r.state);

    /* Partial computation + batch continuation */
    CIndicatorResult pr = marketfi_indicator(inputs, 10, options, NULL, 0);
    const double *rest_inputs[MARKETFI_INPUTS] = {high + 10, low + 10, volume + 10};
    CBatchResult br = marketfi_batch(pr.state, rest_inputs, 5, NULL, 0);
    /* br.outputs[0] -> MARKETFI values for the last 5 bars */
    tulip_ffi_batch_result_free(br);
    tulip_ffi_result_free(pr);
    marketfi_state_free(pr.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high   := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                       85.90, 86.58, 86.98, 88.00, 87.87}
    low    := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01}
    volume := []float64{5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0,
                        3936200.0, 4732000.0, 4841300.0, 3915300.0, 6830800.0, 6694100.0,
                        5293600.0, 7985800.0, 4807900.0}
    options := []float64{} // no options (MARKETFI has zero options)

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Marketfi.Indicator(high, low, volume, options, nil)
    fmt.Println(res.Rows[0]) // MarketFi values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Marketfi.Indicator(high[:10], low[:10], volume[:10], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[10:], low[10:], volume[10:], nil)
    fmt.Println(batch.Rows[0]) // continued MarketFi values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Marketfi;

    double[] high   = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                       85.90, 86.58, 86.98, 88.00, 87.87};
    double[] low    = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                       84.03, 85.39, 85.76, 87.17, 87.01};
    double[] volume = {5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0,
                       3936200.0, 4732000.0, 4841300.0, 3915300.0, 6830800.0, 6694100.0,
                       5293600.0, 7985800.0, 4807900.0};
    double[] options = {}; // no options (MARKETFI has zero options)

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Marketfi.indicator(new double[][] {high, low, volume}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // MarketFi values
    }

    // Partial computation + state continuation.
    int n = 10;
    Outcome p = Marketfi.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(volume, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 15),
            java.util.Arrays.copyOfRange(low, n, 15),
            java.util.Arrays.copyOfRange(volume, n, 15)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued MarketFi
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0, 900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.marketfi.indicator([high, low, volume], [])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low    = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const volume = Float64Array.from([5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900]);

    const [outputs, state] = ti.marketfi.indicator([high, low, volume], []);
    console.log('MarketFi:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.marketfi.indicator([high.slice(0, n), low.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), volume.slice(n)]);
    console.log('Continued MarketFi:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.marketfi.indicator([high, low, volume], []);
    console.log('MarketFi:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.marketfi.indicator([high.slice(0, n), low.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), volume.slice(n)]);
    console.log('Continued MarketFi:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::marketfi::{Marketfi, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), v4.as_slice()],
    ];
    let results = Marketfi::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "C"

    **By assets** — same option applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    const double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    const double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    const double v1[] = {5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0,
                         3936200.0, 4732000.0, 4841300.0, 3915300.0};

    const double h2[] = {85.00, 85.50, 86.00, 86.50, 87.00, 87.50, 88.00, 88.50, 89.00, 89.50};
    const double l2[] = {84.00, 84.50, 85.00, 85.50, 86.00, 86.50, 87.00, 87.50, 88.00, 88.50};
    const double v2[] = {4500000.0, 5500000.0, 6500000.0, 4000000.0, 4500000.0, 3800000.0,
                         4200000.0, 5000000.0, 5100000.0, 4100000.0};

    const double h3[] = {78.00, 79.00, 80.00, 81.00, 82.00, 83.00, 84.00, 85.00, 86.00, 87.00};
    const double l3[] = {77.00, 78.00, 79.00, 80.00, 81.00, 82.00, 83.00, 84.00, 85.00, 86.00};
    const double v3[] = {3500000.0, 4500000.0, 5500000.0, 3000000.0, 3500000.0, 2800000.0,
                         3200000.0, 4000000.0, 4100000.0, 3100000.0};

    const double h4[] = {95.00, 96.00, 97.00, 98.00, 99.00, 100.00, 101.00, 102.00, 103.00, 104.00};
    const double l4[] = {94.00, 95.00, 96.00, 97.00, 98.00, 99.00, 100.00, 101.00, 102.00, 103.00};
    const double v4[] = {6500000.0, 7500000.0, 8500000.0, 6000000.0, 6500000.0, 5800000.0,
                         6200000.0, 7000000.0, 7100000.0, 6100000.0};

    const double *const asset1[MARKETFI_INPUTS] = {h1, l1, v1};
    const double *const asset2[MARKETFI_INPUTS] = {h2, l2, v2};
    const double *const asset3[MARKETFI_INPUTS] = {h3, l3, v3};
    const double *const asset4[MARKETFI_INPUTS] = {h4, l4, v4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = marketfi_simd_by_assets(simd_inputs, 4, 10, NULL, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        marketfi_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    assets := [][indicators.MarketfiInputs][]float64{
        {a1_high, a1_low, a1_volume},
        {a2_high, a2_low, a2_volume},
        {a3_high, a3_low, a3_volume},
        {a4_high, a4_low, a4_volume},
    }
    sim, _ := indicators.Marketfi.SimdByAssets(assets, []float64{}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    _This indicator has no options, so by-options SIMD does not offer._

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Marketfi;

    double[] a1_high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] a1_low  = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] a1_volume = {5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0,
                          3936200.0, 4732000.0, 4841300.0, 3915300.0};

    double[] a2_high = {85.00, 85.50, 86.00, 86.50, 87.00, 87.50, 88.00, 88.50, 89.00, 89.50};
    double[] a2_low  = {84.00, 84.50, 85.00, 85.50, 86.00, 86.50, 87.00, 87.50, 88.00, 88.50};
    double[] a2_volume = {4500000.0, 5500000.0, 6500000.0, 4000000.0, 4500000.0, 3800000.0,
                          4200000.0, 5000000.0, 5100000.0, 4100000.0};

    double[] a3_high = {78.00, 79.00, 80.00, 81.00, 82.00, 83.00, 84.00, 85.00, 86.00, 87.00};
    double[] a3_low  = {77.00, 78.00, 79.00, 80.00, 81.00, 82.00, 83.00, 84.00, 85.00, 86.00};
    double[] a3_volume = {3500000.0, 4500000.0, 5500000.0, 3000000.0, 3500000.0, 2800000.0,
                          3200000.0, 4000000.0, 4100000.0, 3100000.0};

    double[] a4_high = {95.00, 96.00, 97.00, 98.00, 99.00, 100.00, 101.00, 102.00, 103.00, 104.00};
    double[] a4_low  = {94.00, 95.00, 96.00, 97.00, 98.00, 99.00, 100.00, 101.00, 102.00, 103.00};
    double[] a4_volume = {6500000.0, 7500000.0, 8500000.0, 6000000.0, 6500000.0, 5800000.0,
                          6200000.0, 7000000.0, 7100000.0, 6100000.0};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1_high, a1_low, a1_volume}, {a2_high, a2_low, a2_volume}, {a3_high, a3_low, a3_volume}, {a4_high, a4_low, a4_volume}};
    try (SimdResult sim = Marketfi.simdByAssets(assets, new double[] {})) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1, v1], [h2, l2, v2], [h3, l3, v3], [h4, l4, v4]]
    outputs_list, states = tulip_rs.indicators.marketfi.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), volume.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.marketfi.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
