# Ichimoku — Ichimoku Kinkō Hyō

A comprehensive trend-following system that defines support/resistance, trend direction, and momentum in a single glance; the conversion and base lines act like short- and long-period midpoint averages, while the leading spans project a "cloud" forward by `long_period` bars.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[short_period, long_period]` &nbsp;|&nbsp; **Outputs:** `[conversion, base, leading_span_a, leading_span_b]` &nbsp;|&nbsp; **Optional:** `[lagging_span]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::ichimoku::indicator;

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

    // options: [short_period, long_period]
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[9.0, 26.0], None).unwrap();
    println!("Conversion:     {:?}", outputs[0]);
    println!("Base:           {:?}", outputs[1]);
    println!("Leading Span A: {:?}", outputs[2]);
    println!("Leading Span B: {:?}", outputs[3]);

    // State continuation
    let n = high.len() - 5;
    let partial_inputs = [&high[..n], &low[..n], &close[..n]];
    let (outputs2, mut state) = indicator(&partial_inputs, &[9.0, 26.0], None).unwrap();
    println!("Partial Conversion: {:?}", outputs2[0]);

    let rest_inputs = [&high[n..], &low[n..], &close[n..]];
    let continued = state.batch_indicator(&rest_inputs, None).unwrap();
    println!("Continued Conversion: {:?}", continued[0]);
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

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.ichimoku.indicator([high, low, close], [9.0, 26.0])
    print("Conversion:    ", outputs[0])
    print("Base:          ", outputs[1])
    print("Leading Span A:", outputs[2])
    print("Leading Span B:", outputs[3])

    # State continuation
    n = len(high) - 5
    outputs2, state = tulip_rs.indicators.ichimoku.indicator(
        [high[:n], low[:n], close[:n]], [9.0, 26.0]
    )
    print("Partial Conversion:", outputs2[0])

    continued = state.batch_indicator([high[n:], low[n:], close[n:]])
    print("Continued Conversion:", continued[0])
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

    // options: [short_period, long_period]
    const [outputs, state] = ti.ichimoku.indicator([high, low, close], [9, 26]);
    console.log('Conversion:    ', outputs[0]);
    console.log('Base:          ', outputs[1]);
    console.log('Leading Span A:', outputs[2]);
    console.log('Leading Span B:', outputs[3]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.ichimoku.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [9, 26]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Conversion:', continued[0]);
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

    const [outputs, state] = ti.ichimoku.indicator([high, low, close], [9, 26]);
    console.log('Conversion:    ', outputs[0]);
    console.log('Base:          ', outputs[1]);
    console.log('Leading Span A:', outputs[2]);
    console.log('Leading Span B:', outputs[3]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.ichimoku.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [9, 26]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Conversion:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `ichimoku` exposes 1 optional output: `lagging_span`. The lagging span is the close price shifted back by `long_period` bars, useful for confirming trend signals against historical price. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::ichimoku::indicator;

    // ... (same high, low, close data as above)
    let mask = [true];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice()],
        &[9.0, 26.0],
        Some(&mask),
    ).unwrap();

    let conversion   = &outputs[0]; // conversion (primary)
    let base         = &outputs[1]; // base (primary)
    let leading_a    = &outputs[2]; // leading_span_a (primary)
    let leading_b    = &outputs[3]; // leading_span_b (primary)
    let lagging_span = &outputs[4]; // lagging_span (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    # ... (same high, low, close data as above)
    outputs, state = tulip_rs.indicators.ichimoku.indicator(
        [high, low, close], [9.0, 26.0],
        optional_outputs=[True],
    )

    conversion   = outputs[0]  # conversion (primary)
    base         = outputs[1]  # base (primary)
    leading_a    = outputs[2]  # leading_span_a (primary)
    leading_b    = outputs[3]  # leading_span_b (primary)
    lagging_span = outputs[4]  # lagging_span (optional — requested)
    ```

=== "Node.js"

    `ichimoku` exposes 1 optional output: `lagging_span`.

    ```javascript
    const [allOut] = ti.ichimoku.indicator([high, low, close], [9, 26], [true]);
    const conversion  = allOut[0]; // primary
    const base        = allOut[1]; // primary
    const leadingA    = allOut[2]; // primary
    const leadingB    = allOut[3]; // primary
    const laggingSpan = allOut[4]; // optional 0: lagging_span
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.ichimoku.indicator([high, low, close], [9, 26], [true]);
    const conversion  = allOut[0]; // primary
    const base        = allOut[1]; // primary
    const leadingA    = allOut[2]; // primary
    const leadingB    = allOut[3]; // primary
    const laggingSpan = allOut[4]; // optional 0: lagging_span
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ichimoku::indicator_by_assets;

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

    let results = indicator_by_assets::<4>(&inputs, &[9.0, 26.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} Conversion: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Base:       {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::ichimoku::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[7.0, 22.0], &[9.0, 26.0], &[11.0, 30.0], &[13.0, 34.0]];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.0.iter().enumerate() {
        println!("Short/Long {}/{}: Conversion={:?}", opts[i][0], opts[i][1], out[0]);
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
    outputs_list, states = tulip_rs.indicators.ichimoku.simd_by_assets(simd_inputs, [9.0, 26.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Conversion: {out[0]}")
        print(f"Asset {i + 1} Base:       {out[1]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    simd_options = [[7.0, 22.0], [9.0, 26.0], [11.0, 30.0], [13.0, 34.0]]
    outputs_list, states = tulip_rs.indicators.ichimoku.simd_by_options(
        [high, low, close], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Short/Long {simd_options[i][0]}/{simd_options[i][1]}: {out[0]}")
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
    const [results] = ti.ichimoku.simdByAssets(simdInputs, [9, 26]);
    results.forEach((out, i) => {
        console.log(`Asset ${i + 1} Conversion:`, out[0]);
        console.log(`Asset ${i + 1} Base:      `, out[1]);
    });
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[7, 22], [9, 26], [11, 30], [13, 34]];
    const [results] = ti.ichimoku.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Short/Long ${simdOptions[i][0]}/${simdOptions[i][1]}:`, out[0]));
    ```
