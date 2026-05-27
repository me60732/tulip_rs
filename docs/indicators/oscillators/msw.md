# MSW — Mesa Sine Wave

Fits a sine wave to the recent price data over `period` bars. The crossover of the sine and lead (phase-advanced) lines can signal cycle turns and potential entry/exit points.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[msw_sine, msw_lead]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::msw::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("MSW Sine: {:?}", outputs[0]);
    println!("MSW Lead: {:?}", outputs[1]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[10.0], None).unwrap();
    println!("Partial MSW Sine: {:?}", outputs2[0]);
    println!("Partial MSW Lead: {:?}", outputs2[1]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued MSW Sine: {:?}", continued[0]);
    println!("Continued MSW Lead: {:?}", continued[1]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.msw.indicator([close], [10.0])
    print("MSW Sine:", outputs[0])
    print("MSW Lead:", outputs[1])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.msw.indicator([partial], [10.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued MSW Sine:", continued[0])
    print("Continued MSW Lead:", continued[1])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.msw.indicator([close], [5]);
    console.log('MSW Sine:', outputs[0]);
    console.log('MSW Lead:', outputs[1]);

    // State continuation
    const [, state2] = ti.msw.indicator([close.slice(0, -5)], [5]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Sine:', continued[0], 'Lead:', continued[1]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::msw::indicator_by_assets;

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

    let results = indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} Sine: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Lead: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::msw::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} Sine: {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Lead: {:?}", i + 1, opt_outputs[1]);
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
    outputs_list, states = tulip_rs.indicators.msw.simd_by_assets(simd_inputs, [10.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Sine: {out[0]}")
        print(f"Asset {i + 1} Lead: {out[1]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.msw.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} Sine: {out[0]}")
        print(f"Option set {i + 1} Lead: {out[1]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.msw.simdByAssets(simdInputs, [5]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} Sine:`, out[0], 'Lead:', out[1]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.msw.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} Sine:`, out[0], 'Lead:', out[1]));
    ```
