# StochRSI — Stochastic RSI

Applies the Stochastic Oscillator formula to RSI values rather than price, producing an extremely sensitive momentum indicator that oscillates between 0 and 100.

!!! note "Scaling differs from most libraries"
    In the original publication — Chande & Kroll, *The New Technical Trader* (1994) — the
    StochRSI formula was printed without the ×100 scaling factor. This typesetting error
    led most indicator libraries to adopt a 0–1 ratio instead. This implementation uses
    the corrected 0–100 scale, consistent with the standard Stochastic Oscillator (%K).
    Users migrating from other libraries should divide their expected values by 100.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[stochrsi]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::stochrsi::{StochRsi, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = StochRsi::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("StochRSI(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = StochRsi::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial StochRSI: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued StochRSI: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[STOCHRSI_OPTIONS] = {14.0};
    const double *inputs[STOCHRSI_INPUTS] = {close};

    /* Full computation */
    CIndicatorResult r = stochrsi_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the StochRSI(14) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    stochrsi_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = stochrsi_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {84.55, 84.36};
    const double *new_inputs[STOCHRSI_INPUTS] = {new_close};
    CBatchResult b = stochrsi_batch(p.state, new_inputs, 2, NULL, 0);
    /* b.outputs[0] -> StochRSI values for just the two new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    stochrsi_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0}

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Stochrsi.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // StochRSI(14) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Stochrsi.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued StochRSI values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.stochrsi.indicator([close], [14.0])
    print("StochRSI(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.stochrsi.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued StochRSI:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.stochrsi.indicator([close], [14]);
    console.log('StochRSI(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.stochrsi.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued StochRSI:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.stochrsi.indicator([close], [14]);
    console.log('StochRSI(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.stochrsi.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued StochRSI:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `stochrsi` exposes 1 optional output: `rsi`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::stochrsi::{StochRsi, TIndicatorState, Indicator};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // one per optional output
    let (outputs, _state) = StochRsi::indicator(&[close.as_slice()], &[14.0], Some(&mask)).unwrap();

    let stochrsi = &outputs[0]; // stochrsi (primary)
    let rsi      = &outputs[1]; // rsi (optional — requested)
    ```

=== "C"

    `stochrsi` exposes 1 optional output: `rsi`.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[STOCHRSI_OPTIONS] = {14.0};
    const double *inputs[STOCHRSI_INPUTS] = {close};

    bool optional_outputs[1] = {true}; // request rsi
    CIndicatorResult r = stochrsi_indicator(inputs, 10, options, optional_outputs, 1);
    /* r.outputs[0] -> stochrsi (primary) */
    /* r.outputs[1] -> rsi (optional — requested) */
    tulip_ffi_result_free(r);
    stochrsi_state_free(r.state);
    ```

=== "Go"

    `stochrsi` exposes 1 optional output: `rsi`. Pass a boolean mask as the third argument.

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0}

    mask := []bool{true} // request rsi
    res, st, _ := indicators.Stochrsi.Indicator(close, options, mask)

    stochrsi := res.Rows[0] // stochrsi (primary)
    rsi      := res.Rows[1] // rsi (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.stochrsi.indicator(
        [close], [14.0],
        optional_outputs=[True],
    )

    stochrsi = outputs[0]  # stochrsi (primary)
    rsi      = outputs[1]  # rsi (optional — requested)
    ```

=== "Node.js"

    `stochrsi` exposes 1 optional output: `rsi`.

    ```javascript
    const [allOut] = ti.stochrsi.indicator([close], [14], [true]);
    const stochrsi = allOut[0]; // primary
    const rsi      = allOut[1]; // optional 0: rsi
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.stochrsi.indicator([close], [14], [true]);
    const stochrsi = allOut[0]; // primary
    const rsi      = allOut[1]; // optional 0: rsi
    ```
### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::stochrsi::{StochRsi, Indicator};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = StochRsi::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::stochrsi::{StochRsi, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];

    let results = StochRsi::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel:

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50};
    double a3[] = {55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40};
    double a4[] = {100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8};

    const double *asset1[STOCHRSI_INPUTS] = {a1};
    const double *asset2[STOCHRSI_INPUTS] = {a2};
    const double *asset3[STOCHRSI_INPUTS] = {a3};
    const double *asset4[STOCHRSI_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = stochrsi_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's StochRSI series */
        stochrsi_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    /* Tile the series 20x so longer-period option sets have enough data */
    #define EXPANDED_LEN (10 * 20)
    static double close_expanded[EXPANDED_LEN];
    for (size_t i = 0; i < 20; i++) {
        for (size_t j = 0; j < 10; j++) {
            close_expanded[i * 10 + j] = close[j];
        }
    }
    const double *expanded_inputs[STOCHRSI_INPUTS] = {close_expanded};

    static const double o7[STOCHRSI_OPTIONS]  = {7.0};
    static const double o14[STOCHRSI_OPTIONS] = {14.0};
    static const double o21[STOCHRSI_OPTIONS] = {21.0};
    static const double o28[STOCHRSI_OPTIONS] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r2 = stochrsi_simd_by_options(expanded_inputs, EXPANDED_LEN, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r2.num_results; i++) {
        /* r2.outputs[i][0] -> StochRSI for period simd_opts[i] */
        stochrsi_state_free(r2.states[i]);
    }
    tulip_ffi_simd_result_free(r2);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    c2 := c1
    c3 := c1
    c4 := c1

    assets := [][indicators.StochrsiInputs][]float64{{c1}, {c2}, {c3}, {c4}}
    sim, _ := indicators.Stochrsi.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Stochrsi.SimdByOptions(close, [][]float64{{7}, {14}, {21}, {28}}, nil)
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

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.stochrsi.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.stochrsi.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.stochrsi.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.stochrsi.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
