# StdDev — Standard Deviation

Rolling standard deviation of the price series over `period` bars.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[stddev]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::stddev::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, mut state) = indicator(&[close.as_slice()], &[20.0], None).unwrap();
    println!("{:?}", outputs[0]); // StdDev values

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

    outputs, state = tulip_rs.indicators.stddev.indicator([close], [20.0])
    print(outputs[0])  # StdDev values

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

    const [outputs, state] = ti.stddev.indicator([close], [20]);
    console.log('StdDev(20):', outputs[0]);

    // State continuation
    const [, state2] = ti.stddev.indicator([close.slice(0, -5)], [20]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued StdDev:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.stddev.indicator([close], [20]);
    console.log('StdDev(20):', outputs[0]);

    // State continuation
    const [, state2] = ti.stddev.indicator([close.slice(0, -5)], [20]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued StdDev:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `stddev` exposes 1 optional output: `"sma"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::stddev::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // request sma
    let (outputs, _state) = indicator(&[close.as_slice()], &[5.0], Some(&mask)).unwrap();

    let stddev = &outputs[0]; // stddev (primary)
    let sma    = &outputs[1]; // sma    (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.stddev.indicator(
        [close], [5.0],
        optional_outputs=[True],
    )

    stddev = outputs[0]  # stddev (primary)
    sma    = outputs[1]  # sma    (optional — requested)
    ```

=== "Node.js"

    `stddev` exposes 1 optional output: `sma`.

    ```javascript
    const [allOut] = ti.stddev.indicator([close], [20], [true]);
    const stddev = allOut[0]; // primary
    const sma    = allOut[1]; // optional 0: sma
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::stddev::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[20.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::stddev::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[10.0], &[20.0], &[30.0], &[50.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
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
    outputs_list, states = tulip_rs.indicators.stddev.simd_by_assets(simd_inputs, [20.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[10.0], [20.0], [30.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.stddev.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.stddev.simdByAssets(simdInputs, [20]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[10], [20], [30], [50]];
    const [results] = ti.stddev.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
