# PSAR — Parabolic SAR

A trailing stop-and-reverse indicator. The SAR dot flips below or above price to signal trend direction.

**Inputs:** `[high, low]` | **Options:** `[acceleration_factor_step, acceleration_factor_maximum]` | **Outputs:** `[psar]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::psar::indicator;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    // options: [acceleration_factor_step, acceleration_factor_maximum]
    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[0.02, 0.2], None).unwrap();
    println!("{:?}", outputs[0]); // PSAR values

    // State continuation — feed new bars without reprocessing history
    let new_high = vec![85.30_f64];
    let new_low  = vec![84.60_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    # options: [acceleration_factor_step, acceleration_factor_maximum]
    outputs, state = tulip_rs.indicators.psar.indicator([high, low], [0.02, 0.2])
    print(outputs[0])  # PSAR values

    # State continuation
    new_high = np.array([85.30], dtype=np.float64)
    new_low  = np.array([84.60], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.psar.indicator([high, low], [0.02, 0.2]);
    console.log('PSAR:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.psar.indicator([high.slice(0, n), low.slice(0, n)], [0.02, 0.2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued PSAR:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::psar::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[0.02, 0.2], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::psar::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[0.01, 0.1], &[0.02, 0.2], &[0.03, 0.3], &[0.04, 0.4]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Step/Max {}/{}: {:?}", opts[i][0], opts[i][1], out[0]);
    }
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1],
        [h2, l2],
        [h3, l3],
        [h4, l4],
    ]
    outputs_list, states = tulip_rs.indicators.psar.simd_by_assets(simd_inputs, [0.02, 0.2])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[0.01, 0.1], [0.02, 0.2], [0.03, 0.3], [0.04, 0.4]]
    outputs_list, states = tulip_rs.indicators.psar.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Step/Max {simd_options[i][0]}/{simd_options[i][1]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.psar.simdByAssets(simdInputs, [0.02, 0.2]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[0.01, 0.1], [0.02, 0.2], [0.03, 0.3], [0.04, 0.4]];
    const [results] = ti.psar.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Step ${simdOptions[i][0]}:`, out[0]));
    ```
