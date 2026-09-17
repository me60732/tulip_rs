# AO — Awesome Oscillator

Measures market momentum as the difference between a 5-period and 34-period simple moving average of each bar's midpoint `(high + low) / 2`. No options are required.

**Inputs:** `[high, low]` &nbsp;|&nbsp; **Options:** `[]` (none) &nbsp;|&nbsp; **Outputs:** `[ao]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::ao::{Ao, TIndicatorState, Indicator};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                    90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                    93.80, 94.20, 94.60, 95.00, 95.30_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                    89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                    93.10, 93.50, 93.90, 94.30, 94.60_f64];

    // AO takes no options — pass an empty slice
    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, _state) = Ao::indicator(&inputs, &[], None).unwrap();
    println!("AO: {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..30], &low[..30]];
    let (outputs2, mut state) = Ao::indicator(&inputs2, &[], None).unwrap();
    println!("Partial AO: {:?}", outputs2[0]);

    let new_inputs = [&high[30..], &low[30..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued AO: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                     90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                     93.80, 94.20, 94.60, 95.00, 95.30};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60};
    const double options[AO_OPTIONS] = {}; // no options (AO_OPTIONS == 0)

    /* AO takes 2 inputs: high, low */
    const double *inputs[AO_INPUTS] = {high, low};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = ao_indicator(inputs, 35, options, NULL, 0);
    /* r.outputs[0] -> the AO series, length r.output_lens[0];
       r.outputs[1] -> short_sma (optional — not requested here) */
    tulip_ffi_result_free(r);
    ao_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = ao_indicator(inputs, 30, options, NULL, 0);
    double new_high[] = {93.80, 94.20, 94.60, 95.00, 95.30};
    double new_low[]  = {93.10, 93.50, 93.90, 94.30, 94.60};
    const double *new_inputs[AO_INPUTS] = {new_high, new_low};
    CBatchResult b = ao_batch(p.state, new_inputs, 5, NULL, 0);
    /* b.outputs[0] -> AO values for just the five new bars */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    ao_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                      90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                      93.80, 94.20, 94.60, 95.00, 95.30}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60}
    options := []float64{} // no options

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Ao.Indicator(high, low, options, nil)
    fmt.Println(res.Rows[0]) // AO values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Ao.Indicator(high[:30], low[:30], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[30:], low[30:], nil)
    fmt.Println(batch.Rows[0]) // continued AO values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    # AO requires at least 34 bars (34-period SMA)
    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                     90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                     93.80, 94.20, 94.60, 95.00, 95.30], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60], dtype=np.float64)

    # AO takes no options — pass an empty list
    outputs, state = tulip_rs.indicators.ao.indicator([high, low], [])
    print("AO:", outputs[0])

    # State continuation
    outputs2, state = tulip_rs.indicators.ao.indicator([high[:30], low[:30]], [])
    continued = state.batch_indicator([high[30:], low[30:]])
    print("Continued AO:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    // AO requires at least 34 bars (34-period SMA)
    const high = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                  85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                  90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                  93.80, 94.20, 94.60, 95.00, 95.30]);
    const low  = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                  84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                  89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                  93.10, 93.50, 93.90, 94.30, 94.60]);

    const [outputs, state] = ti.ao.indicator([high, low], []);
    console.log('AO:', outputs[0]);

    // State continuation
    const [, state2] = ti.ao.indicator([high.slice(0, 30), low.slice(0, 30)], []);
    const continued = state2.batchIndicator([high.slice(30), low.slice(30)]);
    console.log('Continued AO:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    // AO requires at least 34 bars (34-period SMA)
    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                  85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                  90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                  93.80, 94.20, 94.60, 95.00, 95.30];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                  84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                  89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                  93.10, 93.50, 93.90, 94.30, 94.60];

    const [outputs, state] = ti.ao.indicator([high, low], []);
    console.log('AO:', outputs[0]);

    // State continuation
    const [, state2] = ti.ao.indicator([high.slice(0, 30), low.slice(0, 30)], []);
    const continued = state2.batchIndicator([high.slice(30), low.slice(30)]);
    console.log('Continued AO:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `ao` exposes 3 optional outputs: `short_sma`, `long_sma`, `medprice`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::ao::{Ao, TIndicatorState, Indicator};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                    90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                    93.80, 94.20, 94.60, 95.00, 95.30_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                    89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                    93.10, 93.50, 93.90, 94.30, 94.60_f64];

    let mask = [true, false, false]; // one per optional output
    let (outputs, _state) = Ao::indicator(&[high.as_slice(), low.as_slice()], &[], Some(&mask)).unwrap();

    let ao        = &outputs[0]; // ao (primary)
    let short_sma = &outputs[1]; // short_sma (optional — requested)
    // long_sma and medprice not requested
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                     90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                     93.80, 94.20, 94.60, 95.00, 95.30};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60};
    const double options[AO_OPTIONS] = {}; // no options

    /* AO takes 2 inputs: high, low */
    const double *inputs[AO_INPUTS] = {high, low};
    bool optional_outputs[3] = {true, false, false}; // short_sma=true, long_sma=false, medprice=false

    CIndicatorResult r = ao_indicator(inputs, 35, options, optional_outputs, 3);
    /* r.outputs[0] -> ao (primary)
       r.outputs[1] -> short_sma (optional — requested) */
    tulip_ffi_result_free(r);
    ao_state_free(r.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                      90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                      93.80, 94.20, 94.60, 95.00, 95.30}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60}
    options := []float64{} // no options

    // Request only short_sma (optional outputs: short_sma, long_sma, medprice)
    res, st, _ := indicators.Ao.Indicator(high, low, options, []bool{true, false, false})
    fmt.Println(res.Rows[0]) // ao (primary)
    fmt.Println(res.Rows[1]) // "short_sma" (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                     90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                     93.80, 94.20, 94.60, 95.00, 95.30], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60], dtype=np.float64)

    outputs, state = tulip_rs.indicators.ao.indicator(
        [high, low], [],
        optional_outputs=[True, False, False],
    )

    ao        = outputs[0]  # ao (primary)
    short_sma = outputs[1]  # short_sma (optional — requested)
    # long_sma and medprice not requested
    ```

=== "Node.js"

    `ao` exposes 3 optional outputs: `short_sma`, `long_sma`, `medprice`.

    ```javascript
    // Request all optional outputs
    const [allOut] = ti.ao.indicator([high, low], [], [true, true, true]);
    const ao       = allOut[0]; // primary
    const shortSma = allOut[1]; // optional 0: short_sma
    const longSma  = allOut[2]; // optional 1: long_sma
    const medprice = allOut[3]; // optional 2: medprice

    // Request only short_sma
    const [partial] = ti.ao.indicator([high, low], [], [true, false, false]);
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    // Request all optional outputs
    const [allOut] = ti.ao.indicator([high, low], [], [true, true, true]);
    const ao       = allOut[0]; // primary
    const shortSma = allOut[1]; // optional 0: short_sma
    const longSma  = allOut[2]; // optional 1: long_sma
    const medprice = allOut[3]; // optional 2: medprice

    // Request only short_sma
    const [partial] = ti.ao.indicator([high, low], [], [true, false, false]);
    ```
### SIMD

=== "Rust"

    **By assets** — applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ao::{Ao, Indicator};

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                  85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                  90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                  93.80, 94.20, 94.60, 95.00, 95.30_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                  84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                  89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                  93.10, 93.50, 93.90, 94.30, 94.60_f64];
    let h2 = h1.clone(); let l2 = l1.clone();
    let h3 = h1.clone(); let l3 = l1.clone();
    let h4 = h1.clone(); let l4 = l1.clone();

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];

    let results = Ao::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

=== "C"

    **By assets** — applied to 4 assets in parallel:

    ```c
    #include "tulip_rs_ffi.h"

    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                   90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                   93.80, 94.20, 94.60, 95.00, 95.30};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                   89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                   93.10, 93.50, 93.90, 94.30, 94.60};

    /* AO has 2 inputs: high, low — fill ALL fields per asset! */
    const double *const asset1[AO_INPUTS] = {h1, l1};
    const double *const asset2[AO_INPUTS] = {h1, l1};
    const double *const asset3[AO_INPUTS] = {h1, l1};
    const double *const asset4[AO_INPUTS] = {h1, l1};

    /* simd_inputs is indexed by asset (the N=4 SIMD lanes) */
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    const double options[AO_OPTIONS] = {}; // no options

    CSimdResult r = ao_simd_by_assets(simd_inputs, 4, 35, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's AO series, length r.output_lens[i][0];
           r.outputs[i][1] -> short_sma (optional — not requested here) */
        ao_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                    90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                    93.80, 94.20, 94.60, 95.00, 95.30}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                    89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                    93.10, 93.50, 93.90, 94.30, 94.60}

    // Reuse the same data for assets 2–4 in this example
    h2, l2 := h1, l1
    h3, l3 := h1, l1
    h4, l4 := h1, l1

    assets := [][indicators.AoInputs][]float64{{h1, l1}, {h2, l2}, {h3, l3}, {h4, l4}}
    sim, _ := indicators.Ao.SimdByAssets(assets, []float64{}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                     90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                     93.80, 94.20, 94.60, 95.00, 95.30], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60], dtype=np.float64)

    simd_inputs = [
        [high,        low],
        [high + 0.5,  low + 0.5],
        [high - 0.5,  low - 0.5],
        [high * 1.01, low * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.ao.simd_by_assets(simd_inputs, [])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.ao.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
