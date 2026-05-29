# VIDYA — Variable Index Dynamic Average

Similar to KAMA but uses the Chande Momentum Oscillator as its efficiency measure. The three options control the short and long CMO periods and the base smoothing constant alpha.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[short_period, long_period, alpha]` &nbsp;|&nbsp; **Outputs:** `[vidya]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::vidya::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Options: [short_period, long_period, alpha]
    let (outputs, _state) = indicator(&[close.as_slice()], &[2.0, 5.0, 0.2], None).unwrap();
    println!("VIDYA: {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[2.0, 5.0, 0.2], None).unwrap();
    println!("Partial VIDYA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued VIDYA: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Options: [short_period, long_period, alpha]
    outputs, state = tulip_rs.indicators.vidya.indicator([close], [2.0, 5.0, 0.2])
    print("VIDYA:", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.vidya.indicator([partial], [2.0, 5.0, 0.2])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued VIDYA:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.vidya.indicator([close], [2, 5, 0.2]);
    console.log('VIDYA:', outputs[0]);

    // State continuation
    const [, state2] = ti.vidya.indicator([close.slice(0, -5)], [2, 5, 0.2]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued VIDYA:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.vidya.indicator([close], [2, 5, 0.2]);
    console.log('VIDYA:', outputs[0]);

    // State continuation
    const [, state2] = ti.vidya.indicator([close.slice(0, -5)], [2, 5, 0.2]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued VIDYA:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `vidya` exposes 4 optional outputs: `"short_sma"`, `"long_sma"`, `"short_sdtdev"`, `"long_sdtdev"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::vidya::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Request short_sma and long_sma but not the stddev outputs
    let mask = [true, true, false, false]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[5.0, 20.0, 0.2], Some(&mask)).unwrap();

    let vidya     = &outputs[0]; // vidya (primary)
    let short_sma = &outputs[1]; // "short_sma" (optional — requested)
    let long_sma  = &outputs[2]; // "long_sma" (optional — requested)
                                 // "short_sdtdev" not requested
                                 // "long_sdtdev" not requested
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Request short_sma and long_sma but not the stddev outputs
    outputs, state = tulip_rs.indicators.vidya.indicator(
        [close], [5.0, 20.0, 0.2],
        optional_outputs=[True, True, False, False],
    )

    vidya     = outputs[0]  # vidya (primary)
    short_sma = outputs[1]  # "short_sma" (optional — requested)
    long_sma  = outputs[2]  # "long_sma" (optional — requested)
                             # "short_sdtdev" not requested
                             # "long_sdtdev" not requested
    ```

=== "Node.js"

    `vidya` exposes 4 optional outputs: `short_sma`, `long_sma`, `short_stddev`, `long_stddev`.

    ```javascript
    // Request all optional outputs
    const [allOut] = ti.vidya.indicator([close], [2, 5, 0.2], [true, true, true, true]);
    const vidya       = allOut[0]; // primary
    const shortSma    = allOut[1]; // optional 0: short_sma
    const longSma     = allOut[2]; // optional 1: long_sma
    const shortStddev = allOut[3]; // optional 2: short_stddev
    const longStddev  = allOut[4]; // optional 3: long_stddev

    // Request only the SMAs
    const [partial] = ti.vidya.indicator([close], [2, 5, 0.2], [true, true, false, false]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::vidya::indicator_by_assets;

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

    let results = indicator_by_assets::<4>(&inputs, &[2.0, 5.0, 0.2], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::vidya::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[2.0, 5.0, 0.2],
        &[3.0, 7.0, 0.3],
        &[4.0, 9.0, 0.4],
        &[5.0, 11.0, 0.5],
    ];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.vidya.simd_by_assets(simd_inputs, [2.0, 5.0, 0.2])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [2.0, 5.0, 0.2],
        [3.0, 7.0, 0.3],
        [4.0, 9.0, 0.4],
        [5.0, 11.0, 0.5],
    ]
    outputs_list, states = tulip_rs.indicators.vidya.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...close]],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.vidya.simdByAssets(simdInputs, [2, 5, 0.2]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[2, 5, 0.2], [3, 7, 0.3], [4, 9, 0.4], [5, 11, 0.5]];
    const [results] = ti.vidya.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```
