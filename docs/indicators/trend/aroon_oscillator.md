# Aroon Oscillator

The difference between Aroon Up and Aroon Down. Positive values indicate bullish momentum; negative bearish.

**Inputs:** `[high, low]` | **Options:** `[period]` | **Outputs:** `[aroonosc]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::aroonosc::{AroonOsc, Indicator, TIndicatorState};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = AroonOsc::indicator(&inputs, &[25.0], None).unwrap();
    println!("{:?}", outputs[0]); // Aroon Oscillator values

    // State continuation — feed new bars without reprocessing history
    let partial_high = high[..8].to_vec();
    let partial_low  = low[..8].to_vec();
    let (outputs2, mut state) = AroonOsc::indicator(&[partial_high.as_slice(), partial_low.as_slice()], &[25.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high = vec![85.90_f64];
    let new_low  = vec![84.03_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    double options[AROONOSC_OPTIONS] = {25.0}; // period
    const double *inputs[AROONOSC_INPUTS] = {high, low};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = aroonosc_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> aroonosc, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    aroonosc_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = aroonosc_indicator(inputs, 8, options, NULL, 0);
    double new_high[] = {85.90};
    double new_low[]  = {84.03};
    const double *new_inputs[AROONOSC_INPUTS] = {new_high, new_low};
    CBatchResult b = aroonosc_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> aroonosc for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    aroonosc_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    options := []float64{25.0} // period

    // Full computation — Rows is just [aroonosc], valid until Close.
    res, st, _ := indicators.Aroonosc.Indicator(high, low, options, nil)
    fmt.Println(res.Rows[0]) // Aroon Oscillator values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Aroonosc.Indicator(high[:8], low[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], nil)
    fmt.Println(batch.Rows[0]) // continued Aroon Oscillator values
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

    outputs, state = tulip_rs.indicators.aroonosc.indicator([high, low], [25.0])
    print(outputs[0])  # Aroon Oscillator values

    # State continuation
    new_high = np.array([85.30], dtype=np.float64)
    new_low  = np.array([84.60], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low  = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);

    const [outputs, state] = ti.aroonosc.indicator([high, low], [25]);
    console.log('Aroon Oscillator:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.aroonosc.indicator([high.slice(0, n), low.slice(0, n)], [25]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued AroonOsc:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.aroonosc.indicator([high, low], [25]);
    console.log('Aroon Oscillator:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.aroonosc.indicator([high.slice(0, n), low.slice(0, n)], [25]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued AroonOsc:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `aroonosc` exposes 2 optional outputs: `aroon_down`, `aroon_up`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::aroonosc::{AroonOsc, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low   = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();

    let mask = [true, true];
    let (outputs, _state) = AroonOsc::indicator(
        &[high.as_slice(), low.as_slice()],
        &[25.0],
        Some(&mask),
    ).unwrap();

    let aroonosc  = &outputs[0]; // aroonosc (primary)
    let aroon_down = &outputs[1]; // aroon_down (optional — requested)
    let aroon_up   = &outputs[2]; // aroon_up (optional — requested)
    ```

=== "C"

    `aroonosc` exposes 2 optional outputs: `aroon_down`, `aroon_up`. Pass a boolean mask — one `bool` per optional output, in header order.

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    double options[AROONOSC_OPTIONS] = {25.0}; // period
    const double *inputs[AROONOSC_INPUTS] = {high, low};
    bool optional_outputs[2] = {true, true}; // aroon_down, aroon_up

    CIndicatorResult r = aroonosc_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> aroonosc (primary), length r.output_lens[0] */
    /* r.outputs[1] -> aroon_down (optional), length r.output_lens[1] */
    /* r.outputs[2] -> aroon_up (optional),   length r.output_lens[2] */
    tulip_ffi_result_free(r);
    aroonosc_state_free(r.state);
    ```

=== "Go"

    `aroonosc` exposes 2 optional outputs: `aroon_down`, `aroon_up`. By-options isn't offered.

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    high := make([]float64, len(close))
    low := make([]float64, len(close))
    for i := range close {
        high[i] = close[i] + 1.0
        low[i] = close[i] - 1.0
    }
    options := []float64{25.0} // period
    mask := []bool{true, true} // aroon_down, aroon_up

    res, _st, _ := indicators.Aroonosc.Indicator(high, low, options, mask)

    aroonosc  := res.Rows[0]   // aroonosc (primary)
    aroonDown := res.Rows[1]   // aroon_down (optional — requested)
    aroonUp   := res.Rows[2]   // aroon_up (optional — requested)
    res.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high  = close + 1.0
    low   = close - 1.0

    outputs, state = tulip_rs.indicators.aroonosc.indicator(
        [high, low], [25.0],
        optional_outputs=[True, True],
    )

    aroonosc   = outputs[0]  # aroonosc (primary)
    aroon_down = outputs[1]  # aroon_down (optional — requested)
    aroon_up   = outputs[2]  # aroon_up (optional — requested)
    ```

=== "Node.js"

    `aroonosc` exposes 2 optional outputs: `aroon_down`, `aroon_up`.

    ```javascript
    const [allOut] = ti.aroonosc.indicator([high, low], [25], [true, true]);
    const aroonosc  = allOut[0]; // primary
    const aroonDown = allOut[1]; // optional 0: aroon_down
    const aroonUp   = allOut[2]; // optional 1: aroon_up
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.aroonosc.indicator([high, low], [25], [true, true]);
    const aroonosc  = allOut[0]; // primary
    const aroonDown = allOut[1]; // optional 0: aroon_down
    const aroonUp   = allOut[2]; // optional 1: aroon_up
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::aroonosc::{AroonOsc, Indicator};

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = AroonOsc::indicator_by_assets::<4>(&inputs, &[25.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::aroonosc::{AroonOsc, IndicatorByOptions};

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[25.0], &[50.0]];
    let results = AroonOsc::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "C"

    **By assets** — same option applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double a1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double l2[] = {71.10, 71.85, 72.40, 72.00, 73.20, 73.85, 74.10, 74.60, 75.00, 75.50};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[AROONOSC_INPUTS] = {a1, l1};
    const double *asset2[AROONOSC_INPUTS] = {a2, l2};
    const double *const *const simd_inputs[4] = {asset1, asset2, NULL, NULL}; /* N=4 lanes */
    double options[AROONOSC_OPTIONS] = {25.0};
    bool optional_outputs[2] = {true, true};

    CSimdResult r = aroonosc_simd_by_assets(simd_inputs, 4, 10, options, optional_outputs, 2);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's aroonosc, length r.output_lens[i][0] */
        aroonosc_state_free(r.states[i]);
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
    const double *inputs[AROONOSC_INPUTS] = {high, low};

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
    const double *expanded_inputs[AROONOSC_INPUTS] = {high_expanded, low_expanded};

    static const double o5[AROONOSC_OPTIONS]   = {5.0};
    static const double o10[AROONOSC_OPTIONS]  = {10.0};
    static const double o25[AROONOSC_OPTIONS]  = {25.0};
    static const double o50[AROONOSC_OPTIONS]  = {50.0};
    const double *const simd_opts[4] = {o5, o10, o25, o50};

    CSimdResult r = aroonosc_simd_by_options(expanded_inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) aroonosc_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11}

    // Reuse the same data for assets 2–4 in this example
    h2, l2 := h1, l1
    h3, l3 := h1, l1
    h4, l4 := h1, l1

    assets := [][indicators.AroonoscInputs][]float64{{h1, l1}, {h2, l2}, {h3, l3}, {h4, l4}}
    sim, _ := indicators.Aroonosc.SimdByAssets(assets, []float64{25.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0]) // aroonosc
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}

    // Tile the series 20x so longer-period option sets have enough data
    #define EXPANDED_LEN (10 * 20)
    high_exp := make([]float64, EXPANDED_LEN)
    low_exp := make([]float64, EXPANDED_LEN)
    for i := 0; i < 20; i++ {
        for j := 0; j < 10; j++ {
            high_exp[i*10+j] = high[j]
            low_exp[i*10+j] = low[j]
        }
    }

    sim2, _ := indicators.Aroonosc.SimdByOptions(high_exp, low_exp, [][]float64{{5.0}, {10.0}, {25.0}, {50.0}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
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
    outputs_list, states = tulip_rs.indicators.aroonosc.simd_by_assets(simd_inputs, [25.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [25.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.aroonosc.simd_by_options(
        [high, low], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
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
    const [results] = ti.aroonosc.simdByAssets(simdInputs, [25]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [25], [50]];
    const [results] = ti.aroonosc.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
