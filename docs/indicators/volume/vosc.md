# VOSC — Volume Oscillator

The percentage difference between two volume moving averages. Expanding volume oscillator supports the price trend.

**Inputs:** `[volume]` | **Options:** `[short_period, long_period]` | **Outputs:** `[vosc]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::vosc::indicator;

    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    // options: [short_period, long_period]
    let (outputs, mut state) = indicator(&[volume.as_slice()], &[5.0, 10.0], None).unwrap();
    println!("{:?}", outputs[0]); // VOSC values

    // State continuation — feed new bars without reprocessing history
    let new_volume = vec![1600.0, 1250.0_f64];
    let continued = state.batch_indicator(&[new_volume.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.vosc.indicator([volume], [5.0, 10.0])
    print(outputs[0])  # VOSC values

    # State continuation
    new_volume = np.array([1600.0, 1250.0], dtype=np.float64)
    continued = state.batch_indicator([new_volume])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.vosc.indicator([volume], [5, 10]);
    console.log('VOSC:', outputs[0]);

    // State continuation
    const n = volume.length - 5;
    const [, state2] = ti.vosc.indicator([volume.slice(0, n)], [5, 10]);
    const continued = state2.batchIndicator([volume.slice(n)]);
    console.log('Continued VOSC:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.vosc.indicator([volume], [5, 10]);
    console.log('VOSC:', outputs[0]);

    // State continuation
    const n = volume.length - 5;
    const [, state2] = ti.vosc.indicator([volume.slice(0, n)], [5, 10]);
    const continued = state2.batchIndicator([volume.slice(n)]);
    console.log('Continued VOSC:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `vosc` exposes 2 optional outputs: `short_sma`, `long_sma`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::vosc::indicator;

    let volume = vec![10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0_f64];

    let mask = [true, true];
    let (outputs, _state) = indicator(&[volume.as_slice()], &[5.0, 20.0], Some(&mask)).unwrap();

    let vosc      = &outputs[0]; // vosc (primary)
    let short_sma = &outputs[1]; // short_sma (optional — requested)
    let long_sma  = &outputs[2]; // long_sma (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    volume = np.array([10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.vosc.indicator(
        [volume], [5.0, 20.0],
        optional_outputs=[True, True],
    )

    vosc      = outputs[0]  # vosc (primary)
    short_sma = outputs[1]  # short_sma (optional — requested)
    long_sma  = outputs[2]  # long_sma (optional — requested)
    ```

=== "Node.js"

    `vosc` exposes 2 optional outputs: `short_sma`, `long_sma`.

    ```javascript
    const [allOut] = ti.vosc.indicator([volume], [5, 10], [true, true]);
    const vosc     = allOut[0]; // primary
    const shortSma = allOut[1]; // optional 0: short_sma
    const longSma  = allOut[2]; // optional 1: long_sma
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::vosc::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [
        &[v1.as_slice()],
        &[v2.as_slice()],
        &[v3.as_slice()],
        &[v4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[5.0, 10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::vosc::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[3.0, 6.0], &[5.0, 10.0], &[8.0, 16.0], &[12.0, 24.0]];
    let results = indicator_by_options::<4>(&[volume.as_slice()], &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [v1],
        [v2],
        [v3],
        [v4],
    ]
    outputs_list, states = tulip_rs.indicators.vosc.simd_by_assets(simd_inputs, [5.0, 10.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[3.0, 6.0], [5.0, 10.0], [8.0, 16.0], [12.0, 24.0]]
    outputs_list, states = tulip_rs.indicators.vosc.simd_by_options([volume], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...volume]],
        [volume.map(v => v * 1.1)],
        [volume.map(v => v * 0.9)],
        [volume.map(v => v * 1.02)],
    ];
    const [results] = ti.vosc.simdByAssets(simdInputs, [5, 10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[3, 6], [5, 10], [8, 16], [12, 24]];
    const [results] = ti.vosc.simdByOptions([volume], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```
