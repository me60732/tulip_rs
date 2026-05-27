# ROCR — Rate of Change Ratio — `rocr`

The ratio of the current price to the price `period` bars ago (equivalent to `1 + ROC / 100`).

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[rocr]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::rocr::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.rocr.indicator([close], [10.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.rocr.indicator([close], [10]);
    console.log('ROCR(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.rocr.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued ROCR:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::rocr::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::rocr::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[20.0], &[50.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.rocr.simd_by_assets(simd_inputs, [10.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [20.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.rocr.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.rocr.simdByAssets(simdInputs, [10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [20], [50]];
    const [results] = ti.rocr.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
