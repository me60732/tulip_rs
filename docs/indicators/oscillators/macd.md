# MACD — Moving Average Convergence Divergence

Shows the relationship between two EMAs of different periods. The histogram visualises the difference between the MACD line and its signal line, highlighting momentum shifts.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[fast_period, slow_period, signal_period]` &nbsp;|&nbsp; **Outputs:** `[macd, signal, histogram]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::macd::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Options: [fast_period, slow_period, signal_period]
    let (outputs, _state) = indicator(&[close.as_slice()], &[12.0, 26.0, 9.0], None).unwrap();
    println!("MACD line:  {:?}", outputs[0]);
    println!("Signal:     {:?}", outputs[1]);
    println!("Histogram:  {:?}", outputs[2]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[12.0, 26.0, 9.0], None).unwrap();
    println!("Partial MACD: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued MACD:      {:?}", continued[0]);
    println!("Continued Signal:    {:?}", continued[1]);
    println!("Continued Histogram: {:?}", continued[2]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Options: [fast_period, slow_period, signal_period]
    outputs, state = tulip_rs.indicators.macd.indicator([close], [12.0, 26.0, 9.0])
    print("MACD line: ", outputs[0])
    print("Signal:    ", outputs[1])
    print("Histogram: ", outputs[2])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.macd.indicator([partial], [12.0, 26.0, 9.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued MACD:      ", continued[0])
    print("Continued Signal:    ", continued[1])
    print("Continued Histogram: ", continued[2])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.macd.indicator([close], [12, 26, 9]);
    console.log('MACD line:', outputs[0]);
    console.log('Signal:',    outputs[1]);
    console.log('Histogram:', outputs[2]);

    // State continuation
    const [, state2] = ti.macd.indicator([close.slice(0, -1)], [12, 26, 9]);
    const continued = state2.batchIndicator([close.slice(-1)]);
    console.log('Continued MACD:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.macd.indicator([close], [12, 26, 9]);
    console.log('MACD line:', outputs[0]);
    console.log('Signal:',    outputs[1]);
    console.log('Histogram:', outputs[2]);

    // State continuation
    const [, state2] = ti.macd.indicator([close.slice(0, -1)], [12, 26, 9]);
    const continued = state2.batchIndicator([close.slice(-1)]);
    console.log('Continued MACD:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `macd` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::macd::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[12.0, 26.0, 9.0], Some(&mask)).unwrap();

    let macd_line   = &outputs[0]; // macd_line (primary)
    let signal_line = &outputs[1]; // signal_line (primary)
    let histogram   = &outputs[2]; // histogram (primary)
    let short_ema   = &outputs[3]; // short_ema (optional — requested)
    let long_ema    = &outputs[4]; // long_ema (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.macd.indicator(
        [close], [12.0, 26.0, 9.0],
        optional_outputs=[True, True],
    )

    macd_line   = outputs[0]  # macd_line (primary)
    signal_line = outputs[1]  # signal_line (primary)
    histogram   = outputs[2]  # histogram (primary)
    short_ema   = outputs[3]  # short_ema (optional — requested)
    long_ema    = outputs[4]  # long_ema (optional — requested)
    ```

=== "Node.js"

    `macd` exposes 2 optional outputs: `short_ema`, `long_ema`.

    ```javascript
    const [allOut] = ti.macd.indicator([close], [12, 26, 9], [true, true]);
    const macdLine = allOut[0]; // primary
    const signal   = allOut[1]; // primary
    const hist     = allOut[2]; // primary
    const shortEma = allOut[3]; // optional 0: short_ema
    const longEma  = allOut[4]; // optional 1: long_ema
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::macd::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[12.0, 26.0, 9.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} MACD: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Signal: {:?}", i + 1, asset_outputs[1]);
        println!("Asset {} Histogram: {:?}", i + 1, asset_outputs[2]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::macd::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[6.0,  13.0,  5.0],
        &[12.0, 26.0,  9.0],
        &[19.0, 39.0, 14.0],
        &[24.0, 52.0, 18.0],
    ];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} MACD:      {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Signal:    {:?}", i + 1, opt_outputs[1]);
        println!("Option set {} Histogram: {:?}", i + 1, opt_outputs[2]);
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.macd.simd_by_assets(simd_inputs, [12.0, 26.0, 9.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} MACD:      {out[0]}")
        print(f"Asset {i + 1} Signal:    {out[1]}")
        print(f"Asset {i + 1} Histogram: {out[2]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [6.0,  13.0,  5.0],
        [12.0, 26.0,  9.0],
        [19.0, 39.0, 14.0],
        [24.0, 52.0, 18.0],
    ]
    outputs_list, states = tulip_rs.indicators.macd.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} MACD:      {out[0]}")
        print(f"Option set {i + 1} Signal:    {out[1]}")
        print(f"Option set {i + 1} Histogram: {out[2]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...close]],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.macd.simdByAssets(simdInputs, [12, 26, 9]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} MACD:`, out[0], 'Signal:', out[1]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[6, 13, 5], [12, 26, 9], [19, 39, 14], [24, 52, 18]];
    const [results] = ti.macd.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} MACD:`, out[0]));
    ```
