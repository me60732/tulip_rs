# Super Trend

A trend-following overlay that plots above price in a downtrend and below price in an uptrend; direction flips when price crosses the band. Built on ATR-based dynamic bands, it adapts volatility automatically.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period, step]` &nbsp;|&nbsp; **Outputs:** `[supertrend]` &nbsp;|&nbsp; **Optional:** `[atr, tr, medprice]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::supertrend::{SuperTrend, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                     89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                     91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                     88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                     90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    // options: [period, step] — step is the ATR multiplier
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = SuperTrend::indicator(&inputs, &[10.0, 3.0], None).unwrap();
    println!("Super Trend: {:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = SuperTrend::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[10.0, 3.0], None).unwrap();
    println!("Super Trend: {:?}", outputs2[0]);

    let new_high   = vec![85.90_f64];
    let new_low    = vec![84.03_f64];
    let new_close  = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()], None).unwrap();
    println!("Continued Super Trend: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                      89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                      91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                      88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                      90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    # options: [period, step] — step is the ATR multiplier
    outputs, state = tulip_rs.indicators.supertrend.indicator([high, low, close], [10.0, 3.0])
    print("Super Trend:", outputs[0])

    # State continuation
    n = len(high) - 5
    outputs2, state = tulip_rs.indicators.supertrend.indicator(
        [high[:n], low[:n], close[:n]], [10.0, 3.0]
    )
    print("Partial Super Trend:", outputs2[0])

    continued = state.batch_indicator([high[n:], low[n:], close[n:]])
    print("Continued Super Trend:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                                     85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                                     89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                                     91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                                     84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                                     88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                                     90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20]);

    // options: [period, step] — step is the ATR multiplier
    const [outputs, state] = ti.supertrend.indicator([high, low, close], [10, 3.0]);
    console.log('Super Trend:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.supertrend.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [10, 3.0]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Super Trend:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                   85.90, 86.58, 86.98, 88.00, 87.87, 88.20, 88.70, 89.10, 88.50, 89.00,
                   89.60, 89.90, 89.30, 90.10, 90.50, 91.00, 90.30, 91.00, 91.60, 92.00,
                   91.30, 92.00, 92.60, 93.00, 92.30, 93.00, 93.60, 94.00, 93.30, 94.10];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                   84.03, 85.39, 85.76, 87.17, 87.01, 87.20, 87.80, 88.20, 87.60, 88.00,
                   88.60, 88.90, 88.30, 89.00, 89.40, 89.80, 89.20, 89.90, 90.50, 90.80,
                   90.20, 90.90, 91.50, 91.80, 91.20, 91.90, 92.50, 92.80, 92.20, 93.00];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20];

    const [outputs, state] = ti.supertrend.indicator([high, low, close], [10, 3.0]);
    console.log('Super Trend:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.supertrend.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [10, 3.0]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Super Trend:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `supertrend` exposes 3 optional outputs: `atr`, `tr`, `medprice`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::supertrend::{SuperTrend, Indicator, TIndicatorState};

    // ... (same high, low, close data as above)
    let mask = [true, true, true];
    let (outputs, _state) = SuperTrend::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[10.0, 3.0],
        Some(&mask),
    ).unwrap();

    let supertrend = &outputs[0]; // supertrend (primary)
    let atr        = &outputs[1]; // atr (optional — requested)
    let tr         = &outputs[2]; // tr (optional — requested)
    let medprice   = &outputs[3]; // medprice (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    # ... (same high, low, close data as above)
    outputs, state = tulip_rs.indicators.supertrend.indicator(
        [high, low, close], [10.0, 3.0],
        optional_outputs=[True, True, True],
    )

    supertrend = outputs[0]  # supertrend (primary)
    atr        = outputs[1]  # atr (optional — requested)
    tr         = outputs[2]  # tr (optional — requested)
    medprice   = outputs[3]  # medprice (optional — requested)
    ```

=== "Node.js"

    `supertrend` exposes 3 optional outputs: `atr`, `tr`, `medprice`.

    ```javascript
    const [allOut] = ti.supertrend.indicator([high, low, close], [10, 3.0], [true, true, true]);
    const supertrend = allOut[0]; // primary
    const atr        = allOut[1]; // optional 0: atr
    const tr         = allOut[2]; // optional 1: tr
    const medprice   = allOut[3]; // optional 2: medprice
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.supertrend.indicator([high, low, close], [10, 3.0], [true, true, true]);
    const supertrend = allOut[0]; // primary
    const atr        = allOut[1]; // optional 0: atr
    const tr         = allOut[2]; // optional 1: tr
    const medprice   = allOut[3]; // optional 2: medprice
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::supertrend::indicator_by_assets;

    let h1 = high.clone(); let l1 = low.clone(); let c1 = close.clone();
    let h2 = h1.clone();   let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone();   let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone();   let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[10.0, 3.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::supertrend::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[7.0, 2.0], &[10.0, 3.0], &[14.0, 3.5], &[20.0, 4.0]];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period/Step {}/{}: {:?}", opts[i][0], opts[i][1], out[0]);
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    simd_inputs = [
        [high,        low,        close],
        [high + 0.5,  low + 0.5,  close + 0.5],
        [high - 0.5,  low - 0.5,  close - 0.5],
        [high * 1.01, low * 1.01, close * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.supertrend.simd_by_assets(simd_inputs, [10.0, 3.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    simd_options = [[7.0, 2.0], [10.0, 3.0], [14.0, 3.5], [20.0, 4.0]]
    outputs_list, states = tulip_rs.indicators.supertrend.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period/Step {simd_options[i][0]}/{simd_options[i][1]}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.supertrend.simdByAssets(simdInputs, [10, 3.0]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[7, 2.0], [10, 3.0], [14, 3.5], [20, 4.0]];
    const [results] = ti.supertrend.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period/Step ${simdOptions[i][0]}/${simdOptions[i][1]}:`, out[0]));
    ```
