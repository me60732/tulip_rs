# EF — Efficiency Ratio

Measures how efficiently price moves in one direction over `period` bars; values near 1 indicate a strong trending market, values near 0 indicate a choppy, sideways market.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[ef]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::ef::{Ef, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let (outputs, mut state) = Ef::indicator(&[close.as_slice()], &[5.0], None).unwrap();
    println!("EF(5): {:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial_close = close[..8].to_vec();
    let (outputs2, mut state) = Ef::indicator(&[partial_close.as_slice()], &[5.0], None).unwrap();
    println!("Partial EF: {:?}", outputs2[0]);

    let new_close = vec![86.54_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued EF: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    outputs, state = tulip_rs.indicators.ef.indicator([close], [5.0])
    print("EF(5):", outputs[0])

    # State continuation
    n = len(close) - 5
    outputs2, state = tulip_rs.indicators.ef.indicator([close[:n]], [5.0])
    print("Partial EF:", outputs2[0])

    continued = state.batch_indicator([close[n:]])
    print("Continued EF:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                                     83.15, 82.84, 83.99, 84.55, 84.36,
                                     85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.ef.indicator([close], [5]);
    console.log('EF(5):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.ef.indicator([close.slice(0, n)], [5]);
    const continued = state2.batchIndicator([close.slice(n)]);
    console.log('Continued EF:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.ef.indicator([close], [5]);
    console.log('EF(5):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.ef.indicator([close.slice(0, n)], [5]);
    const continued = state2.batchIndicator([close.slice(n)]);
    console.log('Continued EF:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ef::{Ef, Indicator};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                  85.53, 86.54, 86.89, 87.77, 87.29_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                  77.10, 77.80, 78.20, 78.90, 79.30_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                  58.90, 59.20, 59.60, 60.00, 60.30_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                  104.2, 104.7, 105.0, 105.5, 106.0_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = Ef::indicator_by_assets::<4>(&inputs, &[5.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::ef::{Ef, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let opts: [&[f64; 1]; 4] = [&[3.0], &[5.0], &[7.0], &[10.0]];

    let results = Ef::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.ef.simd_by_assets(simd_inputs, [5.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    simd_options = [[3.0], [5.0], [7.0], [10.0]]
    outputs_list, states = tulip_rs.indicators.ef.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.ef.simdByAssets(simdInputs, [5]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[3], [5], [7], [10]];
    const [results] = ti.ef.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
