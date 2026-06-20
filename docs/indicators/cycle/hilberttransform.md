# Hilbert Transform — Hilbert Transform

Decomposes the roofing-filtered price series into in-phase and quadrature components using Ehlers' Hilbert Transform; the two outputs represent the cycle's cosine and sine components respectively.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[ss_period, hp_period]` &nbsp;|&nbsp; **Outputs:** `[in_phase, quadrature]` &nbsp;|&nbsp; **Optional:** `[roofing, highpass]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::hilberttransform::indicator;

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    // Options: [ss_period, hp_period]
    let (outputs, _state) = indicator(&[close.as_slice()], &[10.0, 20.0], None).unwrap();
    println!("In-Phase:    {:?}", outputs[0]);
    println!("Quadrature:  {:?}", outputs[1]);

    // State continuation
    let n = close.len() - 5;
    let partial = close[..n].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[10.0, 20.0], None).unwrap();
    println!("Partial In-Phase: {:?}", outputs2[0]);

    let rest = close[n..].to_vec();
    let continued = state.batch_indicator(&[rest.as_slice()], None).unwrap();
    println!("Continued In-Phase:   {:?}", continued[0]);
    println!("Continued Quadrature: {:?}", continued[1]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    # Options: [ss_period, hp_period]
    outputs, state = tulip_rs.indicators.hilberttransform.indicator([close], [10.0, 20.0])
    print("In-Phase:   ", outputs[0])
    print("Quadrature: ", outputs[1])

    # State continuation
    partial = close[:-5]
    outputs2, state = tulip_rs.indicators.hilberttransform.indicator([partial], [10.0, 20.0])
    rest = close[-5:]
    continued = state.batch_indicator([rest])
    print("Continued In-Phase:   ", continued[0])
    print("Continued Quadrature: ", continued[1])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ]);

    const [outputs, state] = ti.hilberttransform.indicator([close], [10, 20]);
    console.log('In-Phase:   ', outputs[0]);
    console.log('Quadrature: ', outputs[1]);

    // State continuation
    const [, state2] = ti.hilberttransform.indicator([close.slice(0, -5)], [10, 20]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued In-Phase:   ', continued[0]);
    console.log('Continued Quadrature: ', continued[1]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ];

    const [outputs, state] = ti.hilberttransform.indicator([close], [10, 20]);
    console.log('In-Phase:   ', outputs[0]);
    console.log('Quadrature: ', outputs[1]);

    // State continuation
    const [, state2] = ti.hilberttransform.indicator([close.slice(0, -5)], [10, 20]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued In-Phase:   ', continued[0]);
    console.log('Continued Quadrature: ', continued[1]);
    ```

### Optional Outputs

=== "Rust"

    `hilberttransform` exposes 2 optional outputs: `roofing`, `highpass`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::hilberttransform::indicator;

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    let mask = [true, true]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[10.0, 20.0], Some(&mask)).unwrap();

    let in_phase    = &outputs[0]; // in_phase (primary)
    let quadrature  = &outputs[1]; // quadrature (primary)
    let roofing     = &outputs[2]; // roofing (optional — requested)
    let highpass    = &outputs[3]; // highpass (optional — requested)
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    outputs, state = tulip_rs.indicators.hilberttransform.indicator(
        [close], [10.0, 20.0],
        optional_outputs=[True, True],
    )

    in_phase   = outputs[0]  # in_phase (primary)
    quadrature = outputs[1]  # quadrature (primary)
    roofing    = outputs[2]  # roofing (optional — requested)
    highpass   = outputs[3]  # highpass (optional — requested)
    ```

=== "Node.js"

    `hilberttransform` exposes 2 optional outputs: `roofing`, `highpass`.

    ```javascript
    const [allOut] = ti.hilberttransform.indicator([close], [10, 20], [true, true]);
    const inPhase    = allOut[0]; // primary
    const quadrature = allOut[1]; // primary
    const roofing    = allOut[2]; // optional 0: roofing
    const highpass   = allOut[3]; // optional 1: highpass
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.hilberttransform.indicator([close], [10, 20], [true, true]);
    const inPhase    = allOut[0]; // primary
    const quadrature = allOut[1]; // primary
    const roofing    = allOut[2]; // optional 0: roofing
    const highpass   = allOut[3]; // optional 1: highpass
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::hilberttransform::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36_f64];
    let a3 = vec![78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36_f64];
    let a4 = vec![83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[10.0, 20.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} In-Phase:   {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Quadrature: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::hilberttransform::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 2]; 4] = [
        &[5.0,  10.0],
        &[10.0, 20.0],
        &[14.0, 30.0],
        &[20.0, 40.0],
    ];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} In-Phase:   {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Quadrature: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 3.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.hilberttransform.simd_by_assets(simd_inputs, [10.0, 20.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} In-Phase:   {out[0]}")
        print(f"Asset {i + 1} Quadrature: {out[1]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_options = [
        [5.0,  10.0],
        [10.0, 20.0],
        [14.0, 30.0],
        [20.0, 40.0],
    ]
    outputs_list, states = tulip_rs.indicators.hilberttransform.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} In-Phase:   {out[0]}")
        print(f"Option set {i + 1} Quadrature: {out[1]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v + 5.0)],
        [close.map(v => v - 3.0)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.hilberttransform.simdByAssets(simdInputs, [10, 20]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} In-Phase:`, out[0], 'Quadrature:', out[1]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[5, 10], [10, 20], [14, 30], [20, 40]];
    const [results] = ti.hilberttransform.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} In-Phase:`, out[0]));
    ```
