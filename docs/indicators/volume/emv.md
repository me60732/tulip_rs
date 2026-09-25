# EMV — Ease of Movement

Relates price change to volume, indicating how easily a price moves. High values suggest price is moving easily on low volume.

**Inputs:** `[high, low, volume]` | **Options:** `[]` | **Outputs:** `[emv]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::emv::{Emv, Indicator, TIndicatorState};

    let high   = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low    = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
    let (outputs, mut state) = Emv::indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // EMV values

    // State continuation — feed new bars without reprocessing history
    let new_high   = vec![85.20_f64];
    let new_low    = vec![84.50_f64];
    let new_volume = vec![1550.0_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]   = {82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]    = {81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11};
    double volume[] = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0};

    const double *inputs[EMV_INPUTS] = {high, low, volume};
    const double options[EMV_OPTIONS] = {}; // no options

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = emv_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the EMV series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    emv_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = emv_indicator(inputs, 8, options, NULL, 0);
    double new_high[]   = {85.20};
    double new_low[]    = {84.50};
    double new_volume[] = {1550.0};
    const double *new_inputs[EMV_INPUTS] = {new_high, new_low, new_volume};
    CBatchResult b = emv_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> EMV value for the single new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    emv_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    // emv takes high, low, volume — and no options.
    res, st, _ := indicators.Emv.Indicator(high, low, volume, nil, nil)
    fmt.Println(res.Rows[0]) // EMV values
    res.Close()
    st.Close()

    // State continuation — feed new bars without reprocessing history.
    res2, st2, _ := indicators.Emv.Indicator(high[:8], low[:8], volume[:8], nil, nil)
    res2.Close()
    batch, _ := st2.Batch(newHigh, newLow, newVolume, nil)
    fmt.Println(batch.Rows[0]) // EMV for the new bar
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Emv;

    double[] high   = {82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low    = {81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11};
    double[] volume = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0};
    double[] options = {}; // no options

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Emv.indicator(new double[][] {high, low, volume}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // EMV values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Emv.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(volume, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10),
            java.util.Arrays.copyOfRange(volume, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued EMV
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.emv.indicator([high, low, volume], [])
    print(outputs[0])  # EMV values

    # State continuation
    new_high   = np.array([85.20], dtype=np.float64)
    new_low    = np.array([84.50], dtype=np.float64)
    new_volume = np.array([1550.0], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_volume])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low    = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const volume = Float64Array.from([5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900]);

    const [outputs, state] = ti.emv.indicator([high, low, volume], []);
    console.log('EMV:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.emv.indicator([high.slice(0, n), low.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), volume.slice(n)]);
    console.log('Continued EMV:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.emv.indicator([high, low, volume], []);
    console.log('EMV:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.emv.indicator([high.slice(0, n), low.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), volume.slice(n)]);
    console.log('Continued EMV:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `emv` exposes 1 optional output: `medprice`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

=== "C"

    The mask is an array of booleans (one per optional output) passed to `emv_indicator()`:

    ```c
    #include "tulip_rs_ffi.h"

    double close[]  = {81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36};
    double high[]   = {82.59, 82.06, 83.87, 84.00, 84.61,
                       84.15, 83.84, 84.99, 85.55, 85.36};
    double low[]    = {80.59, 80.06, 81.87, 82.00, 82.61,
                       82.15, 81.84, 82.99, 83.55, 83.36};
    double volume[] = {10000.0, 12000.0, 9500.0, 11000.0, 13000.0,
                       9800.0, 10500.0, 12500.0, 11800.0, 10200.0};

    bool optional_outputs[1] = {true}; // medprice

    CIndicatorResult r = emv_indicator(inputs, 10, options, optional_outputs, 1);
    /* r.outputs[0] -> emv (primary) */
    /* r.outputs[1] -> medprice (requested) */
    tulip_ffi_result_free(r);
    emv_state_free(r.state);
    ```

=== "Go"

    ```go
    // emv exposes 1 optional output: medprice — one bool per optional.
    res, st, _ := indicators.Emv.Indicator(high, low, volume, nil, []bool{true})
    defer res.Close()
    defer st.Close()

    emv      := res.Rows[0] // emv (primary)
    medprice := res.Rows[1] // medprice (optional — requested)
    fmt.Println(emv, medprice)
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Emv;

    double[] high   = {82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low    = {81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11};
    double[] volume = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0};
    double[] options = {}; // no options

    boolean[] mask = {true}; // medprice
    Outcome oc = Emv.indicator(new double[][] {high, low, volume}, options, mask);
    try (Result res = oc.result()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // emv (primary)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // medprice (optional — requested)
    }
    oc.state().close();
    ```

    ```rust
    use tulip_rs::indicators::emv::{Emv, Indicator, TIndicatorState};

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high   = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low    = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();
    let volume = vec![10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0_f64];

    let mask = [true];
    let (outputs, _state) = Emv::indicator(
        &[high.as_slice(), low.as_slice(), volume.as_slice()],
        &[],
        Some(&mask),
    ).unwrap();

    let emv      = &outputs[0]; // emv (primary)
    let medprice = &outputs[1]; // medprice (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high   = close + 1.0
    low    = close - 1.0
    volume = np.array([10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.emv.indicator(
        [high, low, volume], [],
        optional_outputs=[True],
    )

    emv      = outputs[0]  # emv (primary)
    medprice = outputs[1]  # medprice (optional — requested)
    ```

=== "Node.js"

    `emv` exposes 1 optional output: `medprice`.

    ```javascript
    const [allOut] = ti.emv.indicator([high, low, volume], [], [true]);
    const emv      = allOut[0]; // primary
    const medprice = allOut[1]; // optional 0: medprice
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.emv.indicator([high, low, volume], [], [true]);
    const emv      = allOut[0]; // primary
    const medprice = allOut[1]; // optional 0: medprice
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::emv::{Emv, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), v4.as_slice()],
    ];
    let results = Emv::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "C"

    **By assets** — same option applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1_high[]   = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double a1_low[]    = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double a1_volume[] = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0, 900.0, 1500.0, 1800.0, 1000.0, 1700.0};

    double a2_high[]   = {98.58, 96.47, 99.63, 99.96, 100.62, 100.68, 99.99, 101.16, 102.31, 103.50};
    double a2_low[]    = {97.95, 95.81, 98.94, 99.27, 100.01, 100.06, 99.33, 100.50, 101.65, 102.80};
    double a2_volume[] = {1440.0, 1800.0, 1560.0, 1320.0, 1920.0, 1680.0, 1440.0, 2040.0, 2160.0, 1800.0};

    double a3_high[]   = {75.00, 74.50, 76.00, 76.30, 76.85, 76.90, 76.33, 77.30, 77.84, 78.00};
    double a3_low[]    = {74.29, 73.64, 75.31, 75.65, 76.07, 76.11, 75.49, 75.30, 77.15, 77.11};
    double a3_volume[] = {600.0, 700.0, 550.0, 800.0, 650.0, 450.0, 750.0, 900.0, 500.0, 850.0};

    double a4_high[]   = {102.00, 101.25, 103.50, 103.80, 104.30, 104.35, 103.75, 104.75, 105.25, 105.40};
    double a4_low[]    = {100.65, 99.80, 102.00, 102.20, 103.00, 103.05, 102.40, 103.30, 104.10, 104.25};
    double a4_volume[] = {1728.0, 2160.0, 1872.0, 1584.0, 2304.0, 2016.0, 1728.0, 2448.0, 2592.0, 2160.0};

    const double *asset1[EMV_INPUTS] = {a1_high, a1_low, a1_volume};
    const double *asset2[EMV_INPUTS] = {a2_high, a2_low, a2_volume};
    const double *asset3[EMV_INPUTS] = {a3_high, a3_low, a3_volume};
    const double *asset4[EMV_INPUTS] = {a4_high, a4_low, a4_volume};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = emv_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        emv_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    v1 := []float64{1200.0, 1400.0, 1100.0, 1600.0, 1300.0, 900.0, 1500.0, 1800.0, 1000.0, 1700.0}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, v2 := h1, l1, v1
    h3, l3, v3 := h1, l1, v1
    h4, l4, v4 := h1, l1, v1

    assets := [][indicators.EmvInputs][]float64{{h1, l1, v1}, {h2, l2, v2}, {h3, l3, v3}, {h4, l4, v4}}
    sim, _ := indicators.Emv.SimdByAssets(assets, nil, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Java"

    **By assets** — applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Emv;

    double[] a1_high = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] a1_low  = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] a1_volume = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0, 900.0, 1500.0, 1800.0, 1000.0, 1700.0};

    double[] a2_high = {98.58, 96.47, 99.63, 99.96, 100.62, 100.68, 99.99, 101.16, 102.31, 103.50};
    double[] a2_low  = {97.95, 95.81, 98.94, 99.27, 100.01, 100.06, 99.33, 100.50, 101.65, 102.80};
    double[] a2_volume = {1440.0, 1800.0, 1560.0, 1320.0, 1920.0, 1680.0, 1440.0, 2040.0, 2160.0, 1800.0};

    double[] a3_high = {75.00, 74.50, 76.00, 76.30, 76.85, 76.90, 76.33, 77.30, 77.84, 78.00};
    double[] a3_low  = {74.29, 73.64, 75.31, 75.65, 76.07, 76.11, 75.49, 75.30, 77.15, 77.11};
    double[] a3_volume = {600.0, 700.0, 550.0, 800.0, 650.0, 450.0, 750.0, 900.0, 500.0, 850.0};

    double[] a4_high = {102.00, 101.25, 103.50, 103.80, 104.30, 104.35, 103.75, 104.75, 105.25, 105.40};
    double[] a4_low  = {100.65, 99.80, 102.00, 102.20, 103.00, 103.05, 102.40, 103.30, 104.10, 104.25};
    double[] a4_volume = {1728.0, 2160.0, 1872.0, 1584.0, 2304.0, 2016.0, 1728.0, 2448.0, 2592.0, 2160.0};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1_high, a1_low, a1_volume}, {a2_high, a2_low, a2_volume}, {a3_high, a3_low, a3_volume}, {a4_high, a4_low, a4_volume}};
    try (SimdResult sim = Emv.simdByAssets(assets, new double[] {}, null)) {
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
    simd_inputs = [
        [h1, l1, v1],
        [h2, l2, v2],
        [h3, l3, v3],
        [h4, l4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.emv.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
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
    const [results] = ti.emv.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
