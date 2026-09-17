# APO — Absolute Price Oscillator

The raw difference between two EMAs (short minus long). Positive values indicate upward momentum.

**Inputs:** `[real]` | **Options:** `[short_period, long_period]` | **Outputs:** `[apo]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::apo::{Apo, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // options: [short_period, long_period]
    let (outputs, mut state) = Apo::indicator(&[close.as_slice()], &[12.0, 26.0], None).unwrap();
    println!("{:?}", outputs[0]); // APO values

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Apo::indicator(&[partial.as_slice()], &[12.0, 26.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[APO_OPTIONS] = {12.0, 26.0}; // short_period, long_period
    const double *inputs[APO_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = apo_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the APO series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    apo_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = apo_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.53};
    const double *new_inputs[APO_INPUTS] = {new_close};
    CBatchResult b = apo_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> APO values for the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    apo_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{12.0, 26.0} // short_period, long_period

    // Full computation — Rows is just [apo], valid until Close.
    res, st, _ := indicators.Apo.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // APO values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Apo.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued APO values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.apo.indicator([close], [12.0, 26.0])
    print(outputs[0])  # APO values

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

    const [outputs, state] = ti.apo.indicator([close], [12, 26]);
    console.log('APO:', outputs[0]);

    // State continuation
    const [, state2] = ti.apo.indicator([close.slice(0, -3)], [12, 26]);
    const continued = state2.batchIndicator([close.slice(-3)]);
    console.log('Continued APO:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.apo.indicator([close], [12, 26]);
    console.log('APO:', outputs[0]);

    // State continuation
    const [, state2] = ti.apo.indicator([close.slice(0, -3)], [12, 26]);
    const continued = state2.batchIndicator([close.slice(-3)]);
    console.log('Continued APO:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `apo` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::apo::{Apo, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true];
    let (outputs, _state) = Apo::indicator(&[close.as_slice()], &[5.0, 20.0], Some(&mask)).unwrap();

    let apo       = &outputs[0]; // APO values (primary)
    let short_ema = &outputs[1]; // short_ema (optional — requested)
    let long_ema  = &outputs[2]; // long_ema (optional — requested)
    ```

=== "C"

    `apo` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask — one `bool` per optional output, in header order.

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double options[APO_OPTIONS] = {5.0, 20.0}; // short_period, long_period
    const double *inputs[APO_INPUTS] = {close};
    bool optional_outputs[2] = {true, true}; // short_ema, long_ema

    CIndicatorResult r = apo_indicator(inputs, 10, options, optional_outputs, 2);
    /* r.outputs[0] -> apo (primary), length r.output_lens[0] */
    /* r.outputs[1] -> short_ema (optional), length r.output_lens[1] */
    /* r.outputs[2] -> long_ema (optional), length r.output_lens[2] */
    tulip_ffi_result_free(r);
    apo_state_free(r.state);
    ```

=== "Go"

    `apo` exposes 2 optional outputs: `short_ema`, `long_ema`.

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{5.0, 20.0} // short_period, long_period
    mask := []bool{true, true} // short_ema, long_ema

    res, _st, _ := indicators.Apo.Indicator(close, options, mask)

    apo       := res.Rows[0] // APO values (primary)
    shortEma  := res.Rows[1] // short_ema (optional — requested)
    longEma   := res.Rows[2] // long_ema (optional — requested)
    res.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.apo.indicator(
        [close], [5.0, 20.0],
        optional_outputs=[True, True],
    )

    apo       = outputs[0]  # APO values (primary)
    short_ema = outputs[1]  # short_ema (optional — requested)
    long_ema  = outputs[2]  # long_ema (optional — requested)
    ```

=== "Node.js"

    `apo` exposes 2 optional outputs: `short_ema`, `long_ema`.

    ```javascript
    const [allOut] = ti.apo.indicator([close], [12, 26], [true, true]);
    const apo      = allOut[0]; // primary
    const shortEma = allOut[1]; // optional 0: short_ema
    const longEma  = allOut[2]; // optional 1: long_ema
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.apo.indicator([close], [12, 26], [true, true]);
    const apo      = allOut[0]; // primary
    const shortEma = allOut[1]; // optional 0: short_ema
    const longEma  = allOut[2]; // optional 1: long_ema
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::apo::{Apo, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = Apo::indicator_by_assets::<4>(&inputs, &[12.0, 26.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::apo::{Apo, IndicatorByOptions};

    let opts: [&[f64; 2]; 4] = [&[6.0, 13.0], &[12.0, 26.0], &[19.0, 39.0], &[24.0, 52.0]];
    let results = Apo::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50}
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40}
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8}

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[APO_INPUTS] = {a1};
    const double *asset2[APO_INPUTS] = {a2};
    const double *asset3[APO_INPUTS] = {a3};
    const double *asset4[APO_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    double options[APO_OPTIONS] = {12.0, 26.0}; // short_period, long_period
    bool optional_outputs[2] = {true, true};

    CSimdResult r = apo_simd_by_assets(simd_inputs, 4, 10, options, optional_outputs, 2);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        apo_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in one call:

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[APO_INPUTS] = {close};

    /* Tile the series 20x so longer-period option sets have enough data */
    #define EXPANDED_LEN (10 * 20)
    static double close_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 20; i++)
        for (size_t j = 0; j < 10; j++) close_expanded[i * 10 + j] = close[j];
    const double *expanded_inputs[APO_INPUTS] = {close_expanded};

    static const double o6_13[APO_OPTIONS] = {6.0, 13.0};
    static const double o12_26[APO_OPTIONS] = {12.0, 26.0};
    static const double o19_39[APO_OPTIONS] = {19.0, 39.0};
    static const double o24_52[APO_OPTIONS] = {24.0, 52.0};
    const double *const simd_opts[4] = {o6_13, o12_26, o19_39, o24_52};

    CSimdResult r = apo_simd_by_options(expanded_inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) apo_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := a1, a1, a1

    assets := [][indicators.ApoInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Apo.SimdByAssets(assets, []float64{12.0, 26.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    // Tile the series so longer-period option sets have enough data
    expanded := make([]float64, len(close)*20)
    for i := 0; i < 20; i++ {
        copy(expanded[i*len(close):], close)
    }

    sim2, _ := indicators.Apo.SimdByOptions(expanded, [][]float64{{6.0, 13.0}, {12.0, 26.0}, {19.0, 39.0}, {24.0, 52.0}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Option set %d: %v\n", i+1, lanes[0])
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
    outputs_list, states = tulip_rs.indicators.apo.simd_by_assets(simd_inputs, [12.0, 26.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[6.0, 13.0], [12.0, 26.0], [19.0, 39.0], [24.0, 52.0]]
    outputs_list, states = tulip_rs.indicators.apo.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.apo.simdByAssets(simdInputs, [12, 26]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[6, 13], [12, 26], [19, 39], [24, 52]];
    const [results] = ti.apo.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```
