# Median Price — `medprice`

`(High + Low) / 2` for each bar.

**Inputs:** `[high, low]` | **Options:** none | **Outputs:** `[medprice]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::medprice::indicator;

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.medprice.indicator([high, low], [])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.medprice.indicator([high, low], []);
    console.log('MedPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.medprice.indicator([high.slice(0, n), low.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued MedPrice:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.medprice.indicator([high, low], []);
    console.log('MedPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.medprice.indicator([high.slice(0, n), low.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued MedPrice:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::medprice::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1], [h2, l2], [h3, l3], [h4, l4]]
    outputs_list, states = tulip_rs.indicators.medprice.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.medprice.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
