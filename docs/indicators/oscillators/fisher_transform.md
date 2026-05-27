# Fisher Transform

Converts prices into a Gaussian normal distribution. Sharp moves in the Fisher value can signal potential price reversals; the signal line is a one-bar lag of the Fisher line.

**Inputs:** `[high, low]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[fisher, fisher_signal]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::fisher::indicator;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[10.0], None).unwrap();
    println!("Fisher:        {:?}", outputs[0]);
    println!("Fisher Signal: {:?}", outputs[1]);

    // State continuation
    let inputs2 = [&high[..10], &low[..10]];
    let (outputs2, mut state) = indicator(&inputs2, &[10.0], None).unwrap();
    println!("Partial Fisher: {:?}", outputs2[0]);

    let new_inputs = [&high[10..], &low[10..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued Fisher:        {:?}", continued[0]);
    println!("Continued Fisher Signal: {:?}", continued[1]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    outputs, state = tulip_rs.indicators.fisher.indicator([high, low], [10.0])
    print("Fisher:        ", outputs[0])
    print("Fisher Signal: ", outputs[1])

    # State continuation
    outputs2, state = tulip_rs.indicators.fisher.indicator([high[:10], low[:10]], [10.0])
    continued = state.batch_indicator([high[10:], low[10:]])
    print("Continued Fisher:        ", continued[0])
    print("Continued Fisher Signal: ", continued[1])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.fisher.indicator([high, low], [9]);
    console.log('Fisher:', outputs[0]);
    console.log('Signal:', outputs[1]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.fisher.indicator([high.slice(0, n), low.slice(0, n)], [9]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued Fisher:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::fisher::indicator_by_assets;

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                  85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                  84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let h2 = h1.clone(); let l2 = l1.clone();
    let h3 = h1.clone(); let l3 = l1.clone();
    let h4 = h1.clone(); let l4 = l1.clone();

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} Fisher:        {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Fisher Signal: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::fisher::indicator_by_options;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let inputs = [high.as_slice(), low.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} Fisher:        {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Fisher Signal: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    simd_inputs = [
        [high,        low],
        [high + 0.5,  low + 0.5],
        [high - 0.5,  low - 0.5],
        [high * 1.01, low * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.fisher.simd_by_assets(simd_inputs, [10.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Fisher:        {out[0]}")
        print(f"Asset {i + 1} Fisher Signal: {out[1]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.fisher.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} Fisher:        {out[0]}")
        print(f"Option set {i + 1} Fisher Signal: {out[1]}")
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
    const [results] = ti.fisher.simdByAssets(simdInputs, [9]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} Fisher:`, out[0], 'Signal:', out[1]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.fisher.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]} Fisher:`, out[0], 'Signal:', out[1]));
    ```
