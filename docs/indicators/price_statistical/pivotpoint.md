# Pivot Point — `pivotpoint`

Classic floor-trader pivot points calculated from the previous bar's high, low, and close. Provides a central pivot level plus two support and two resistance levels.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[s3, s2, s1, pivot, r1, r2, r3]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::pivotpoint::{PivotPoint, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    // options: [period]
    let (outputs, mut state) = PivotPoint::indicator(&inputs, &[5.0], None).unwrap();

    println!("Pivot: {:?}", outputs[0]);
    println!("R1:    {:?}", outputs[1]);
    println!("S1:    {:?}", outputs[2]);
    println!("R2:    {:?}", outputs[3]);
    println!("S2:    {:?}", outputs[4]);

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = PivotPoint::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[5.0], None).unwrap();

    println!("Pivot: {:?}", outputs2[0]);
    println!("R1:    {:?}", outputs2[1]);
    println!("S1:    {:?}", outputs2[2]);

    let new_high   = vec![85.90_f64];
    let new_low    = vec![84.03_f64];
    let new_close  = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()], None).unwrap();
    println!("Continued Pivot: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    double options[PIVOTPOINT_OPTIONS] = {5.0}; // period
    const double *inputs[PIVOTPOINT_INPUTS] = {high, low, close};

    /* Full computation - one output row with 7 values: s3,s2,s1,pp,r1,r2,r3 */
    CIndicatorResult r = pivotpoint_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0][i] -> the i-th value in the single output row */
    tulip_ffi_result_free(r);
    pivotpoint_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = pivotpoint_indicator(inputs, 8, options, NULL, 0);
    double new_high[]   = {85.90};
    double new_low[]    = {84.03};
    double new_close[]  = {85.53};
    const double *new_inputs[PIVOTPOINT_INPUTS] = {new_high, new_low, new_close};
    CBatchResult b = pivotpoint_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0][i] -> continued values for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    pivotpoint_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{5.0}

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Pivotpoint.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // s3,s2,s1,pp,r1,r2,r3 values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Pivotpoint.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued pivot values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    // options: [period]
    outputs, state = tulip_rs.indicators.pivotpoint.indicator([high, low, close], [5.0])

    print(f"Pivot: {outputs[0]}")
    print(f"R1:    {outputs[1]}")
    print(f"S1:    {outputs[2]}")
    print(f"R2:    {outputs[3]}")
    print(f"S2:    {outputs[4]}")
    ```

### SIMD

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1_high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    a1_low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    a1_close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    a2_high := []float64{82.15 * 1.2, 81.89 * 1.2, 83.03 * 1.2, 83.30 * 1.2, 83.85 * 1.2,
                         83.90 * 1.2, 83.33 * 1.2, 84.30 * 1.2, 84.84 * 1.2, 85.00 * 1.2}
    a2_low := []float64{81.29 * 1.2, 80.64 * 1.2, 81.31 * 1.2, 82.65 * 1.2, 83.07 * 1.2,
                        83.11 * 1.2, 82.49 * 1.2, 82.30 * 1.2, 84.15 * 1.2, 84.11 * 1.2}
    a2_close := []float64{81.59 * 1.2, 81.06 * 1.2, 82.87 * 1.2, 83.00 * 1.2, 83.61 * 1.2,
                          83.15 * 1.2, 82.84 * 1.2, 83.99 * 1.2, 84.55 * 1.2, 84.36 * 1.2}

    a3_high := []float64{90.0 + 0*0.5 + 82.15*0.1, 90.0 + 1*0.5 + 81.89*0.1,
                         90.0 + 2*0.5 + 83.03*0.1, 90.0 + 3*0.5 + 83.30*0.1, 90.0 + 4*0.5 + 83.85*0.1,
                         90.0 + 5*0.5 + 83.90*0.1, 90.0 + 6*0.5 + 83.33*0.1, 90.0 + 7*0.5 + 84.30*0.1,
                         90.0 + 8*0.5 + 84.84*0.1, 90.0 + 9*0.5 + 85.00*0.1}
    a3_low := []float64{90.0 + 0*0.5 + 81.29*0.1, 90.0 + 1*0.5 + 80.64*0.1,
                        90.0 + 2*0.5 + 81.31*0.1, 90.0 + 3*0.5 + 82.65*0.1, 90.0 + 4*0.5 + 83.07*0.1,
                        90.0 + 5*0.5 + 83.11*0.1, 90.0 + 6*0.5 + 82.49*0.1, 90.0 + 7*0.5 + 82.30*0.1,
                        90.0 + 8*0.5 + 84.15*0.1, 90.0 + 9*0.5 + 84.11*0.1}
    a3_close := []float64{90.0 + 0*0.5 + 81.59*0.1, 90.0 + 1*0.5 + 81.06*0.1,
                          90.0 + 2*0.5 + 82.87*0.1, 90.0 + 3*0.5 + 83.00*0.1, 90.0 + 4*0.5 + 83.61*0.1,
                          90.0 + 5*0.5 + 83.15*0.1, 90.0 + 6*0.5 + 82.84*0.1, 90.0 + 7*0.5 + 83.99*0.1,
                          90.0 + 8*0.5 + 84.55*0.1, 90.0 + 9*0.5 + 84.36*0.1}

    a4_high := []float64{100.0 - 0*0.3 + 82.15*0.05, 100.0 - 1*0.3 + 81.89*0.05,
                         100.0 - 2*0.3 + 83.03*0.05, 100.0 - 3*0.3 + 83.30*0.05, 100.0 - 4*0.3 + 83.85*0.05,
                         100.0 - 5*0.3 + 83.90*0.05, 100.0 - 6*0.3 + 83.33*0.05, 100.0 - 7*0.3 + 84.30*0.05,
                         100.0 - 8*0.3 + 84.84*0.05, 100.0 - 9*0.3 + 85.00*0.05}
    a4_low := []float64{100.0 - 0*0.3 + 81.29*0.05, 100.0 - 1*0.3 + 80.64*0.05,
                        100.0 - 2*0.3 + 81.31*0.05, 100.0 - 3*0.3 + 82.65*0.05, 100.0 - 4*0.3 + 83.07*0.05,
                        100.0 - 5*0.3 + 83.11*0.05, 100.0 - 6*0.3 + 82.49*0.05, 100.0 - 7*0.3 + 82.30*0.05,
                        100.0 - 8*0.3 + 84.15*0.05, 100.0 - 9*0.3 + 84.11*0.05}
    a4_close := []float64{100.0 - 0*0.3 + 81.59*0.05, 100.0 - 1*0.3 + 81.06*0.05,
                          100.0 - 2*0.3 + 82.87*0.05, 100.0 - 3*0.3 + 83.00*0.05, 100.0 - 4*0.3 + 83.61*0.05,
                          100.0 - 5*0.3 + 83.15*0.05, 100.0 - 6*0.3 + 82.84*0.05, 100.0 - 7*0.3 + 83.99*0.05,
                          100.0 - 8*0.3 + 84.55*0.05, 100.0 - 9*0.3 + 84.36*0.05}

    assets := [][indicators.PivotpointInputs][]float64{{a1_high, a1_low, a1_close}, {a2_high, a2_low, a2_close},
                                                        {a3_high, a3_low, a3_close}, {a4_high, a4_low, a4_close}}
    sim, _ := indicators.Pivotpoint.SimdByAssets(assets, []float64{5.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    sim2, _ := indicators.Pivotpoint.SimdByOptions(high, low, close, [][]float64{{5}, {10}, {14}, {20}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.pivotpoint.indicator([high, low, close], []);
    console.log('Pivot:', outputs[0]);
    console.log('R1:',   outputs[1]);
    console.log('S1:',   outputs[2]);
    console.log('R2:',   outputs[3]);
    console.log('S2:',   outputs[4]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.pivotpoint.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Pivot:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.pivotpoint.indicator([high, low, close], []);
    console.log('Pivot:', outputs[0]);
    console.log('R1:',   outputs[1]);
    console.log('S1:',   outputs[2]);
    console.log('R2:',   outputs[3]);
    console.log('S2:',   outputs[4]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.pivotpoint.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Pivot:', continued[0]);
    ```
