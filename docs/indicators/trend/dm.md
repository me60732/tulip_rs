# DM — Directional Movement

Raw directional movement values before smoothing. +DM captures upward movement; -DM captures downward movement.

**Inputs:** `[high, low]` | **Options:** `[period]` | **Outputs:** `[plus_dm, minus_dm]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::dm::{Dm, Indicator, TIndicatorState};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = Dm::indicator(&inputs, &[14.0], None).unwrap();
    println!("+DM: {:?}", outputs[0]);
    println!('-DM: {:?}', outputs[1]);

    // State continuation — feed new bars without reprocessing history
    let partial_high = high[..8].to_vec();
    let partial_low  = low[..8].to_vec();
    let (outputs2, mut state) = Dm::indicator(&[partial_high.as_slice(), partial_low.as_slice()], &[14.0], None).unwrap();
    println!("+DM: {:?}", outputs2[0]);
    println!('-DM: {:?}', outputs2[1]);

    let new_high = vec![85.90_f64];
    let new_low  = vec![84.03_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice()],
        None,
    ).unwrap();
    println!("+DM continued: {:?}", continued[0]);
    println!('-DM continued: {:?}', continued[1]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    double options[DM_OPTIONS] = {14.0}; // period
    const double *inputs[DM_INPUTS] = {high, low};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = dm_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> plus_dm, length r.output_lens[0] */
    /* r.outputs[1] -> minus_dm, length r.output_lens[1] */
    tulip_ffi_result_free(r);
    dm_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = dm_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.90};
    double new_low[]  = {84.03};
    const double *new_inputs[DM_INPUTS] = {new_high, new_low};
    CBatchResult b = dm_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> plus_dm for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    dm_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    options := []float64{14.0} // period

    // Full computation — Rows is [+dm, -dm], valid until Close.
    res, st, _ := indicators.Dm.Indicator(high, low, options, nil)
    fmt.Println("+DM:", res.Rows[0])
    fmt.Println("-DM:", res.Rows[1])
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Dm.Indicator(high[:8], low[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], nil)
    fmt.Println("+DM continued:", batch.Rows[0])
    fmt.Println("-DM continued:", batch.Rows[1])
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    outputs, state = tulip_rs.indicators.dm.indicator([high, low], [14.0])
    print(outputs[0])  # Plus DM
    print(outputs[1])  # Minus DM

    # State continuation
    new_high = np.array([85.30], dtype=np.float64)
    new_low  = np.array([84.60], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low])
    print(continued[0])  # Plus DM continued
    print(continued[1])  # Minus DM continued
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low  = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);

    const [outputs, state] = ti.dm.indicator([high, low], [14]);
    console.log('+DM:', outputs[0]);
    console.log('-DM:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.dm.indicator([high.slice(0, n), low.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued +DM:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.dm.indicator([high, low], [14]);
    console.log('+DM:', outputs[0]);
    console.log('-DM:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.dm.indicator([high.slice(0, n), low.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued +DM:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::dm::{Dm, Indicator};

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = Dm::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} +DM: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} -DM: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::dm::{Dm, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = Dm::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {} +DM: {:?}", opts[i][0], out[0]);
        println!("Period {} -DM: {:?}", opts[i][0], out[1]);
    }
    ```

=== "C"

    **By assets** — same option applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double h2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double l2[] = {71.10, 71.85, 72.40, 72.00, 73.20, 73.85, 74.10, 74.60, 75.00, 75.50};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[DM_INPUTS] = {h1, l1};
    const double *asset2[DM_INPUTS] = {h2, l2};
    const double *const *const simd_inputs[4] = {asset1, asset2, NULL, NULL}; /* N=4 lanes */
    double options[DM_OPTIONS] = {14.0};

    CSimdResult r = dm_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's plus_dm, length r.output_lens[i][0] */
        /* r.outputs[i][1] -> asset i's minus_dm, length r.output_lens[i][1] */
        dm_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, N different periods in one call:

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    const double *inputs[DM_INPUTS] = {high, low};

    /* Tile the series 20x so longer-period option sets have enough data */
    #define EXPANDED_LEN (10 * 20)
    static double high_expanded[EXPANDED_LEN];
    static double low_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 20; i++) {
        for (size_t j = 0; j < 10; j++) {
            high_expanded[i * 10 + j] = high[j];
            low_expanded[i * 10 + j]  = low[j];
        }
    }
    const double *expanded_inputs[DM_INPUTS] = {high_expanded, low_expanded};

    static const double o7[DM_OPTIONS]   = {7.0};
    static const double o14[DM_OPTIONS]  = {14.0};
    static const double o21[DM_OPTIONS]  = {21.0};
    static const double o28[DM_OPTIONS]  = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = dm_simd_by_options(expanded_inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) dm_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    h2 := []float64{72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50}
    l2 := []float64{71.10, 71.85, 72.40, 72.00, 73.20, 73.85, 74.10, 74.60, 75.00, 75.50}

    assets := [][indicators.DmInputs][]float64{{h1, l1}, {h2, l2}}
    sim, _ := indicators.Dm.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d +DM: %v\n", i+1, lanes[0])
        fmt.Printf("Asset %d -DM: %v\n", i+1, lanes[1])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}

    // Tile the series so longer-period option sets have enough data
    expandedHigh := make([]float64, len(high)*20)
    expandedLow := make([]float64, len(low)*20)
    for i := 0; i < 20; i++ {
        copy(expandedHigh[i*len(high):], high)
        copy(expandedLow[i*len(low):], low)
    }

    sim2, _ := indicators.Dm.SimdByOptions(expandedHigh, expandedLow, [][]float64{{7.0}, {14.0}, {21.0}, {28.0}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period %d +DM: %v\n", i+1, lanes[0])
        fmt.Printf("Period %d -DM: %v\n", i+1, lanes[1])
    }
    sim2.Close()
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1],
        [h2, l2],
        [h3, l3],
        [h4, l4],
    ]
    outputs_list, states = tulip_rs.indicators.dm.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1} +DM: {asset_outputs[0]}")
        print(f"Asset {i+1} -DM: {asset_outputs[1]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.dm.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]} +DM: {out[0]}")
        print(f"Period {simd_options[i][0]} -DM: {out[1]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.dm.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} +DM:`, out[0], '-DM:', out[1]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.dm.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]} +DM:`, out[0], '-DM:', out[1]));
    ```
