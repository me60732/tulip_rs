# Aroon Oscillator

The difference between Aroon Up and Aroon Down. Positive values indicate bullish momentum; negative bearish.

**Inputs:** `[high, low]` | **Options:** `[period]` | **Outputs:** `[aroonosc]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::aroonosc::indicator;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[25.0], None).unwrap();
    println!("{:?}", outputs[0]); // Aroon Oscillator values

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

    outputs, state = tulip_rs.indicators.aroonosc.indicator([high, low], [25.0])
    print(outputs[0])  # Aroon Oscillator values

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

    const [outputs, state] = ti.aroonosc.indicator([high, low], [25]);
    console.log('Aroon Oscillator:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.aroonosc.indicator([high.slice(0, n), low.slice(0, n)], [25]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued AroonOsc:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `aroonosc` exposes 2 optional outputs: `aroon_down`, `aroon_up`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::aroonosc::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low   = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();

    let mask = [true, true];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice()],
        &[25.0],
        Some(&mask),
    ).unwrap();

    let aroonosc  = &outputs[0]; // aroonosc (primary)
    let aroon_down = &outputs[1]; // aroon_down (optional — requested)
    let aroon_up   = &outputs[2]; // aroon_up (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high  = close + 1.0
    low   = close - 1.0

    outputs, state = tulip_rs.indicators.aroonosc.indicator(
        [high, low], [25.0],
        optional_outputs=[True, True],
    )

    aroonosc   = outputs[0]  # aroonosc (primary)
    aroon_down = outputs[1]  # aroon_down (optional — requested)
    aroon_up   = outputs[2]  # aroon_up (optional — requested)
    ```

=== "Node.js"

    `aroonosc` exposes 2 optional outputs: `aroon_down`, `aroon_up`.

    ```javascript
    const [allOut] = ti.aroonosc.indicator([high, low], [25], [true, true]);
    const aroonosc  = allOut[0]; // primary
    const aroonDown = allOut[1]; // optional 0: aroon_down
    const aroonUp   = allOut[2]; // optional 1: aroon_up
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::aroonosc::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[25.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::aroonosc::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[25.0], &[50.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
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
    outputs_list, states = tulip_rs.indicators.aroonosc.simd_by_assets(simd_inputs, [25.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [25.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.aroonosc.simd_by_options(
        [high, low], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.aroonosc.simdByAssets(simdInputs, [25]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [25], [50]];
    const [results] = ti.aroonosc.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
