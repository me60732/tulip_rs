# ADAPTIVEMSW — Adaptive MESA Sine Wave

Automatically adapts the Mesa Sine Wave to the dominant cycle period without requiring a fixed lookback parameter.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[]` (none) &nbsp;|&nbsp; **Outputs:** `[sine, lead_sine]` &nbsp;|&nbsp; **Optional:** `[dc_period]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::adaptivemsw::{AdaptiveMSW, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    // adaptivemsw takes no options — pass an empty slice
    let (outputs, _state) = AdaptiveMSW::indicator(&[close.as_slice()], &[], None).unwrap();
    println!("Sine:      {:?}", outputs[0]);
    println!("Lead Sine: {:?}", outputs[1]);

    // State continuation
    let partial = close[..35].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[], None).unwrap();
    println!("Partial Sine:      {:?}", outputs2[0]);
    println!("Partial Lead Sine: {:?}", outputs2[1]);

    let new_close = close[35..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued Sine:      {:?}", continued[0]);
    println!("Continued Lead Sine: {:?}", continued[1]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    # adaptivemsw takes no options — pass an empty list
    outputs, state = tulip_rs.indicators.adaptivemsw.indicator([close], [])
    print("Sine:      ", outputs[0])
    print("Lead Sine: ", outputs[1])

    # State continuation
    partial = close[:35]
    outputs2, state = tulip_rs.indicators.adaptivemsw.indicator([partial], [])
    new_close = close[35:]
    continued = state.batch_indicator([new_close])
    print("Continued Sine:      ", continued[0])
    print("Continued Lead Sine: ", continued[1])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20]);

    const [outputs, state] = ti.adaptivemsw.indicator([close], []);
    console.log('Sine:     ', outputs[0]);
    console.log('Lead Sine:', outputs[1]);

    // State continuation
    const [, state2] = ti.adaptivemsw.indicator([close.slice(0, -5)], []);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Sine:     ', continued[0]);
    console.log('Continued Lead Sine:', continued[1]);
    ```

=== "WASM"

    ```javascript
    import { init, adaptivemsw } from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20];

    const [outputs, state] = adaptivemsw.indicator([close], []);
    console.log('Sine:     ', outputs[0]);
    console.log('Lead Sine:', outputs[1]);

    // State continuation
    const [, state2] = adaptivemsw.indicator([close.slice(0, -5)], []);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Sine:     ', continued[0]);
    console.log('Continued Lead Sine:', continued[1]);
    ```

### Optional Outputs

=== "Rust"

    `adaptivemsw` exposes 1 optional output: `dc_period`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::adaptivemsw::{AdaptiveMSW, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                     85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                     88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                     90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];

    let mask = [true]; // one per optional output
    let (outputs, _state) = AdaptiveMSW::indicator(&[close.as_slice()], &[], Some(&mask)).unwrap();

    let sine      = &outputs[0]; // sine (primary)
    let lead_sine = &outputs[1]; // lead_sine (primary)
    let dc_period = &outputs[2]; // dc_period (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    outputs, state = tulip_rs.indicators.adaptivemsw.indicator(
        [close], [],
        optional_outputs=[True],
    )

    sine      = outputs[0]  # sine (primary)
    lead_sine = outputs[1]  # lead_sine (primary)
    dc_period = outputs[2]  # dc_period (optional — requested)
    ```

=== "Node.js"

    `adaptivemsw` exposes 1 optional output: `dc_period`.

    ```javascript
    const [allOut] = ti.adaptivemsw.indicator([close], [], [true]);
    const sine     = allOut[0]; // primary
    const leadSine = allOut[1]; // primary
    const dcPeriod = allOut[2]; // optional 0: dc_period
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = adaptivemsw.indicator([close], [], [true]);
    const sine     = allOut[0]; // primary
    const leadSine = allOut[1]; // primary
    const dcPeriod = allOut[2]; // optional 0: dc_period
    ```

### SIMD

=== "Rust"

    **By assets** — applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::adaptivemsw::{AdaptiveMSW, Indicator, TIndicatorState};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                  85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                  88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                  90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50,
                  77.00, 77.50, 78.00, 78.50, 79.00, 79.50, 80.00, 80.50, 81.00, 81.50,
                  82.00, 82.50, 83.00, 83.50, 84.00, 84.50, 85.00, 85.50, 86.00, 86.50,
                  87.00, 87.50, 88.00, 88.50, 89.00, 89.50, 90.00, 90.50, 91.00, 91.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40,
                  58.70, 59.00, 59.30, 59.60, 59.90, 60.20, 60.50, 60.80, 61.10, 61.40,
                  61.70, 62.00, 62.30, 62.60, 62.90, 63.20, 63.50, 63.80, 64.10, 64.40,
                  64.70, 65.00, 65.30, 65.60, 65.90, 66.20, 66.50, 66.80, 67.10, 67.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8,
                  104.1, 104.5, 105.0, 105.3, 105.8, 106.0, 106.5, 107.0, 107.3, 107.8,
                  108.1, 108.5, 109.0, 109.3, 109.8, 110.0, 110.5, 111.0, 111.3, 111.8,
                  112.1, 112.5, 113.0, 113.3, 113.8, 114.0, 114.5, 115.0, 115.3, 115.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = AdaptiveMSW::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} Sine:      {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Lead Sine: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.adaptivemsw.simd_by_assets(simd_inputs, [])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Sine:      {out[0]}")
        print(f"Asset {i + 1} Lead Sine: {out[1]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.adaptivemsw.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1} Sine:`, out[0], 'Lead:', out[1]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
