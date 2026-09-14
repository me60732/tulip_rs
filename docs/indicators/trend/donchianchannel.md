# Donchian Channel

Three-band channel based on the rolling highest high and lowest low over `period` bars. `upper = max(high, period)`, `lower = min(low, period)`, `middle = (upper + lower) / 2`. Used to identify breakouts — a close above the upper band signals bullish momentum. Rendered as a price overlay.

**Inputs:** `[high, low]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[lower, middle, upper]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::donchianchannel::{DonchianChannel, Indicator, TIndicatorState};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = DonchianChannel::indicator(&inputs, &[14.0], None).unwrap();
    println!("Lower:  {:?}", outputs[0]);
    println!("Middle: {:?}", outputs[1]);
    println!("Upper:  {:?}", outputs[2]);

    // State continuation — feed new bars without reprocessing history
    let partial_high = high[..8].to_vec();
    let partial_low  = low[..8].to_vec();
    let (outputs2, mut state) = DonchianChannel::indicator(&[partial_high.as_slice(), partial_low.as_slice()], &[14.0], None).unwrap();
    println!("Lower:  {:?}", outputs2[0]);
    println!("Middle: {:?}", outputs2[1]);
    println!("Upper:  {:?}", outputs2[2]);

    let new_high = vec![85.90_f64];
    let new_low  = vec![84.03_f64];
    let continued = state.batch_indicator(&[new_high.as_slice(), new_low.as_slice()], None).unwrap();
    println!("Continued Lower:  {:?}", continued[0]);
    println!("Continued Middle: {:?}", continued[1]);
    println!("Continued Upper:  {:?}", continued[2]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    double options[DONCHIANCHANNEL_OPTIONS] = {14.0};
    const double *inputs[DONCHIANCHANNEL_INPUTS] = {high, low};

    CIndicatorResult r = donchianchannel_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> lower, r.outputs[1] -> middle, r.outputs[2] -> upper */
    tulip_ffi_result_free(r);
    donchianchannel_state_free(r.state);

    /* Partial + continuation */
    CIndicatorResult p = donchianchannel_indicator(inputs, 8, options, NULL, 0);
    const double *rest_inputs[DONCHIANCHANNEL_INPUTS] = {high + 8, low + 8};
    CBatchResult b = donchianchannel_batch(p.state, rest_inputs, 2, NULL, 0);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    donchianchannel_state_free(p.state);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    outputs, state = tulip_rs.indicators.donchianchannel.indicator([high, low], [14.0])
    print("Lower: ", outputs[0])
    print("Middle:", outputs[1])
    print("Upper: ", outputs[2])

    # State continuation
    outputs2, state = tulip_rs.indicators.donchianchannel.indicator([high[:8], low[:8]], [14.0])
    print("Partial Lower: ", outputs2[0])
    print("Partial Middle:", outputs2[1])
    print("Partial Upper: ", outputs2[2])

    continued = state.batch_indicator([high[8:], low[8:]])
    print("Continued Lower: ", continued[0])
    print("Continued Middle:", continued[1])
    print("Continued Upper: ", continued[2])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low  = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);

    const [outputs, state] = ti.donchianchannel.indicator([high, low], [14]);
    console.log('Donchian Lower:', outputs[0]);
    console.log('Donchian Middle:', outputs[1]);
    console.log('Donchian Upper:', outputs[2]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.donchianchannel.indicator([high.slice(0, n), low.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued Lower:', continued[0]);
    console.log('Continued Middle:', continued[1]);
    console.log('Continued Upper:', continued[2]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.donchianchannel.indicator([high, low], [14]);
    console.log('Donchian Lower:', outputs[0]);
    console.log('Donchian Middle:', outputs[1]);
    console.log('Donchian Upper:', outputs[2]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.donchianchannel.indicator([high.slice(0, n), low.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued Lower:', continued[0]);
    console.log('Continued Middle:', continued[1]);
    console.log('Continued Upper:', continued[2]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::donchianchannel::{DonchianChannel, Indicator};

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let h2 = h1.clone(); let l2 = l1.clone();
    let h3 = h1.clone(); let l3 = l1.clone();
    let h4 = h1.clone(); let l4 = l1.clone();

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];

    let results = DonchianChannel::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Lower:  {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Middle: {:?}", i + 1, asset_outputs[1]);
        println!("Asset {} Upper:  {:?}", i + 1, asset_outputs[2]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::donchianchannel::{DonchianChannel, IndicatorByOptions};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let inputs = [high.as_slice(), low.as_slice()];
    let results = DonchianChannel::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period {} Lower:  {:?}", opts[i][0], opt_outputs[0]);
        println!("Period {} Middle: {:?}", opts[i][0], opt_outputs[1]);
        println!("Period {} Upper:  {:?}", opts[i][0], opt_outputs[2]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel:

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double h2[] = {h1[0]*1.1, h1[1]*1.1, h1[2]*1.1, h1[3]*1.1, h1[4]*1.1,
                   h1[5]*1.1, h1[6]*1.1, h1[7]*1.1, h1[8]*1.1, h1[9]*1.1};
    double l2[] = {l1[0]*1.1, l1[1]*1.1, l1[2]*1.1, l1[3]*1.1, l1[4]*1.1,
                   l1[5]*1.1, l1[6]*1.1, l1[7]*1.1, l1[8]*1.1, l1[9]*1.1};
    double h3[] = {h1[0]*0.9, h1[1]*0.9, h1[2]*0.9, h1[3]*0.9, h1[4]*0.9,
                   h1[5]*0.9, h1[6]*0.9, h1[7]*0.9, h1[8]*0.9, h1[9]*0.9};
    double l3[] = {l1[0]*0.9, l1[1]*0.9, l1[2]*0.9, l1[3]*0.9, l1[4]*0.9,
                   l1[5]*0.9, l1[6]*0.9, l1[7]*0.9, l1[8]*0.9, l1[9]*0.9};
    double h4[] = {h1[0]*1.01, h1[1]*1.01, h1[2]*1.01, h1[3]*1.01, h1[4]*1.01,
                   h1[5]*1.01, h1[6]*1.01, h1[7]*1.01, h1[8]*1.01, h1[9]*1.01};
    double l4[] = {l1[0]*1.01, l1[1]*1.01, l1[2]*1.01, l1[3]*1.01, l1[4]*1.01,
                   l1[5]*1.01, l1[6]*1.01, l1[7]*1.01, l1[8]*1.01, l1[9]*1.01};

    const double *asset1[DONCHIANCHANNEL_INPUTS] = {h1, l1};
    const double *asset2[DONCHIANCHANNEL_INPUTS] = {h2, l2};
    const double *asset3[DONCHIANCHANNEL_INPUTS] = {h3, l3};
    const double *asset4[DONCHIANCHANNEL_INPUTS] = {h4, l4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = donchianchannel_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        donchianchannel_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double high_expanded[40], low_expanded[40];
    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 10; j++) {
            high_expanded[i * 10 + j] = h1[j];
            low_expanded[i * 10 + j] = l1[j];
        }
    }
    const double *expanded_inputs[DONCHIANCHANNEL_INPUTS] = {high_expanded, low_expanded};

    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = donchianchannel_simd_by_options(expanded_inputs, 40, simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        donchianchannel_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    simd_inputs = [
        [high,        low],
        [high + 0.5,  low + 0.5],
        [high - 0.5,  low - 0.5],
        [high * 1.01, low * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.donchianchannel.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Lower:  {out[0]}")
        print(f"Asset {i + 1} Middle: {out[1]}")
        print(f"Asset {i + 1} Upper:  {out[2]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.donchianchannel.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]} Lower:  {out[0]}")
        print(f"Period {simd_options[i][0]} Middle: {out[1]}")
        print(f"Period {simd_options[i][0]} Upper:  {out[2]}")
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
    const [results] = ti.donchianchannel.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => {
        console.log(`Asset ${i + 1} Lower:`, out[0]);
        console.log(`Asset ${i + 1} Middle:`, out[1]);
        console.log(`Asset ${i + 1} Upper:`, out[2]);
    });
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.donchianchannel.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0], out[1], out[2]));
    ```
