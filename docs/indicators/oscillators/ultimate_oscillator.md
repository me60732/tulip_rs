# Ultimate Oscillator

Combines momentum from three different time periods (short, medium, and long) to reduce the false signals that arise from using any single timeframe alone.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[short_period, medium_period, long_period]` &nbsp;|&nbsp; **Outputs:** `[ultosc]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::ultosc::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Options: [short_period, medium_period, long_period]
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[7.0, 14.0, 28.0], None).unwrap();
    println!("Ultimate Oscillator: {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = indicator(&inputs2, &[7.0, 14.0, 28.0], None).unwrap();
    println!("Partial Ultimate Oscillator: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued Ultimate Oscillator: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Options: [short_period, medium_period, long_period]
    outputs, state = tulip_rs.indicators.ultosc.indicator([high, low, close], [7.0, 14.0, 28.0])
    print("Ultimate Oscillator:", outputs[0])

    # State continuation
    outputs2, state = tulip_rs.indicators.ultosc.indicator([high[:8], low[:8], close[:8]], [7.0, 14.0, 28.0])
    continued = state.batch_indicator([high[8:], low[8:], close[8:]])
    print("Continued Ultimate Oscillator:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.ultosc.indicator([high, low, close], [7, 14, 28]);
    console.log('UltOsc:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.ultosc.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [7, 14, 28]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued UltOsc:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ultosc::indicator_by_assets;

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[7.0, 14.0, 28.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::ultosc::indicator_by_options;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[7.0,  14.0, 28.0],
        &[5.0,  10.0, 20.0],
        &[10.0, 20.0, 40.0],
        &[4.0,  8.0,  16.0],
    ];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [
        [high,        low,        close],
        [high + 0.5,  low + 0.5,  close + 0.5],
        [high - 0.5,  low - 0.5,  close - 0.5],
        [high * 1.01, low * 1.01, close * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.ultosc.simd_by_assets(simd_inputs, [7.0, 14.0, 28.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [7.0,  14.0, 28.0],
        [5.0,  10.0, 20.0],
        [10.0, 20.0, 40.0],
        [4.0,  8.0,  16.0],
    ]
    outputs_list, states = tulip_rs.indicators.ultosc.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.ultosc.simdByAssets(simdInputs, [7, 14, 28]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[7, 14, 28], [5, 10, 20], [10, 20, 40], [4, 8, 16]];
    const [results] = ti.ultosc.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```
