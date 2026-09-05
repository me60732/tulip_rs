# Chandelier Exit

A trailing stop-loss indicator that dynamically adjusts with volatility. The long line sits at the highest high over the look-back period minus a multiple of ATR; the short line sits at the lowest low plus the same multiple. A close crossing below the long line, or above the short line, signals a potential trend reversal or exit.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period, step]` &nbsp;|&nbsp; **Outputs:** `[long, short]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::chandelierexit::{ChandelierExit, Indicator, TIndicatorState};

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
    // Options: [period, step]  — step is the ATR multiplier (default 2)
    let (outputs, mut state) = ChandelierExit::indicator(&inputs, &[14.0, 2.0], None).unwrap();
    println!("{:?}", outputs[0]); // long stop values
    println!("{:?}", outputs[1]); // short stop values

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = ChandelierExit::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[14.0, 2.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high  = vec![86.54_f64];
    let new_low   = vec![85.39_f64];
    let new_close = vec![86.53_f64];
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
                      83.90, 83.33, 84.30, 84.84, 85.00,
                      85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11,
                      84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29], dtype=np.float64)

    outputs, state = tulip_rs.indicators.chandelierexit.indicator(
        [high, low, close], [14.0, 2.0]
    )
    print(outputs[0])  # long stop values
    print(outputs[1])  # short stop values

    # State continuation
    new_high  = np.array([88.50], dtype=np.float64)
    new_low   = np.array([87.30], dtype=np.float64)
    new_close = np.array([88.10], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.chandelierexit.indicator([high, low, close], [14, 2]);
    console.log('Long stop:', outputs[0]);
    console.log('Short stop:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.chandelierexit.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14, 2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued long stop:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.chandelierexit.indicator([high, low, close], [14, 2]);
    console.log('Long stop:', outputs[0]);
    console.log('Short stop:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.chandelierexit.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14, 2]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued long stop:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `chandelierexit` exposes 4 optional outputs: `atr`, `tr`, `min`, `max`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::chandelierexit::{ChandelierExit, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29_f64];

    let mask = [true, true, false, false];
    let (outputs, _state) = ChandelierExit::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0, 2.0],
        Some(&mask),
    ).unwrap();

    let long = &outputs[0]; // long (primary)
    let short = &outputs[1]; // short (primary)
    let atr  = &outputs[2]; // atr (optional — requested)
    let tr   = &outputs[3]; // tr (optional — requested)
    // min and max not requested — omitted from outputs
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

    outputs, state = tulip_rs.indicators.chandelierexit.indicator(
        [high, low, close], [14.0, 2.0],
        optional_outputs=[True, True, False, False],
    )

    long  = outputs[0]  # long (primary)
    short = outputs[1]  # short (primary)
    atr   = outputs[2]  # atr (optional — requested)
    tr    = outputs[3]  # tr (optional — requested)
    # min and max not requested — omitted from outputs
    ```

=== "Node.js"

    `chandelierexit` exposes 4 optional outputs: `atr`, `tr`, `min`, `max`.

    ```javascript
    const [allOut] = ti.chandelierexit.indicator([high, low, close], [14, 2], [true, true, true, true]);
    const long  = allOut[0]; // primary
    const short = allOut[1]; // primary
    const atr   = allOut[2]; // optional 0: atr
    const tr    = allOut[3]; // optional 1: tr
    const min   = allOut[4]; // optional 2: min (rolling lowest low)
    const max   = allOut[5]; // optional 3: max (rolling highest high)

    // Request only atr and tr
    const [partial] = ti.chandelierexit.indicator([high, low, close], [14, 2], [true, true, false, false]);
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.chandelierexit.indicator([high, low, close], [14, 2], [true, true, true, true]);
    const long  = allOut[0]; // primary
    const short = allOut[1]; // primary
    const atr   = allOut[2]; // optional 0: atr
    const tr    = allOut[3]; // optional 1: tr
    const min   = allOut[4]; // optional 2: min (rolling lowest low)
    const max   = allOut[5]; // optional 3: max (rolling highest high)

    // Request only atr and tr
    const [partial] = ti.chandelierexit.indicator([high, low, close], [14, 2], [true, true, false, false]);
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::chandelierexit::{ChandelierExit, Indicator};

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = ChandelierExit::indicator_by_assets::<4>(&inputs, &[14.0, 2.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: long={:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::chandelierexit::{ChandelierExit, IndicatorByOptions};

    let opts: [&[f64; 2]; 4] = [&[10.0, 2.0], &[14.0, 2.0], &[20.0, 2.0], &[30.0, 3.0]];
    let results = ChandelierExit::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period={} step={}: long={:?}", opts[i][0], opts[i][1], out[0]);
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
    outputs_list, states = tulip_rs.indicators.chandelierexit.simd_by_assets(simd_inputs, [14.0, 2.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: long={asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[10.0, 2.0], [14.0, 2.0], [20.0, 2.0], [30.0, 3.0]]
    outputs_list, states = tulip_rs.indicators.chandelierexit.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period={simd_options[i][0]} step={simd_options[i][1]}: long={out[0]}")
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
    const [results] = ti.chandelierexit.simdByAssets(simdInputs, [14, 2]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}: long=`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[10, 2], [14, 2], [20, 2], [30, 3]];
    const [results] = ti.chandelierexit.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period=${simdOptions[i][0]} step=${simdOptions[i][1]}: long=`, out[0]));
    ```
