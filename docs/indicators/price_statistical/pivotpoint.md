# Pivot Point — `pivotpoint`

Classic floor-trader pivot points calculated from the previous bar's high, low, and close. Provides a central pivot level plus two support and two resistance levels.

**Inputs:** `[high, low, close]` | **Options:** none | **Outputs:** `[pivot, r1, s1, r2, s2]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::pivotpoint::{PivotPoint, Indicator, TIndicatorState};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = PivotPoint::indicator(&inputs, &[], None).unwrap();

    println!("Pivot: {:?}", outputs[0]);
    println!("R1:    {:?}", outputs[1]);
    println!("S1:    {:?}", outputs[2]);
    println!("R2:    {:?}", outputs[3]);
    println!("S2:    {:?}", outputs[4]);

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let (outputs2, mut state) = PivotPoint::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice()], &[], None).unwrap();

    println!("Pivot: {:?}", outputs2[0]);
    println!("R1:    {:?}", outputs2[1]);
    println!("S1:    {:?}", outputs2[2]);

    let new_high   = vec![85.90_f64];
    let new_low    = vec![84.03_f64];
    let new_close  = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()], None).unwrap();
    println!("Continued Pivot: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.pivotpoint.indicator([high, low, close], [])

    print(f"Pivot: {outputs[0]}")
    print(f"R1:    {outputs[1]}")
    print(f"S1:    {outputs[2]}")
    print(f"R2:    {outputs[3]}")
    print(f"S2:    {outputs[4]}")
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.pivotpoint.indicator([high, low, close], []);
    console.log('Pivot:', outputs[0]);
    console.log('R1:',   outputs[1]);
    console.log('S1:',   outputs[2]);
    console.log('R2:',   outputs[3]);
    console.log('S2:',   outputs[4]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.pivotpoint.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Pivot:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.pivotpoint.indicator([high, low, close], []);
    console.log('Pivot:', outputs[0]);
    console.log('R1:',   outputs[1]);
    console.log('S1:',   outputs[2]);
    console.log('R2:',   outputs[3]);
    console.log('S2:',   outputs[4]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.pivotpoint.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued Pivot:', continued[0]);
    ```
