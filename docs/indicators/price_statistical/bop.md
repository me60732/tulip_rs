# BOP — Balance of Power — `bop`

Measures the strength of buyers vs sellers: `(Close - Open) / (High - Low)`.

**Inputs:** `[open, high, low, close]` | **Options:** none | **Outputs:** `[bop]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::bop::{Bop, TIndicatorState, Indicator};

    let open  = vec![81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25_f64];
    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [open.as_slice(), high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _) = Bop::indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation
    let partial_open  = open[..8].to_vec();
    let partial_high  = high[..8].to_vec();
    let partial_low   = low[..8].to_vec();
    let partial_close = close[..8].to_vec();
    let inputs2 = [partial_open.as_slice(), partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()];
    let (outputs2, mut state) = Bop::indicator(&inputs2, &[], None).unwrap();
    println!("Partial BOP: {:?}", outputs2[0]);

    let new_open  = open[8..].to_vec();
    let new_high  = high[8..].to_vec();
    let new_low   = low[8..].to_vec();
    let new_close = close[8..].to_vec();
    let new_inputs = [new_open.as_slice(), new_high.as_slice(), new_low.as_slice(), new_close.as_slice()];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued BOP: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.bop.indicator([open_, high, low, close], [])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.bop.indicator([open_, high, low, close], []);
    console.log('BOP:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.bop.indicator([open_.slice(0, n), high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([open_.slice(n), high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued BOP:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.bop.indicator([open_, high, low, close], []);
    console.log('BOP:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.bop.indicator([open_.slice(0, n), high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([open_.slice(n), high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued BOP:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::bop::{Bop, Indicator};

    let o1 = vec![81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25_f64];
    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let o2 = o1.clone(); let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let o3 = o1.clone(); let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let o4 = o1.clone(); let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 4]; 4] = [
        &[o1.as_slice(), h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[o2.as_slice(), h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[o3.as_slice(), h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[o4.as_slice(), h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = Bop::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[o1, h1, l1, c1], [o2, h2, l2, c2], [o3, h3, l3, c3], [o4, h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.bop.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...open_], high.slice(), low.slice(), close.slice()],
        [open_.map(v => v * 1.1), high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [open_.map(v => v * 0.9), high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [open_.map(v => v * 1.02), high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.bop.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
