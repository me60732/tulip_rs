# APO — Absolute Price Oscillator

The raw difference between two EMAs (short minus long). Positive values indicate upward momentum.

**Inputs:** `[real]` | **Options:** `[short_period, long_period]` | **Outputs:** `[apo]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::apo::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // options: [short_period, long_period]
    let (outputs, mut state) = indicator(&[close.as_slice()], &[12.0, 26.0], None).unwrap();
    println!("{:?}", outputs[0]); // APO values

    // State continuation — feed new bars without reprocessing history
    let new_close = vec![85.10, 85.72_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
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
    use tulip_rs::indicators::apo::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true];
    let (outputs, _state) = indicator(&[close.as_slice()], &[5.0, 20.0], Some(&mask)).unwrap();

    let apo       = &outputs[0]; // APO values (primary)
    let short_ema = &outputs[1]; // short_ema (optional — requested)
    let long_ema  = &outputs[2]; // long_ema (optional — requested)
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

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::apo::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[12.0, 26.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::apo::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[6.0, 13.0], &[12.0, 26.0], &[19.0, 39.0], &[24.0, 52.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
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
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.apo.simdByAssets(simdInputs, [12, 26]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[6, 13], [12, 26], [19, 39], [24, 52]];
    const [results] = ti.apo.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```
