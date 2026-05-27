# PVI — Positive Volume Index

Tracks price changes on days when volume increases. Complements NVI.

**Inputs:** `[real, volume]` | **Options:** `[]` | **Outputs:** `[pvi]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::pvi::indicator;

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    let inputs = [close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // PVI values

    // State continuation — feed new bars without reprocessing history
    let new_close  = vec![85.50_f64];
    let new_volume = vec![1900.0_f64];
    let continued = state.batch_indicator(
        &[new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.pvi.indicator([close, volume], [])
    print(outputs[0])  # PVI values

    # State continuation
    new_close  = np.array([85.50], dtype=np.float64)
    new_volume = np.array([1900.0], dtype=np.float64)
    continued = state.batch_indicator([new_close, new_volume])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.pvi.indicator([close, volume], []);
    console.log('PVI:', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.pvi.indicator([close.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([close.slice(n), volume.slice(n)]);
    console.log('Continued PVI:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::pvi::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[c1.as_slice(), v1.as_slice()],
        &[c2.as_slice(), v2.as_slice()],
        &[c3.as_slice(), v3.as_slice()],
        &[c4.as_slice(), v4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [c1, v1],
        [c2, v2],
        [c3, v3],
        [c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.pvi.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...close], [...volume]],
        [close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.pvi.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
