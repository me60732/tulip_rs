# CMF — Chaikin Money Flow

Measures buying and selling pressure. For each bar: MFV = ((close − low) − (high − close)) / (high − low) × volume. CMF = sum(MFV, period) / sum(volume, period). Values near +1 indicate strong buying pressure; values near −1 indicate strong selling pressure.

**Inputs:** `[high, low, close, volume]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[cmf]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::chaikinmf::{ChaikinMf, Indicator, TIndicatorState};

    let high   = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low    = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1500.0, 1300.0, 1100.0, 1600.0,
                      1400.0, 1200.0, 1700.0, 1800.0, 1500.0_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = ChaikinMf::indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // CMF values

    // State continuation — feed new bars without reprocessing history
    let new_high   = vec![85.20_f64];
    let new_low    = vec![84.50_f64];
    let new_close  = vec![85.00_f64];
    let new_volume = vec![1550.0_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(),
          new_close.as_slice(), new_volume.as_slice()],
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
    double close[]  = {81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36};
    double volume[] = {1200.0, 1500.0, 1300.0, 1100.0, 1600.0,
                       1400.0, 1200.0, 1700.0, 1800.0, 1500.0};
    const double options[CHAIKINMF_OPTIONS] = {14.0}; // period
    const double *inputs[CHAIKINMF_INPUTS] = {high, low, close, volume};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = chaikinmf_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the CMF series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    chaikinmf_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = chaikinmf_indicator(inputs, 8, options, NULL, 0);
    double new_high[]   = {85.20};
    double new_low[]    = {84.50};
    double new_close[]  = {85.00};
    double new_volume[] = {1550.0};
    const double *new_inputs[CHAIKINMF_INPUTS] = {new_high, new_low, new_close, new_volume};
    CBatchResult b = chaikinmf_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> CMF value for the single new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    chaikinmf_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high   := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00}
    low    := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11}
    close  := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    volume := []float64{1200.0, 1500.0, 1300.0, 1100.0, 1600.0,
                        1400.0, 1200.0, 1700.0, 1800.0, 1500.0}
    options := []float64{14.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Chaikinmf.Indicator(high, low, close, volume, options, nil)
    fmt.Println(res.Rows[0]) // CMF values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Chaikinmf.Indicator(high[:8], low[:8], close[:8], volume[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], volume[8:], nil)
    fmt.Println(batch.Rows[0]) // continued CMF values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Chaikinmf;

    double[] high   = {82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low    = {81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11};
    double[] close  = {81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36};
    double[] volume = {1200.0, 1500.0, 1300.0, 1100.0, 1600.0,
                       1400.0, 1200.0, 1700.0, 1800.0, 1500.0};
    double[] options = {14.0}; // period

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Chaikinmf.indicator(new double[][] {high, low, close, volume}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // CMF values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Chaikinmf.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n),
        java.util.Arrays.copyOfRange(volume, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10),
            java.util.Arrays.copyOfRange(close, n, 10),
            java.util.Arrays.copyOfRange(volume, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued CMF
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
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1500.0, 1300.0, 1100.0, 1600.0,
                       1400.0, 1200.0, 1700.0, 1800.0, 1500.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.chaikinmf.indicator([high, low, close, volume], [14.0])
    print(outputs[0])  # CMF values

    # State continuation
    new_high   = np.array([85.20], dtype=np.float64)
    new_low    = np.array([84.50], dtype=np.float64)
    new_close  = np.array([85.00], dtype=np.float64)
    new_volume = np.array([1550.0], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close, new_volume])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low    = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close  = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);
    const volume = Float64Array.from([1200.0, 1500.0, 1300.0, 1100.0, 1600.0, 1400.0, 1200.0, 1700.0, 1800.0, 1500.0, 1350.0, 1650.0, 1450.0, 1750.0, 1600.0]);

    const [outputs, state] = ti.chaikinmf.indicator([high, low, close, volume], [14]);
    console.log('CMF(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.chaikinmf.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued CMF:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [1200.0, 1500.0, 1300.0, 1100.0, 1600.0, 1400.0, 1200.0, 1700.0, 1800.0, 1500.0, 1350.0, 1650.0, 1450.0, 1750.0, 1600.0];

    const [outputs, state] = ti.chaikinmf.indicator([high, low, close, volume], [14]);
    console.log('CMF(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.chaikinmf.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued CMF:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::chaikinmf::{ChaikinMf, Indicator};

    let inputs: [&[&[f64]; 4]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice(), v4.as_slice()],
    ];
    let results = ChaikinMf::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::chaikinmf::{ChaikinMf, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = ChaikinMf::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same option applied to 4 assets in one call:

    ```c
    double a1_high[]   = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double a1_low[]    = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double a1_close[]  = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a1_volume[] = {1200.0, 1500.0, 1300.0, 1100.0, 1600.0, 1400.0, 1200.0, 1700.0, 1800.0, 1500.0};

    double a2_high[]   = {98.58, 96.47, 99.63, 99.96, 100.62, 100.68, 99.99, 101.16, 102.31, 103.50};
    double a2_low[]    = {97.95, 95.81, 98.94, 99.27, 100.01, 100.06, 99.33, 100.50, 101.65, 102.80};
    double a2_close[]  = {98.24, 96.15, 99.30, 99.63, 100.30, 100.36, 99.67, 100.83, 102.00, 103.20};
    double a2_volume[] = {1440.0, 1800.0, 1560.0, 1320.0, 1920.0, 1680.0, 1440.0, 2040.0, 2160.0, 1800.0};

    double a3_high[]   = {75.00, 74.50, 76.00, 76.30, 76.85, 76.90, 76.33, 77.30, 77.84, 78.00};
    double a3_low[]    = {74.29, 73.64, 75.31, 75.65, 76.07, 76.11, 75.49, 75.30, 77.15, 77.11};
    double a3_close[]  = {74.59, 74.06, 76.87, 76.00, 76.61, 76.15, 75.84, 76.99, 77.55, 77.36};
    double a3_volume[] = {600.0, 750.0, 650.0, 550.0, 800.0, 700.0, 600.0, 850.0, 900.0, 750.0};

    double a4_high[]   = {102.00, 101.25, 103.50, 103.80, 104.30, 104.35, 103.75, 104.75, 105.25, 105.40};
    double a4_low[]    = {100.65, 99.80, 102.00, 102.20, 103.00, 103.05, 102.40, 103.30, 104.10, 104.25};
    double a4_close[]  = {101.30, 100.60, 103.00, 103.20, 103.75, 103.70, 103.10, 104.20, 104.65, 104.80};
    double a4_volume[] = {1728.0, 2160.0, 1872.0, 1584.0, 2304.0, 2016.0, 1728.0, 2448.0, 2592.0, 2160.0};

    const double *asset1[CHAIKINMF_INPUTS] = {a1_high, a1_low, a1_close, a1_volume};
    const double *asset2[CHAIKINMF_INPUTS] = {a2_high, a2_low, a2_close, a2_volume};
    const double *asset3[CHAIKINMF_INPUTS] = {a3_high, a3_low, a3_close, a3_volume};
    const double *asset4[CHAIKINMF_INPUTS] = {a4_high, a4_low, a4_close, a4_volume};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = chaikinmf_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        chaikinmf_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, N different periods in one call:

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[]   = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]    = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double close[]  = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double volume[] = {1200.0, 1500.0, 1300.0, 1100.0, 1600.0, 1400.0, 1200.0, 1700.0, 1800.0, 1500.0};
    const double *inputs[CHAIKINMF_INPUTS] = {high, low, close, volume};

    /* Tile the series 20x so longer-period option sets have enough data */
    #define EXPANDED_LEN (10 * 20)
    static double high_expanded[EXPANDED_LEN];
    static double low_expanded[EXPANDED_LEN];
    static double close_expanded[EXPANDED_LEN];
    static double volume_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 20; i++) {
        for (size_t j = 0; j < 10; j++) {
            high_expanded[i * 10 + j]   = high[j];
            low_expanded[i * 10 + j]    = low[j];
            close_expanded[i * 10 + j]  = close[j];
            volume_expanded[i * 10 + j] = volume[j];
        }
    }
    const double *expanded_inputs[CHAIKINMF_INPUTS] = {high_expanded, low_expanded, close_expanded, volume_expanded};

    static const double o7[CHAIKINMF_OPTIONS]  = {7.0};
    static const double o14[CHAIKINMF_OPTIONS] = {14.0};
    static const double o21[CHAIKINMF_OPTIONS] = {21.0};
    static const double o28[CHAIKINMF_OPTIONS] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = chaikinmf_simd_by_options(expanded_inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) chaikinmf_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same option applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    v1 := []float64{1200.0, 1500.0, 1300.0, 1100.0, 1600.0, 1400.0, 1200.0, 1700.0, 1800.0, 1500.0}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, c2, v2 := h1, l1, c1, v1
    h3, l3, c3, v3 := h1, l1, c1, v1
    h4, l4, c4, v4 := h1, l1, c1, v1

    assets := [][indicators.ChaikinmfInputs][]float64{{h1, l1, c1, v1}, {h2, l2, c2, v2}, {h3, l3, c3, v3}, {h4, l4, c4, v4}}
    sim, _ := indicators.Chaikinmf.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    sim2, _ := indicators.Chaikinmf.SimdByOptions(high, low, close, volume,
        [][]float64{{7.0}, {14.0}, {21.0}, {28.0}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Chaikinmf;

    double[] a1_high   = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] a1_low    = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] a1_close  = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double[] a1_volume = {1200.0, 1500.0, 1300.0, 1100.0, 1600.0, 1400.0, 1200.0, 1700.0, 1800.0, 1500.0};

    double[] a2_high   = {98.58, 96.47, 99.63, 99.96, 100.62, 100.68, 99.99, 101.16, 102.31, 103.50};
    double[] a2_low    = {97.95, 95.81, 98.94, 99.27, 100.01, 100.06, 99.33, 100.50, 101.65, 102.80};
    double[] a2_close  = {98.24, 96.15, 99.30, 99.63, 100.30, 100.36, 99.67, 100.83, 102.00, 103.20};
    double[] a2_volume = {1440.0, 1800.0, 1560.0, 1320.0, 1920.0, 1680.0, 1440.0, 2040.0, 2160.0, 1800.0};

    double[] a3_high   = {75.00, 74.50, 76.00, 76.30, 76.85, 76.90, 76.33, 77.30, 77.84, 78.00};
    double[] a3_low    = {74.29, 73.64, 75.31, 75.65, 76.07, 76.11, 75.49, 75.30, 77.15, 77.11};
    double[] a3_close  = {74.59, 74.06, 76.87, 76.00, 76.61, 76.15, 75.84, 76.99, 77.55, 77.36};
    double[] a3_volume = {600.0, 750.0, 650.0, 550.0, 800.0, 700.0, 600.0, 850.0, 900.0, 750.0};

    double[] a4_high   = {102.00, 101.25, 103.50, 103.80, 104.30, 104.35, 103.75, 104.75, 105.25, 105.40};
    double[] a4_low    = {100.65, 99.80, 102.00, 102.20, 103.00, 103.05, 102.40, 103.30, 104.10, 104.25};
    double[] a4_close  = {101.30, 100.60, 103.00, 103.20, 103.75, 103.70, 103.10, 104.20, 104.65, 104.80};
    double[] a4_volume = {1728.0, 2160.0, 1872.0, 1584.0, 2304.0, 2016.0, 1728.0, 2448.0, 2592.0, 2160.0};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1_high, a1_low, a1_close, a1_volume}, {a2_high, a2_low, a2_close, a2_volume}, {a3_high, a3_low, a3_close, a3_volume}, {a4_high, a4_low, a4_close, a4_volume}};
    try (SimdResult sim = Chaikinmf.simdByAssets(assets, new double[] {14.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, N different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Chaikinmf;

    double[] high   = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low    = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] close  = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double[] volume = {1200.0, 1500.0, 1300.0, 1100.0, 1600.0, 1400.0, 1200.0, 1700.0, 1800.0, 1500.0};

    try (SimdResult sim = Chaikinmf.simdByOptions(new double[][] {high, low, close, volume},
            new double[][] {{7.0}, {14.0}, {21.0}, {28.0}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```


=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1, v1],
        [h2, l2, c2, v2],
        [h3, l3, c3, v3],
        [h4, l4, c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.chaikinmf.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.chaikinmf.simd_by_options(
        [high, low, close, volume], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice(), volume.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.chaikinmf.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.chaikinmf.simdByOptions([high, low, close, volume], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
