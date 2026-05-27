# TEMA — Triple Exponential Moving Average

Further reduces lag with three EMA layers: `3 * EMA - 3 * EMA(EMA) + EMA(EMA(EMA))`. Reacts to price changes more quickly than DEMA at the cost of additional sensitivity to noise.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[tema]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::tema::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("TEMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial TEMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued TEMA: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.tema.indicator([close], [14.0])
    print("TEMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.tema.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued TEMA:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.tema.indicator([close], [14]);
    console.log('TEMA(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.tema.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TEMA:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `tema` exposes 2 optional outputs: `"dema"`, `"ema"`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::tema::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Request dema but not ema
    let mask = [true, false]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[5.0], Some(&mask)).unwrap();

    let tema = &outputs[0]; // tema (primary)
    let dema = &outputs[1]; // "dema" (optional — requested)
                            // "ema" not requested
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Request dema but not ema
    outputs, state = tulip_rs.indicators.tema.indicator(
        [close], [5.0],
        optional_outputs=[True, False],
    )

    tema = outputs[0]  # tema (primary)
    dema = outputs[1]  # "dema" (optional — requested)
                       # "ema" not requested
    ```

=== "Node.js"

    `tema` exposes 2 optional outputs: `dema`, `ema`. Pass a boolean mask — one bool per optional output.

    ```javascript
    // Request all optional outputs
    const [allOut] = ti.tema.indicator([close], [5], [true, true]);
    const tema = allOut[0]; // primary
    const dema = allOut[1]; // optional 0: dema
    const ema  = allOut[2]; // optional 1: ema

    // Request only dema
    const [partial] = ti.tema.indicator([close], [5], [true, false]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::tema::indicator_by_assets;

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

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::tema::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.tema.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.tema.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...close]],
        [close.map(v => v * 1.1)],
        [close.map(v => v * 0.9)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.tema.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.tema.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
