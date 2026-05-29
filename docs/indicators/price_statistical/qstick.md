# QStick — `qstick`

A moving average of `(Close - Open)` over `period` bars, summarising buying or selling pressure.

**Inputs:** `[open, close]` | **Options:** `[period]` | **Outputs:** `[qstick]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::qstick::indicator;

    let open_ = [81.85_f64, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25];
    let close = [81.59_f64, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36];

    let inputs = [open_.as_slice(), close.as_slice()];
    let (outputs, _) = indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    open_  = np.array([81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.qstick.indicator([open_, close], [14.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.qstick.indicator([open_, close], [14]);
    console.log('QStick(14):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.qstick.indicator([open_.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([open_.slice(n), close.slice(n)]);
    console.log('Continued QStick:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.qstick.indicator([open_, close], [14]);
    console.log('QStick(14):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.qstick.indicator([open_.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([open_.slice(n), close.slice(n)]);
    console.log('Continued QStick:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::qstick::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[o1.as_slice(), c1.as_slice()],
        &[o2.as_slice(), c2.as_slice()],
        &[o3.as_slice(), c3.as_slice()],
        &[o4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::qstick::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let results = indicator_by_options::<4>(&inputs_single, &opts, None).unwrap();
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[o1, c1], [o2, c2], [o3, c3], [o4, c4]]
    outputs_list, states = tulip_rs.indicators.qstick.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.qstick.simd_by_options([open_, close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...open_], [...close]],
        [open_.map(v => v * 1.1), close.map(v => v * 1.1)],
        [open_.map(v => v * 0.9), close.map(v => v * 0.9)],
        [open_.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.qstick.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.qstick.simdByOptions([open_, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
