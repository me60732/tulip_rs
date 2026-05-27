# Trend Indicators

PPO, APO, ADX, ADXR, DM, DI, DX, Aroon, Aroon Oscillator, and PSAR all follow the universal TulipRS calling convention. Refer to the [Calling Convention](../getting_started.md#calling-convention) section for a primer.

---

## PPO — Percentage Price Oscillator

Expresses the MACD as a percentage of the slow EMA, making it comparable across different price levels.

**Inputs:** `[real]` | **Options:** `[short_period, long_period]` | **Outputs:** `[ppo]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::ppo::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // options: [short_period, long_period]
    let (outputs, mut state) = indicator(&[close.as_slice()], &[12.0, 26.0], None).unwrap();
    println!("{:?}", outputs[0]); // PPO values

    // State continuation — feed new bars without reprocessing history
    let new_close = vec![85.10, 85.72_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### Optional Outputs

    `ppo` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::ppo::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true];
    let (outputs, _state) = indicator(&[close.as_slice()], &[5.0, 20.0], Some(&mask)).unwrap();

    let ppo       = &outputs[0]; // PPO values (primary)
    let short_ema = &outputs[1]; // short_ema (optional — requested)
    let long_ema  = &outputs[2]; // long_ema (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::ppo::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[12.0, 26.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::ppo::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[6.0, 13.0], &[12.0, 26.0], &[19.0, 39.0], &[24.0, 52.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.ppo.indicator([close], [12.0, 26.0])
    print(outputs[0])  # PPO values

    # State continuation
    new_close = np.array([85.10, 85.72], dtype=np.float64)
    continued = state.batch_indicator([new_close])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.ppo.indicator(
        [close], [5.0, 20.0],
        optional_outputs=[True, True],
    )

    ppo       = outputs[0]  # PPO values (primary)
    short_ema = outputs[1]  # short_ema (optional — requested)
    long_ema  = outputs[2]  # long_ema (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [np.array(asset1_close, dtype=np.float64)],
        [np.array(asset2_close, dtype=np.float64)],
        [np.array(asset3_close, dtype=np.float64)],
        [np.array(asset4_close, dtype=np.float64)],
    ]
    outputs_list, states = tulip_rs.indicators.ppo.simd_by_assets(simd_inputs, [12.0, 26.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[6.0, 13.0], [12.0, 26.0], [19.0, 39.0], [24.0, 52.0]]
    outputs_list, states = tulip_rs.indicators.ppo.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.ppo.indicator([close], [12, 26]);
    console.log('PPO:', outputs[0]);

    // State continuation
    const [, state2] = ti.ppo.indicator([close.slice(0, -3)], [12, 26]);
    const continued = state2.batchIndicator([close.slice(-3)]);
    console.log('Continued PPO:', continued[0]);
    ```

    ### Optional Outputs

    `ppo` exposes 2 optional outputs: `short_ema`, `long_ema`.

    ```javascript
    const [allOut] = ti.ppo.indicator([close], [12, 26], [true, true]);
    const ppo      = allOut[0]; // primary
    const shortEma = allOut[1]; // optional 0: short_ema
    const longEma  = allOut[2]; // optional 1: long_ema
    ```

    ### SIMD

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.ppo.simdByAssets(simdInputs, [12, 26]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[6, 13], [12, 26], [19, 39], [24, 52]];
    const [results] = ti.ppo.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```

---

## APO — Absolute Price Oscillator

The raw difference between two EMAs (short minus long). Positive values indicate upward momentum.

**Inputs:** `[real]` | **Options:** `[short_period, long_period]` | **Outputs:** `[apo]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::apo::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // options: [short_period, long_period]
    let (outputs, mut state) = indicator(&[close.as_slice()], &[12.0, 26.0], None).unwrap();
    println!("{:?}", outputs[0]); // APO values

    // State continuation — feed new bars without reprocessing history
    let new_close = vec![85.10, 85.72_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### Optional Outputs

    `apo` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::apo::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true];
    let (outputs, _state) = indicator(&[close.as_slice()], &[5.0, 20.0], Some(&mask)).unwrap();

    let apo       = &outputs[0]; // APO values (primary)
    let short_ema = &outputs[1]; // short_ema (optional — requested)
    let long_ema  = &outputs[2]; // long_ema (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::apo::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [
        &[asset1_close.as_slice()],
        &[asset2_close.as_slice()],
        &[asset3_close.as_slice()],
        &[asset4_close.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[12.0, 26.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::apo::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[6.0, 13.0], &[12.0, 26.0], &[19.0, 39.0], &[24.0, 52.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.apo.indicator([close], [12.0, 26.0])
    print(outputs[0])  # APO values

    # State continuation
    new_close = np.array([85.10, 85.72], dtype=np.float64)
    continued = state.batch_indicator([new_close])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.apo.indicator(
        [close], [5.0, 20.0],
        optional_outputs=[True, True],
    )

    apo       = outputs[0]  # APO values (primary)
    short_ema = outputs[1]  # short_ema (optional — requested)
    long_ema  = outputs[2]  # long_ema (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [np.array(asset1_close, dtype=np.float64)],
        [np.array(asset2_close, dtype=np.float64)],
        [np.array(asset3_close, dtype=np.float64)],
        [np.array(asset4_close, dtype=np.float64)],
    ]
    outputs_list, states = tulip_rs.indicators.apo.simd_by_assets(simd_inputs, [12.0, 26.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[6.0, 13.0], [12.0, 26.0], [19.0, 39.0], [24.0, 52.0]]
    outputs_list, states = tulip_rs.indicators.apo.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.apo.indicator([close], [12, 26]);
    console.log('APO:', outputs[0]);

    // State continuation
    const [, state2] = ti.apo.indicator([close.slice(0, -3)], [12, 26]);
    const continued = state2.batchIndicator([close.slice(-3)]);
    console.log('Continued APO:', continued[0]);
    ```

    ### Optional Outputs

    `apo` exposes 2 optional outputs: `short_ema`, `long_ema`.

    ```javascript
    const [allOut] = ti.apo.indicator([close], [12, 26], [true, true]);
    const apo      = allOut[0]; // primary
    const shortEma = allOut[1]; // optional 0: short_ema
    const longEma  = allOut[2]; // optional 1: long_ema
    ```

    ### SIMD

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.apo.simdByAssets(simdInputs, [12, 26]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[6, 13], [12, 26], [19, 39], [24, 52]];
    const [results] = ti.apo.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```

---

## ADX — Average Directional Movement Index

Measures the strength of a trend regardless of direction. Values above 25 indicate a strong trend; below 20 suggest a weak or ranging market.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[adx]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::adx::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // ADX values

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

    ### Optional Outputs

    `adx` exposes 3 optional outputs: `dx`, `atr`, `tr`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::adx::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low   = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();

    let mask = [true, true, false];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let adx = &outputs[0]; // adx (primary)
    let dx  = &outputs[1]; // dx (optional — requested)
    let atr = &outputs[2]; // atr (optional — requested)
    // tr not requested — omitted from outputs
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::adx::indicator_by_assets;

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
    use tulip_rs::indicators::adx::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.adx.indicator([high, low, close], [14.0])
    print(outputs[0])  # ADX values

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high  = close + 1.0
    low   = close - 1.0

    outputs, state = tulip_rs.indicators.adx.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, True, False],
    )

    adx = outputs[0]  # adx (primary)
    dx  = outputs[1]  # dx (optional — requested)
    atr = outputs[2]  # atr (optional — requested)
    # tr not requested — omitted from outputs
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.adx.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.adx.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.adx.indicator([high, low, close], [14]);
    console.log('ADX(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.adx.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued ADX:', continued[0]);
    ```

    ### Optional Outputs

    `adx` exposes 3 optional outputs: `dx`, `atr`, `tr`.

    ```javascript
    const [allOut] = ti.adx.indicator([high, low, close], [14], [true, true, true]);
    const adx = allOut[0]; // primary
    const dx  = allOut[1]; // optional 0: dx
    const atr = allOut[2]; // optional 1: atr
    const tr  = allOut[3]; // optional 2: tr

    // Request only dx
    const [partial] = ti.adx.indicator([high, low, close], [14], [true, false, false]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.adx.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.adx.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## ADXR — Average Directional Movement Index Rating

A smoothed version of ADX, calculated as the average of the current ADX and the ADX from `period` bars ago.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[adxr]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::adxr::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // ADXR values

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

    ### Optional Outputs

    `adxr` exposes 4 optional outputs: `adx`, `dx`, `atr`, `tr`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::adxr::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low   = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();

    let mask = [true, false, false, false];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let adxr = &outputs[0]; // adxr (primary)
    let adx  = &outputs[1]; // adx (optional — requested)
    // dx, atr, tr not requested — omitted from outputs
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::adxr::indicator_by_assets;

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
    use tulip_rs::indicators::adxr::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.adxr.indicator([high, low, close], [14.0])
    print(outputs[0])  # ADXR values

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high  = close + 1.0
    low   = close - 1.0

    outputs, state = tulip_rs.indicators.adxr.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, False, False, False],
    )

    adxr = outputs[0]  # adxr (primary)
    adx  = outputs[1]  # adx (optional — requested)
    # dx, atr, tr not requested — omitted from outputs
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.adxr.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.adxr.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.adxr.indicator([high, low, close], [14]);
    console.log('ADXR(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.adxr.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued ADXR:', continued[0]);
    ```

    ### Optional Outputs

    `adxr` exposes 4 optional outputs: `adx`, `dx`, `atr`, `tr`.

    ```javascript
    const [allOut] = ti.adxr.indicator([high, low, close], [14], [true, true, true, true]);
    const adxr = allOut[0]; // primary
    const adx  = allOut[1]; // optional 0: adx
    const dx   = allOut[2]; // optional 1: dx
    const atr  = allOut[3]; // optional 2: atr
    const tr   = allOut[4]; // optional 3: tr
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.adxr.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.adxr.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## DM — Directional Movement

Raw directional movement values before smoothing. +DM captures upward movement; -DM captures downward movement.

**Inputs:** `[high, low]` | **Options:** `[period]` | **Outputs:** `[plus_dm, minus_dm]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::dm::indicator;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("+DM: {:?}", outputs[0]);
    println!("-DM: {:?}", outputs[1]);

    // State continuation — feed new bars without reprocessing history
    let new_high = vec![85.30_f64];
    let new_low  = vec![84.60_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice()],
        None,
    ).unwrap();
    println!("+DM continued: {:?}", continued[0]);
    println!("-DM continued: {:?}", continued[1]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::dm::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} +DM: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} -DM: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::dm::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {} +DM: {:?}", opts[i][0], out[0]);
        println!("Period {} -DM: {:?}", opts[i][0], out[1]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    outputs, state = tulip_rs.indicators.dm.indicator([high, low], [14.0])
    print(outputs[0])  # Plus DM
    print(outputs[1])  # Minus DM

    # State continuation
    new_high = np.array([85.30], dtype=np.float64)
    new_low  = np.array([84.60], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low])
    print(continued[0])  # Plus DM continued
    print(continued[1])  # Minus DM continued
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1],
        [h2, l2],
        [h3, l3],
        [h4, l4],
    ]
    outputs_list, states = tulip_rs.indicators.dm.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1} +DM: {asset_outputs[0]}")
        print(f"Asset {i+1} -DM: {asset_outputs[1]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.dm.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]} +DM: {out[0]}")
        print(f"Period {simd_options[i][0]} -DM: {out[1]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.dm.indicator([high, low], [14]);
    console.log('+DM:', outputs[0]);
    console.log('-DM:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.dm.indicator([high.slice(0, n), low.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued +DM:', continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.dm.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} +DM:`, out[0], '-DM:', out[1]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.dm.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]} +DM:`, out[0], '-DM:', out[1]));
    ```

---

## DI — Directional Indicator

Smoothed directional movement expressed as a percentage of ATR. +DI and -DI crossovers are used as trade signals.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[plus_di, minus_di]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::di::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("+DI: {:?}", outputs[0]);
    println!("-DI: {:?}", outputs[1]);

    // State continuation — feed new bars without reprocessing history
    let new_high  = vec![85.20_f64];
    let new_low   = vec![84.50_f64];
    let new_close = vec![85.00_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("+DI continued: {:?}", continued[0]);
    println!("-DI continued: {:?}", continued[1]);
    ```

    ### Optional Outputs

    `di` exposes 2 optional outputs: `atr`, `tr`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::di::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low   = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();

    let mask = [true, true];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let plus_di  = &outputs[0]; // plus_di (primary)
    let minus_di = &outputs[1]; // minus_di (primary)
    let atr      = &outputs[2]; // atr (optional — requested)
    let tr       = &outputs[3]; // tr (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::di::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} +DI: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} -DI: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::di::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {} +DI: {:?}", opts[i][0], out[0]);
        println!("Period {} -DI: {:?}", opts[i][0], out[1]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.di.indicator([high, low, close], [14.0])
    print(outputs[0])  # Plus DI
    print(outputs[1])  # Minus DI

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])  # Plus DI continued
    print(continued[1])  # Minus DI continued
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high  = close + 1.0
    low   = close - 1.0

    outputs, state = tulip_rs.indicators.di.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, True],
    )

    plus_di  = outputs[0]  # plus_di (primary)
    minus_di = outputs[1]  # minus_di (primary)
    atr      = outputs[2]  # atr (optional — requested)
    tr       = outputs[3]  # tr (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.di.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1} +DI: {asset_outputs[0]}")
        print(f"Asset {i+1} -DI: {asset_outputs[1]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.di.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]} +DI: {out[0]}")
        print(f"Period {simd_options[i][0]} -DI: {out[1]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.di.indicator([high, low, close], [14]);
    console.log('+DI:', outputs[0]);
    console.log('-DI:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.di.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued +DI:', continued[0]);
    ```

    ### Optional Outputs

    `di` exposes 2 optional outputs: `atr`, `tr`.

    ```javascript
    const [allOut] = ti.di.indicator([high, low, close], [14], [true, true]);
    const plusDI  = allOut[0]; // primary: +di
    const minusDI = allOut[1]; // primary: -di
    const atr     = allOut[2]; // optional 0: atr
    const tr      = allOut[3]; // optional 1: tr
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.di.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} +DI:`, out[0], '-DI:', out[1]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.di.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]} +DI:`, out[0], '-DI:', out[1]));
    ```

---

## DX — Directional Movement Index

The ratio of the difference to the sum of +DI and -DI, expressing directional movement as a single value. ADX is a smoothed DX.

**Inputs:** `[high, low, close]` | **Options:** `[period]` | **Outputs:** `[dx]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::dx::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // DX values

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

    ### Optional Outputs

    `dx` exposes 2 optional outputs: `atr`, `tr`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::dx::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high  = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low   = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();

    let mask = [true, false];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let dx  = &outputs[0]; // dx (primary)
    let atr = &outputs[1]; // atr (optional — requested)
    // tr not requested — omitted from outputs
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::dx::indicator_by_assets;

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
    use tulip_rs::indicators::dx::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.dx.indicator([high, low, close], [14.0])
    print(outputs[0])  # DX values

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high  = close + 1.0
    low   = close - 1.0

    outputs, state = tulip_rs.indicators.dx.indicator(
        [high, low, close], [14.0],
        optional_outputs=[True, False],
    )

    dx  = outputs[0]  # dx (primary)
    atr = outputs[1]  # atr (optional — requested)
    # tr not requested — omitted from outputs
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.dx.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.dx.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.dx.indicator([high, low, close], [14]);
    console.log('DX(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.dx.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued DX:', continued[0]);
    ```

    ### Optional Outputs

    `dx` exposes 2 optional outputs: `atr`, `tr`.

    ```javascript
    const [allOut] = ti.dx.indicator([high, low, close], [14], [true, true]);
    const dx  = allOut[0]; // primary
    const atr = allOut[1]; // optional 0: atr
    const tr  = allOut[2]; // optional 1: tr
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.dx.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.dx.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## Aroon

Measures how recently the highest high and lowest low occurred within the lookback period, as a percentage (0–100).

**Inputs:** `[high, low]` | **Options:** `[period]` | **Outputs:** `[aroon_down, aroon_up]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::aroon::indicator;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[25.0], None).unwrap();
    println!("Aroon Down: {:?}", outputs[0]);
    println!("Aroon Up:   {:?}", outputs[1]);

    // State continuation — feed new bars without reprocessing history
    let new_high = vec![85.30_f64];
    let new_low  = vec![84.60_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice()],
        None,
    ).unwrap();
    println!("Aroon Down continued: {:?}", continued[0]);
    println!("Aroon Up continued:   {:?}", continued[1]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::aroon::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[25.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Aroon Down: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Aroon Up:   {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::aroon::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[25.0], &[50.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {} Aroon Down: {:?}", opts[i][0], out[0]);
        println!("Period {} Aroon Up:   {:?}", opts[i][0], out[1]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    outputs, state = tulip_rs.indicators.aroon.indicator([high, low], [25.0])
    print(outputs[0])  # Aroon Down
    print(outputs[1])  # Aroon Up

    # State continuation
    new_high = np.array([85.30], dtype=np.float64)
    new_low  = np.array([84.60], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low])
    print(continued[0])  # Aroon Down continued
    print(continued[1])  # Aroon Up continued
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1],
        [h2, l2],
        [h3, l3],
        [h4, l4],
    ]
    outputs_list, states = tulip_rs.indicators.aroon.simd_by_assets(simd_inputs, [25.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1} Aroon Down: {asset_outputs[0]}")
        print(f"Asset {i+1} Aroon Up:   {asset_outputs[1]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [25.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.aroon.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]} Aroon Down: {out[0]}")
        print(f"Period {simd_options[i][0]} Aroon Up:   {out[1]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.aroon.indicator([high, low], [25]);
    console.log('Aroon Down:', outputs[0]);
    console.log('Aroon Up:',  outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.aroon.indicator([high.slice(0, n), low.slice(0, n)], [25]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued Aroon Down:', continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.aroon.simdByAssets(simdInputs, [25]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} Down:`, out[0], 'Up:', out[1]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [25], [50]];
    const [results] = ti.aroon.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]} Down:`, out[0], 'Up:', out[1]));
    ```

---

## Aroon Oscillator

The difference between Aroon Up and Aroon Down. Positive values indicate bullish momentum; negative bearish.

**Inputs:** `[high, low]` | **Options:** `[period]` | **Outputs:** `[aroonosc]`

=== "Rust"

    ### Basic

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

    ### Optional Outputs

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

    ### SIMD

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

    ### Basic

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

    ### Optional Outputs

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

    ### SIMD

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

    ### Basic

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

    `aroonosc` exposes 2 optional outputs: `aroon_down`, `aroon_up`.

    ```javascript
    const [allOut] = ti.aroonosc.indicator([high, low], [25], [true, true]);
    const aroonosc  = allOut[0]; // primary
    const aroonDown = allOut[1]; // optional 0: aroon_down
    const aroonUp   = allOut[2]; // optional 1: aroon_up
    ```

    ### SIMD

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

---

## PSAR — Parabolic SAR

A trailing stop-and-reverse indicator. The SAR dot flips below or above price to signal trend direction.

**Inputs:** `[high, low]` | **Options:** `[acceleration_factor_step, acceleration_factor_maximum]` | **Outputs:** `[psar]`

=== "Rust"

    ### Basic

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

    ### SIMD

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

    ### Basic

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

    ### SIMD

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

    ### Basic

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
