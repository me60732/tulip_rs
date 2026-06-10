# SMA Envelope

Three bands around a Simple Moving Average. `middle = SMA(real, period)`, `upper = SMA + SMA × (percentage / 100)`, `lower = SMA − SMA × (percentage / 100)`. The envelope expands and contracts proportionally with the SMA level. Used to identify overbought/oversold conditions relative to the prevailing trend. Rendered as a price overlay.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period, percentage]` &nbsp;|&nbsp; **Outputs:** `[lower, middle, upper]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::smaenvelope::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // options: [period, percentage]
    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0, 2.5], None).unwrap();
    println!("Lower:  {:?}", outputs[0]);
    println!("Middle: {:?}", outputs[1]);
    println!("Upper:  {:?}", outputs[2]);

    // State continuation
    let (outputs2, mut state) = indicator(&[&close[..8]], &[14.0, 2.5], None).unwrap();
    println!("Partial Lower:  {:?}", outputs2[0]);
    println!("Partial Middle: {:?}", outputs2[1]);
    println!("Partial Upper:  {:?}", outputs2[2]);

    let continued = state.batch_indicator(&[&close[8..]], None).unwrap();
    println!("Continued Lower:  {:?}", continued[0]);
    println!("Continued Middle: {:?}", continued[1]);
    println!("Continued Upper:  {:?}", continued[2]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # options: [period, percentage]
    outputs, state = tulip_rs.indicators.smaenvelope.indicator([close], [14.0, 2.5])
    print("Lower: ", outputs[0])
    print("Middle:", outputs[1])
    print("Upper: ", outputs[2])

    # State continuation
    outputs2, state = tulip_rs.indicators.smaenvelope.indicator([close[:8]], [14.0, 2.5])
    print("Partial Lower: ", outputs2[0])
    print("Partial Middle:", outputs2[1])
    print("Partial Upper: ", outputs2[2])

    continued = state.batch_indicator([close[8:]])
    print("Continued Lower: ", continued[0])
    print("Continued Middle:", continued[1])
    print("Continued Upper: ", continued[2])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    // options: [period, percentage]
    const [outputs, state] = ti.smaenvelope.indicator([close], [14, 2.5]);
    console.log('SMA Envelope Lower:', outputs[0]);
    console.log('SMA Envelope Middle:', outputs[1]);
    console.log('SMA Envelope Upper:', outputs[2]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.smaenvelope.indicator([close.slice(0, n)], [14, 2.5]);
    const continued = state2.batchIndicator([close.slice(n)]);
    console.log('Continued Lower:', continued[0]);
    console.log('Continued Middle:', continued[1]);
    console.log('Continued Upper:', continued[2]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    // options: [period, percentage]
    const [outputs, state] = ti.smaenvelope.indicator([close], [14, 2.5]);
    console.log('SMA Envelope Lower:', outputs[0]);
    console.log('SMA Envelope Middle:', outputs[1]);
    console.log('SMA Envelope Upper:', outputs[2]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.smaenvelope.indicator([close.slice(0, n)], [14, 2.5]);
    const continued = state2.batchIndicator([close.slice(n)]);
    console.log('Continued Lower:', continued[0]);
    console.log('Continued Middle:', continued[1]);
    console.log('Continued Upper:', continued[2]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::smaenvelope::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = a1.iter().map(|x| x + 5.0).collect::<Vec<_>>();
    let a3 = a1.iter().map(|x| x - 5.0).collect::<Vec<_>>();
    let a4 = a1.iter().map(|x| x * 1.02).collect::<Vec<_>>();

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0, 2.5], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} Lower:  {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Middle: {:?}", i + 1, asset_outputs[1]);
        println!("Asset {} Upper:  {:?}", i + 1, asset_outputs[2]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::smaenvelope::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 2]; 4] = [
        &[10.0, 2.0],
        &[14.0, 2.5],
        &[20.0, 3.0],
        &[50.0, 5.0],
    ];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} Lower:  {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Middle: {:?}", i + 1, opt_outputs[1]);
        println!("Option set {} Upper:  {:?}", i + 1, opt_outputs[2]);
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [
        [close],
        [close + 5.0],
        [close - 5.0],
        [close * 1.02],
    ]
    outputs_list, states = tulip_rs.indicators.smaenvelope.simd_by_assets(simd_inputs, [14.0, 2.5])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Lower:  {out[0]}")
        print(f"Asset {i + 1} Middle: {out[1]}")
        print(f"Asset {i + 1} Upper:  {out[2]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    simd_options = [
        [10.0, 2.0],
        [14.0, 2.5],
        [20.0, 3.0],
        [50.0, 5.0],
    ]
    outputs_list, states = tulip_rs.indicators.smaenvelope.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} Lower:  {out[0]}")
        print(f"Option set {i + 1} Middle: {out[1]}")
        print(f"Option set {i + 1} Upper:  {out[2]}")
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
    const [results] = ti.smaenvelope.simdByAssets(simdInputs, [14, 2.5]);
    results.forEach((out, i) => {
        console.log(`Asset ${i + 1} Lower:`, out[0]);
        console.log(`Asset ${i + 1} Middle:`, out[1]);
        console.log(`Asset ${i + 1} Upper:`, out[2]);
    });
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[10, 2.0], [14, 2.5], [20, 3.0], [50, 5.0]];
    const [results] = ti.smaenvelope.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0], out[1], out[2]));
    ```
