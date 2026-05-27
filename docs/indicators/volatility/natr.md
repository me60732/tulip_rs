# NATR — Normalized Average True Range

ATR expressed as a percentage of the closing price, making it comparable across different price levels.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[natr]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::natr::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // NATR values (as percentage)

    // State continuation — feed new bars without reprocessing history
    let new_high  = vec![85.20_f64];
    let new_low   = vec![84.50_f64];
    let new_close = vec![85.00_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
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

    outputs, state = tulip_rs.indicators.natr.indicator([high, low, close], [14.0])
    print(outputs[0])  # NATR values (as percentage)

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.natr.indicator([high, low, close], [14]);
    console.log('NATR(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.natr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued NATR:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `natr` exposes 2 optional outputs: `"atr"`, `"tr"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::natr::indicator;

    let high  = vec![82.59, 82.06, 83.87, 84.00, 84.61,
                     84.15, 83.84, 84.99, 85.55, 85.36_f64];
    let low   = vec![80.59, 80.06, 81.87, 82.00, 82.61,
                     82.15, 81.84, 82.99, 83.55, 83.36_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true]; // request atr, tr
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let natr = &outputs[0]; // natr (primary)
    let atr  = &outputs[1]; // atr  (optional — requested)
    let tr   = &outputs[2]; // tr   (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.59, 82.06, 83.87, 84.00, 84.61,
                      84.15, 83.84, 84.99, 85.55, 85.36], dtype=np.float64)
    low   = np.array([80.59, 80.06, 81.87, 82.00, 82.61,
                      82.15, 81.84, 82.99, 83.55, 83.36], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.natr.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, True],
    )

    natr = outputs[0]  # natr (primary)
    atr  = outputs[1]  # atr  (optional — requested)
    tr   = outputs[2]  # tr   (optional — requested)
    ```

=== "Node.js"

    `natr` exposes 2 optional outputs: `atr`, `tr`.

    ```javascript
    const [allOut] = ti.natr.indicator([high, low, close], [14], [true, true]);
    const natr = allOut[0]; // primary
    const atr  = allOut[1]; // optional 0: atr
    const tr   = allOut[2]; // optional 1: tr
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::natr::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::natr::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.natr.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.natr.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.natr.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.natr.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
