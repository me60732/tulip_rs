# CYBERCYCLE — Ehlers CyberCycle

Isolates the dominant market cycle from price data using a high-pass filter followed by cycle extraction; `trigger` is the one-bar-delayed cybercycle used for crossover signals.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[alpha]` &nbsp;|&nbsp; **Outputs:** `[cybercycle]` &nbsp;|&nbsp; **Optional:** `[trigger]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::cybercycle::{Cybercycle, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    // Options: [alpha] — default 0.07
    let (outputs, _state) = Cybercycle::indicator(&[close.as_slice()], &[0.07], None).unwrap();
    println!("CyberCycle: {:?}", outputs[0]);

    // State continuation
    let partial = close[..35].to_vec();
    let (outputs2, mut state) = Cybercycle::indicator(&[partial.as_slice()], &[0.07], None).unwrap();
    println!("Partial CyberCycle: {:?}", outputs2[0]);

    let new_close = close[35..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued CyberCycle: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[CYBERCYCLE_OPTIONS] = {0.07}; // alpha
    const double *inputs[CYBERCYCLE_INPUTS] = {close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = cybercycle_indicator(inputs, 40, options, NULL, 0);
    /* r.outputs[0] -> cybercycle_line */
    tulip_ffi_result_free(r);
    cybercycle_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = cybercycle_indicator(inputs, 35, options, NULL, 0);
    double new_close[] = {92.80, 93.10, 92.50, 93.20};
    const double *new_inputs[CYBERCYCLE_INPUTS] = {new_close};
    CBatchResult b = cybercycle_batch(p.state, new_inputs, 4, NULL, 0);
    /* b.outputs[0] -> cybercycle_line for the 4 new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    cybercycle_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{0.07} // alpha

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Cybercycle.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // cybercycle_line values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Cybercycle.Indicator(close[:35], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[35:], nil)
    fmt.Println(batch.Rows[0]) // continued cybercycle_line values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    # Options: [alpha] — default 0.07
    outputs, state = tulip_rs.indicators.cybercycle.indicator([close], [0.07])
    print("CyberCycle:", outputs[0])

    # State continuation
    partial = close[:35]
    outputs2, state = tulip_rs.indicators.cybercycle.indicator([partial], [0.07])
    new_close = close[35:]
    continued = state.batch_indicator([new_close])
    print("Continued CyberCycle:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20]);

    const [outputs, state] = ti.cybercycle.indicator([close], [0.07]);
    console.log('CyberCycle:', outputs[0]);

    // State continuation
    const [, state2] = ti.cybercycle.indicator([close.slice(0, -5)], [0.07]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued CyberCycle:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init, cybercycle } from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20];

    const [outputs, state] = cybercycle.indicator([close], [0.07]);
    console.log('CyberCycle:', outputs[0]);

    // State continuation
    const [, state2] = cybercycle.indicator([close.slice(0, -5)], [0.07]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued CyberCycle:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `cybercycle` exposes 1 optional output: `trigger`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::cybercycle::{Cybercycle, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    let mask = [true]; // one per optional output
    let (outputs, _state) = Cybercycle::indicator(&[close.as_slice()], &[0.07], Some(&mask)).unwrap();

    let cybercycle = &outputs[0]; // cybercycle (primary)
    let trigger    = &outputs[1]; // trigger (optional — requested)
    ```

=== "C"

    `cybercycle` exposes 1 optional output: `trigger`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[CYBERCYCLE_OPTIONS] = {0.07}; // alpha
    const double *inputs[CYBERCYCLE_INPUTS] = {close};
    bool optional_outputs[1] = {true}; // trigger

    CIndicatorResult r = cybercycle_indicator(inputs, 40, options, optional_outputs, 1);
    /* r.outputs[0] -> cybercycle_line (primary) */
    /* r.outputs[1] -> trigger (optional — requested) */
    tulip_ffi_result_free(r);
    cybercycle_state_free(r.state);
    ```

=== "Go"

    `cybercycle` exposes 1 optional output: `trigger`. Pass a boolean mask as the third argument.

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{0.07} // alpha
    mask := []bool{true} // trigger

    res, st, _ := indicators.Cybercycle.Indicator(close, options, mask)
    cybercycle := res.Rows[0]  // cybercycle (primary)
    trigger    := res.Rows[1]  // trigger (optional — requested)

    fmt.Println(cybercycle)
    fmt.Println(trigger)

    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    outputs, state = tulip_rs.indicators.cybercycle.indicator(
        [close], [0.07],
        optional_outputs=[True],
    )

    cybercycle = outputs[0]  # cybercycle (primary)
    trigger    = outputs[1]  # trigger (optional — requested)
    ```

=== "Node.js"

    `cybercycle` exposes 1 optional output: `trigger`.

    ```javascript
    const [allOut] = ti.cybercycle.indicator([close], [0.07], [true]);
    const cyberCycle = allOut[0]; // primary
    const trigger    = allOut[1]; // optional 0: trigger
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = cybercycle.indicator([close], [0.07], [true]);
    const cyberCycle = allOut[0]; // primary
    const trigger    = allOut[1]; // optional 0: trigger
    ```

### SIMD

=== "Rust"

    **By assets** — same alpha applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::cybercycle::{Cybercycle, Indicator, TIndicatorState};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                  85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                  88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                  90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                  77.00, 77.50, 78.00, 78.50, 79.00, 79.50, 80.00, 80.50, 81.00, 81.50,
                  82.00, 82.50, 83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50,
                  87.00, 87.50, 88.00, 88.50, 89.00, 89.50, 90.00, 90.50, 91.00, 91.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                  58.70, 59.00, 59.30, 59.60, 59.90, 60.20, 60.50, 60.80, 61.10, 61.40,
                  61.70, 62.00, 62.30, 62.60, 62.90, 63.20, 63.50, 63.80, 64.10, 64.40,
                  64.70, 65.00, 65.30, 65.60, 65.90, 66.20, 66.50, 66.80, 67.10, 67.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                  104.1, 104.5, 105.0, 105.3, 105.8, 106.0, 106.5, 107.0, 107.3, 107.8,
                  108.1, 108.5, 109.0, 109.3, 109.8, 110.0, 110.5, 111.0, 111.3, 111.8,
                  112.1, 112.5, 113.0, 113.3, 113.8, 114.0, 114.5, 115.0, 115.3, 115.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = Cybercycle::indicator_by_assets::<4>(&inputs, &[0.07], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```rust
    use tulip_rs::indicators::cybercycle::{Cybercycle, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    let opts: [&[f64; 1]; 4] = [&[0.05], &[0.07], &[0.10], &[0.15]];

    let results = Cybercycle::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Alpha set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same alpha applied to 4 assets in parallel:

    ```c
    #include "tulip_rs_ffi.h"

    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                   77.00, 77.50, 78.00, 78.50, 79.00, 79.50, 80.00, 80.50, 81.00, 81.50,
                   82.00, 82.50, 83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50,
                   87.00, 87.50, 88.00, 88.50, 89.00, 89.50, 90.00, 90.50, 91.00, 91.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                   58.70, 59.00, 59.30, 59.60, 59.90, 60.20, 60.50, 60.80, 61.10, 61.40,
                   61.70, 62.00, 62.30, 62.60, 62.90, 63.20, 63.50, 63.80, 64.10, 64.40,
                   64.70, 65.00, 65.30, 65.60, 65.90, 66.20, 66.50, 66.80, 67.10, 67.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                   104.1, 104.5, 105.0, 105.3, 105.8, 106.0, 106.5, 107.0, 107.3, 107.8,
                   108.1, 108.5, 109.0, 109.3, 109.8, 110.0, 110.5, 111.0, 111.3, 111.8,
                   112.1, 112.5, 113.0, 113.3, 113.8, 114.0, 114.5, 115.0, 115.3, 115.8};

    const double *asset1[CYBERCYCLE_INPUTS] = {a1};
    const double *asset2[CYBERCYCLE_INPUTS] = {a2};
    const double *asset3[CYBERCYCLE_INPUTS] = {a3};
    const double *asset4[CYBERCYCLE_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = cybercycle_simd_by_assets(simd_inputs, 4, 40, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's cybercycle_line series */
        cybercycle_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};

    /* Tile the series 10x so longer-period option sets have enough data */
    #define EXPANDED_LEN (40 * 10)
    static double close_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 10; i++) {
        for (size_t j = 0; j < 40; j++) {
            close_expanded[i * 40 + j] = close[j];
        }
    }
    const double *inputs[CYBERCYCLE_INPUTS] = {close_expanded};

    static const double o05[CYBERCYCLE_OPTIONS] = {0.05};
    static const double o07[CYBERCYCLE_OPTIONS] = {0.07};
    static const double o10[CYBERCYCLE_OPTIONS] = {0.10};
    static const double o15[CYBERCYCLE_OPTIONS] = {0.15};
    const double *const simd_opts[4] = {o05, o07, o10, o15};

    CSimdResult r = cybercycle_simd_by_options(inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's cybercycle_line series */
        cybercycle_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same alpha applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                    85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                    88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                    90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := a1, a1, a1
    options := []float64{0.07} // alpha

    assets := [][indicators.CybercycleInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Cybercycle.SimdByAssets(assets, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := [][]float64{{0.05}, {0.07}, {0.10}, {0.15}}

    sim, _ := indicators.Cybercycle.SimdByOptions(close, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Alpha set %d: %v\n", i+1, lanes[0])
    }
    sim.Close()
    ```

=== "Python"

    **By assets** — same alpha applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.cybercycle.simd_by_assets(simd_inputs, [0.07])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different alpha values in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    simd_options = [[0.05], [0.07], [0.10], [0.15]]
    outputs_list, states = tulip_rs.indicators.cybercycle.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Alpha set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same alpha applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.cybercycle.simdByAssets(simdInputs, [0.07]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different alpha values in parallel:

    ```javascript
    const simdOptions = [[0.05], [0.07], [0.10], [0.15]];
    const [results] = ti.cybercycle.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Alpha set ${i + 1}:`, out[0]));
    ```
