# Keltner Channel

A volatility-based envelope centred on an EMA of close. The middle band is EMA(close, period); the upper band adds a multiple of Wilder ATR(period) and the lower band subtracts it. Rendered as a price overlay, the channel expands during volatile periods and contracts during quiet ones. Optionally emits the underlying ATR and raw TR series.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period, step]` &nbsp;|&nbsp; **Outputs:** `[lower, middle, upper]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::keltnerchannel::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[14.0, 2.0], None).unwrap();
    println!("{:?}", outputs[0]); // lower band
    println!("{:?}", outputs[1]); // middle band (EMA)
    println!("{:?}", outputs[2]); // upper band

    // State continuation — feed new bars without reprocessing history
    let new_high  = vec![88.50_f64];
    let new_low   = vec![87.30_f64];
    let new_close = vec![88.10_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[1]); // continued middle band
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    outputs, state = tulip_rs.indicators.keltnerchannel.indicator(
        [high, low, close], [14.0, 2.0]
    )
    print(outputs[0])  # lower band
    print(outputs[1])  # middle band (EMA)
    print(outputs[2])  # upper band

    # State continuation
    new_high  = np.array([88.50], dtype=np.float64)
    new_low   = np.array([87.30], dtype=np.float64)
    new_close = np.array([88.10], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[1])  # continued middle band
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.keltnerchannel.indicator([high, low, close], [14, 2]);
    console.log('Lower:', outputs[0]);
    console.log('Middle (EMA):', outputs[1]);
    console.log('Upper:', outputs[2]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.keltnerchannel.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14, 2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued middle:', continued[1]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.keltnerchannel.indicator([high, low, close], [14, 2]);
    console.log('Lower:', outputs[0]);
    console.log('Middle (EMA):', outputs[1]);
    console.log('Upper:', outputs[2]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.keltnerchannel.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14, 2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued middle:', continued[1]);
    ```

### Optional Outputs

=== "Rust"

    `keltnerchannel` exposes 2 optional outputs: `atr`, `tr`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::keltnerchannel::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let mask = [true, false];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0, 2.0],
        Some(&mask),
    ).unwrap();

    let lower  = &outputs[0]; // lower (primary)
    let middle = &outputs[1]; // middle (primary)
    let upper  = &outputs[2]; // upper (primary)
    let atr    = &outputs[3]; // atr (optional — requested)
    // tr not requested — omitted from outputs
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    outputs, state = tulip_rs.indicators.keltnerchannel.indicator(
        [high, low, close], [14.0, 2.0],
        optional_outputs=[True, False],
    )

    lower  = outputs[0]  # lower (primary)
    middle = outputs[1]  # middle (primary)
    upper  = outputs[2]  # upper (primary)
    atr    = outputs[3]  # atr (optional — requested)
    # tr not requested — omitted from outputs
    ```

=== "Node.js"

    `keltnerchannel` exposes 2 optional outputs: `atr`, `tr`.

    ```javascript
    const [allOut] = ti.keltnerchannel.indicator([high, low, close], [14, 2], [true, true]);
    const lower  = allOut[0]; // primary
    const middle = allOut[1]; // primary
    const upper  = allOut[2]; // primary
    const atr    = allOut[3]; // optional 0: atr
    const tr     = allOut[4]; // optional 1: tr

    // Request only atr
    const [partial] = ti.keltnerchannel.indicator([high, low, close], [14, 2], [true, false]);
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.keltnerchannel.indicator([high, low, close], [14, 2], [true, true]);
    const lower  = allOut[0]; // primary
    const middle = allOut[1]; // primary
    const upper  = allOut[2]; // primary
    const atr    = allOut[3]; // optional 0: atr
    const tr     = allOut[4]; // optional 1: tr

    // Request only atr
    const [partial] = ti.keltnerchannel.indicator([high, low, close], [14, 2], [true, false]);
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::keltnerchannel::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[14.0, 2.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: middle={:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::keltnerchannel::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[10.0, 1.5], &[14.0, 2.0], &[20.0, 2.0], &[30.0, 2.5]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period={} step={}: middle={:?}", opts[i][0], opts[i][1], out[1]);
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
    outputs_list, states = tulip_rs.indicators.keltnerchannel.simd_by_assets(simd_inputs, [14.0, 2.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: middle={asset_outputs[1]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[10.0, 1.5], [14.0, 2.0], [20.0, 2.0], [30.0, 2.5]]
    outputs_list, states = tulip_rs.indicators.keltnerchannel.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period={simd_options[i][0]} step={simd_options[i][1]}: middle={out[1]}")
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
    const [results] = ti.keltnerchannel.simdByAssets(simdInputs, [14, 2]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}: middle=`, out[1]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[10, 1.5], [14, 2], [20, 2], [30, 2.5]];
    const [results] = ti.keltnerchannel.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period=${simdOptions[i][0]} step=${simdOptions[i][1]}: middle=`, out[1]));
    ```
