# TSF — Time Series Forecast — `tsf`

Projects the linear regression line one bar forward, giving a one-period-ahead price forecast.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[tsf]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::tsf::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.tsf.indicator([close], [14.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.tsf.indicator([close], [14]);
    console.log('TSF(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.tsf.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TSF:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.tsf.indicator([close], [14]);
    console.log('TSF(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.tsf.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TSF:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `tsf` exposes 3 optional outputs: `linreg`, `linregslope`, `linregintercept`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::tsf::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true, false]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], Some(&mask)).unwrap();

    let tsf         = &outputs[0]; // tsf (primary)
    let linreg      = &outputs[1]; // linreg (optional — requested)
    let linregslope = &outputs[2]; // linregslope (optional — requested)
    // linregintercept not requested
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.tsf.indicator(
        [close], [14.0],
        optional_outputs=[True, True, False],
    )

    tsf         = outputs[0]  # tsf (primary)
    linreg      = outputs[1]  # linreg (optional — requested)
    linregslope = outputs[2]  # linregslope (optional — requested)
    # linregintercept not requested
    ```

=== "Node.js"

    `tsf` exposes 3 optional outputs: `linreg`, `linregslope`, `linregintercept`.

    ```javascript
    const [allOut] = ti.tsf.indicator([close], [14], [true, true, true]);
    const tsf             = allOut[0]; // primary
    const linreg          = allOut[1]; // optional 0: linreg
    const linregslope     = allOut[2]; // optional 1: linregslope
    const linregintercept = allOut[3]; // optional 2: linregintercept
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::tsf::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::tsf::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.tsf.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.tsf.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.tsf.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.tsf.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
