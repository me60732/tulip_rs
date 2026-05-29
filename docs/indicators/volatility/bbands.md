# BBands — Bollinger Bands

Three bands plotted around a moving average. The width expands and contracts with volatility.

**Inputs:** `[real]` | **Options:** `[period, stddev_multiplier]` | **Outputs:** `[lower, middle, upper]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::bbands::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // options: [period, stddev_multiplier]
    let (outputs, mut state) = indicator(&[close.as_slice()], &[20.0, 2.0], None).unwrap();
    println!("Lower:  {:?}", outputs[0]);
    println!("Middle: {:?}", outputs[1]);
    println!("Upper:  {:?}", outputs[2]);

    // State continuation — feed new bars without reprocessing history
    let new_close = vec![85.10, 85.72_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Lower continued:  {:?}", continued[0]);
    println!("Middle continued: {:?}", continued[1]);
    println!("Upper continued:  {:?}", continued[2]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # options: [period, stddev_multiplier]
    outputs, state = tulip_rs.indicators.bbands.indicator([close], [20.0, 2.0])
    print(outputs[0])  # Lower band
    print(outputs[1])  # Middle band
    print(outputs[2])  # Upper band

    # State continuation
    new_close = np.array([85.10, 85.72], dtype=np.float64)
    continued = state.batch_indicator([new_close])
    print(continued[0])  # Lower continued
    print(continued[1])  # Middle continued
    print(continued[2])  # Upper continued
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.bbands.indicator([close], [20, 2]);
    console.log('Lower:', outputs[0]);
    console.log('Middle:', outputs[1]);
    console.log('Upper:', outputs[2]);

    // State continuation
    const [, state2] = ti.bbands.indicator([close.slice(0, -5)], [20, 2]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Lower:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.bbands.indicator([close], [20, 2]);
    console.log('Lower:', outputs[0]);
    console.log('Middle:', outputs[1]);
    console.log('Upper:', outputs[2]);

    // State continuation
    const [, state2] = ti.bbands.indicator([close.slice(0, -5)], [20, 2]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Lower:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::bbands::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[20.0, 2.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Lower:  {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Middle: {:?}", i + 1, asset_outputs[1]);
        println!("Asset {} Upper:  {:?}", i + 1, asset_outputs[2]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::bbands::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[10.0, 1.5], &[20.0, 2.0], &[30.0, 2.0], &[50.0, 2.5]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {} Lower:  {:?}", i + 1, out[0]);
        println!("Option set {} Middle: {:?}", i + 1, out[1]);
        println!("Option set {} Upper:  {:?}", i + 1, out[2]);
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
    outputs_list, states = tulip_rs.indicators.bbands.simd_by_assets(simd_inputs, [20.0, 2.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1} Lower:  {asset_outputs[0]}")
        print(f"Asset {i+1} Middle: {asset_outputs[1]}")
        print(f"Asset {i+1} Upper:  {asset_outputs[2]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[10.0, 1.5], [20.0, 2.0], [30.0, 2.0], [50.0, 2.5]]
    outputs_list, states = tulip_rs.indicators.bbands.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1} Lower:  {out[0]}")
        print(f"Option set {i+1} Middle: {out[1]}")
        print(f"Option set {i+1} Upper:  {out[2]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.bbands.simdByAssets(simdInputs, [20, 2]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} Lower:`, out[0], 'Middle:', out[1], 'Upper:', out[2]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[10, 1.5], [20, 2], [30, 2], [50, 2.5]];
    const [results] = ti.bbands.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} Lower:`, out[0]));
    ```
