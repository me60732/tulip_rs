# Market Facilitation Index — `marketfi`

`(High - Low) / Volume` — measures the efficiency of price movement per unit of volume traded.

**Inputs:** `[high, low, volume]` | **Options:** none | **Outputs:** `[marketfi]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::marketfi::indicator;

    let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0, 900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.marketfi.indicator([high, low, volume], [])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.marketfi.indicator([high, low, volume], []);
    console.log('MarketFi:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.marketfi.indicator([high.slice(0, n), low.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), volume.slice(n)]);
    console.log('Continued MarketFi:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::marketfi::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), v4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1, v1], [h2, l2, v2], [h3, l3, v3], [h4, l4, v4]]
    outputs_list, states = tulip_rs.indicators.marketfi.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...volume]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.marketfi.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
