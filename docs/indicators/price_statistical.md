# Price & Statistical Indicators

---

## Average Price — `avgprice`

The arithmetic mean of open, high, low, and close for each bar: `(O + H + L + C) / 4`.

**Inputs:** `[open, high, low, close]` | **Options:** none | **Outputs:** `[avgprice]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::avgprice::indicator;

    let open  = [81.85_f64, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25];
    let high  = [82.15_f64, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00];
    let low   = [81.29_f64, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11];
    let close = [81.59_f64, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36];

    let inputs = [open.as_slice(), high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::avgprice::indicator_by_assets;

    let inputs: [&[&[f64]; 4]; 4] = [
        &[o1.as_slice(), h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[o2.as_slice(), h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[o3.as_slice(), h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[o4.as_slice(), h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    open_  = np.array([81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25], dtype=np.float64)
    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.avgprice.indicator([open_, high, low, close], [])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [o1, h1, l1, c1],
        [o2, h2, l2, c2],
        [o3, h3, l3, c3],
        [o4, h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.avgprice.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.avgprice.indicator([open_, high, low, close], []);
    console.log('AvgPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.avgprice.indicator([open_.slice(0, n), high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([open_.slice(n), high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued AvgPrice:', continued[0]);
    ```

    ### SIMD

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...open_], [...high], [...low], [...close]],
        [open_.map(v => v * 1.1), high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [open_.map(v => v * 0.9), high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [open_.map(v => v * 1.02), high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.avgprice.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## Median Price — `medprice`

`(High + Low) / 2` for each bar.

**Inputs:** `[high, low]` | **Options:** none | **Outputs:** `[medprice]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::medprice::indicator;

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

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

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.medprice.indicator([high, low], [])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1], [h2, l2], [h3, l3], [h4, l4]]
    outputs_list, states = tulip_rs.indicators.medprice.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

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

    ### SIMD

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

---

## Typical Price — `typprice`

`(High + Low + Close) / 3` for each bar. Commonly used as the price input for indicators like CCI.

**Inputs:** `[high, low, close]` | **Options:** none | **Outputs:** `[typprice]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::typprice::indicator;

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::typprice::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.typprice.indicator([high, low, close], [])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1, c1], [h2, l2, c2], [h3, l3, c3], [h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.typprice.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.typprice.indicator([high, low, close], []);
    console.log('TypPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.typprice.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued TypPrice:', continued[0]);
    ```

    ### SIMD

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.typprice.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## Weighted Close Price — `wcprice`

`(High + Low + 2 × Close) / 4` — gives double weight to the closing price.

**Inputs:** `[high, low, close]` | **Options:** none | **Outputs:** `[wcprice]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::wcprice::indicator;

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::wcprice::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.wcprice.indicator([high, low, close], [])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1, c1], [h2, l2, c2], [h3, l3, c3], [h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.wcprice.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.wcprice.indicator([high, low, close], []);
    console.log('WCPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.wcprice.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued WCPrice:', continued[0]);
    ```

    ### SIMD

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.wcprice.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## Max — Highest Value Over Period — `max`

The highest value in the input series over a rolling `period` window.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[max]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::max::indicator;

    let close = [81.59_f64, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36];
    let (outputs, _) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::max::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::max::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[20.0], &[50.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.max.indicator([close], [14.0])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.max.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [20.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.max.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.max.indicator([close], [14]);
    console.log('Max(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.max.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Max:', continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.max.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [20], [50]];
    const [results] = ti.max.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## Min — Lowest Value Over Period — `min`

The lowest value in the input series over a rolling `period` window.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[min]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::min::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::min::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::min::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[20.0], &[50.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.min.indicator([close], [14.0])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.min.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [20.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.min.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.min.indicator([close], [14]);
    console.log('Min(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.min.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued Min:', continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.min.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [20], [50]];
    const [results] = ti.min.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## MOM — Momentum — `mom`

The difference between the current price and the price `period` bars ago: `close[i] - close[i - period]`.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[mom]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::mom::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::mom::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::mom::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[20.0], &[50.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.mom.indicator([close], [10.0])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.mom.simd_by_assets(simd_inputs, [10.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [20.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.mom.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.mom.indicator([close], [10]);
    console.log('MOM(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.mom.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued MOM:', continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.mom.simdByAssets(simdInputs, [10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [20], [50]];
    const [results] = ti.mom.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## ROC — Rate of Change — `roc`

The percentage change between the current price and the price `period` bars ago.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[roc]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::roc::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### Optional Outputs

    `roc` exposes 1 optional output: `mom`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::roc::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[10.0], Some(&mask)).unwrap();

    let roc = &outputs[0]; // roc (primary)
    let mom = &outputs[1]; // mom (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::roc::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::roc::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[20.0], &[50.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.roc.indicator([close], [10.0])
    print(outputs[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.roc.indicator(
        [close], [10.0],
        optional_outputs=[True],
    )

    roc = outputs[0]  # roc (primary)
    mom = outputs[1]  # mom (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.roc.simd_by_assets(simd_inputs, [10.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [20.0], [50.0]]
    outputs_list, states = tulip_rs.indicators.roc.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.roc.indicator([close], [10]);
    console.log('ROC(10):', outputs[0]);

    // State continuation
    const [, state2] = ti.roc.indicator([close.slice(0, -5)], [10]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued ROC:', continued[0]);
    ```

    ### Optional Outputs

    `roc` exposes 1 optional output: `mom`.

    ```javascript
    const [allOut] = ti.roc.indicator([close], [10], [true]);
    const roc = allOut[0]; // primary
    const mom = allOut[1]; // optional 0: mom
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.roc.simdByAssets(simdInputs, [10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [20], [50]];
    const [results] = ti.roc.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## ROCR — Rate of Change Ratio — `rocr`

The ratio of the current price to the price `period` bars ago (equivalent to `1 + ROC / 100`).

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[rocr]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::rocr::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

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

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.rocr.indicator([close], [10.0])
    print(outputs[0])
    ```

    ### SIMD

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

    ### Basic

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

---

## BOP — Balance of Power — `bop`

Measures the strength of buyers vs sellers: `(Close - Open) / (High - Low)`.

**Inputs:** `[open, high, low, close]` | **Options:** none | **Outputs:** `[bop]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::bop::indicator;

    let inputs = [open_.as_slice(), high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::bop::indicator_by_assets;

    let inputs: [&[&[f64]; 4]; 4] = [
        &[o1.as_slice(), h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[o2.as_slice(), h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[o3.as_slice(), h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[o4.as_slice(), h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.bop.indicator([open_, high, low, close], [])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[o1, h1, l1, c1], [o2, h2, l2, c2], [o3, h3, l3, c3], [o4, h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.bop.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

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

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...open_], [...high], [...low], [...close]],
        [open_.map(v => v * 1.1), high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [open_.map(v => v * 0.9), high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [open_.map(v => v * 1.02), high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.bop.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## LinReg — Linear Regression — `linreg`

The end-point of a least-squares linear regression line fitted to the last `period` bars. Often used as a low-lag trend line.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[linreg]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::linreg::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### Optional Outputs

    `linreg` exposes 2 optional outputs: `linregslope`, `linregintercept`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::linreg::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], Some(&mask)).unwrap();

    let linreg          = &outputs[0]; // linreg (primary)
    let linregslope     = &outputs[1]; // linregslope (optional — requested)
    let linregintercept = &outputs[2]; // linregintercept (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::linreg::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::linreg::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.linreg.indicator([close], [14.0])
    print(outputs[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.linreg.indicator(
        [close], [14.0],
        optional_outputs=[True, True],
    )

    linreg          = outputs[0]  # linreg (primary)
    linregslope     = outputs[1]  # linregslope (optional — requested)
    linregintercept = outputs[2]  # linregintercept (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.linreg.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.linreg.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.linreg.indicator([close], [14]);
    console.log('LinReg(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.linreg.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued LinReg:', continued[0]);
    ```

    ### Optional Outputs

    `linreg` exposes 2 optional outputs: `linregslope`, `linregintercept`.

    ```javascript
    const [allOut] = ti.linreg.indicator([close], [14], [true, true]);
    const linreg          = allOut[0]; // primary
    const linregslope     = allOut[1]; // optional 0: linregslope
    const linregintercept = allOut[2]; // optional 1: linregintercept
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.linreg.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.linreg.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## TSF — Time Series Forecast — `tsf`

Projects the linear regression line one bar forward, giving a one-period-ahead price forecast.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[tsf]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::tsf::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### Optional Outputs

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

    ### SIMD

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

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.tsf.indicator([close], [14.0])
    print(outputs[0])
    ```

    ### Optional Outputs

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

    ### SIMD

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

    ### Basic

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

    ### Optional Outputs

    `tsf` exposes 3 optional outputs: `linreg`, `linregslope`, `linregintercept`.

    ```javascript
    const [allOut] = ti.tsf.indicator([close], [14], [true, true, true]);
    const tsf             = allOut[0]; // primary
    const linreg          = allOut[1]; // optional 0: linreg
    const linregslope     = allOut[2]; // optional 1: linregslope
    const linregintercept = allOut[3]; // optional 2: linregintercept
    ```

    ### SIMD

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

---

## TRIX — `trix`

The 1-period percentage rate of change of a triple-smoothed EMA. Useful as a momentum oscillator or trend filter; signals only significant moves.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[trix]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::trix::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### Optional Outputs

    `trix` exposes 3 optional outputs: `tema`, `dema`, `ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::trix::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [false, false, true]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[5.0], Some(&mask)).unwrap();

    let trix = &outputs[0]; // trix (primary)
    let ema  = &outputs[1]; // ema (optional — requested)
    // tema not requested, dema not requested
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::trix::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::trix::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[9.0], &[14.0], &[21.0], &[30.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.trix.indicator([close], [14.0])
    print(outputs[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.trix.indicator(
        [close], [5.0],
        optional_outputs=[False, False, True],
    )

    trix = outputs[0]  # trix (primary)
    ema  = outputs[1]  # ema (optional — requested)
    # tema not requested, dema not requested
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.trix.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[9.0], [14.0], [21.0], [30.0]]
    outputs_list, states = tulip_rs.indicators.trix.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.trix.indicator([close], [14]);
    console.log('TRIX(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.trix.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued TRIX:', continued[0]);
    ```

    ### Optional Outputs

    `trix` exposes 3 optional outputs: `tema`, `dema`, `ema`.

    ```javascript
    const [allOut] = ti.trix.indicator([close], [14], [true, true, true]);
    const trix = allOut[0]; // primary
    const tema = allOut[1]; // optional 0: tema
    const dema = allOut[2]; // optional 1: dema
    const ema  = allOut[3]; // optional 2: ema
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.trix.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[9], [14], [21], [30]];
    const [results] = ti.trix.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## DPO — Detrended Price Oscillator — `dpo`

Removes the trend from price by comparing it to a displaced moving average, highlighting underlying cycles.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[dpo]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::dpo::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### Optional Outputs

    `dpo` exposes 1 optional output: `sma`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::dpo::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], Some(&mask)).unwrap();

    let dpo = &outputs[0]; // dpo (primary)
    let sma = &outputs[1]; // sma (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::dpo::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::dpo::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.dpo.indicator([close], [14.0])
    print(outputs[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.dpo.indicator(
        [close], [14.0],
        optional_outputs=[True],
    )

    dpo = outputs[0]  # dpo (primary)
    sma = outputs[1]  # sma (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.dpo.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.dpo.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.dpo.indicator([close], [14]);
    console.log('DPO(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.dpo.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued DPO:', continued[0]);
    ```

    ### Optional Outputs

    `dpo` exposes 1 optional output: `sma`.

    ```javascript
    const [allOut] = ti.dpo.indicator([close], [14], [true]);
    const dpo = allOut[0]; // primary
    const sma = allOut[1]; // optional 0: sma
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.dpo.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.dpo.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## Mass Index — `mass`

Uses the high-low trading range to identify potential trend reversals via range expansion. Watch for values rising above 27 then falling below 26.5 — this "reversal bulge" signals a likely trend change.

**Inputs:** `[high, low]` | **Options:** `[period]` | **Outputs:** `[mass]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::mass::indicator;

    let high = [82.15_f64, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00];
    let low  = [81.29_f64, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, _) = indicator(&inputs, &[25.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::mass::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[25.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::mass::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[15.0], &[20.0], &[25.0], &[30.0]];
    let results = indicator_by_options::<4>(&inputs_single, &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)

    outputs, state = tulip_rs.indicators.mass.indicator([high, low], [25.0])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1], [h2, l2], [h3, l3], [h4, l4]]
    outputs_list, states = tulip_rs.indicators.mass.simd_by_assets(simd_inputs, [25.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[15.0], [20.0], [25.0], [30.0]]
    outputs_list, states = tulip_rs.indicators.mass.simd_by_options([high, low], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.mass.indicator([high, low], [25]);
    console.log('Mass(25):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.mass.indicator([high.slice(0, n), low.slice(0, n)], [25]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued Mass:', continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.mass.simdByAssets(simdInputs, [25]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[15], [20], [25], [30]];
    const [results] = ti.mass.simdByOptions([high, low], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## MD — Mean Deviation — `md`

The mean of the absolute deviations of each bar from the rolling mean over `period` bars. Similar to standard deviation but uses absolute rather than squared differences.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[md]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::md::indicator;

    let (outputs, _) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### Optional Outputs

    `md` exposes 1 optional output: `sma`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::md::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // one per optional output
    let (outputs, _state) = indicator(&[close.as_slice()], &[10.0], Some(&mask)).unwrap();

    let md  = &outputs[0]; // md (primary)
    let sma = &outputs[1]; // sma (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::md::indicator_by_assets;

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::md::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    outputs, state = tulip_rs.indicators.md.indicator([close], [14.0])
    print(outputs[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.md.indicator(
        [close], [10.0],
        optional_outputs=[True],
    )

    md  = outputs[0]  # md (primary)
    sma = outputs[1]  # sma (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.md.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.md.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.md.indicator([close], [14]);
    console.log('MD(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.md.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued MD:', continued[0]);
    ```

    ### Optional Outputs

    `md` exposes 1 optional output: `sma`.

    ```javascript
    const [allOut] = ti.md.indicator([close], [14], [true]);
    const md  = allOut[0]; // primary
    const sma = allOut[1]; // optional 0: sma
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[[...close]], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.md.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.md.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## Market Facilitation Index — `marketfi`

`(High - Low) / Volume` — measures the efficiency of price movement per unit of volume traded.

**Inputs:** `[high, low, volume]` | **Options:** none | **Outputs:** `[marketfi]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::marketfi::indicator;

    let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

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

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0, 900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.marketfi.indicator([high, low, volume], [])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1, v1], [h2, l2, v2], [h3, l3, v3], [h4, l4, v4]]
    outputs_list, states = tulip_rs.indicators.marketfi.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

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

---

## QStick — `qstick`

A moving average of `(Close - Open)` over `period` bars, summarising buying or selling pressure.

**Inputs:** `[open, close]` | **Options:** `[period]` | **Outputs:** `[qstick]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::qstick::indicator;

    let open_ = [81.85_f64, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25];
    let close = [81.59_f64, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36];

    let inputs = [open_.as_slice(), close.as_slice()];
    let (outputs, _) = indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::qstick::indicator_by_assets;

    let inputs: [&[&[f64]; 2]; 4] = [
        &[o1.as_slice(), c1.as_slice()],
        &[o2.as_slice(), c2.as_slice()],
        &[o3.as_slice(), c3.as_slice()],
        &[o4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::qstick::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let results = indicator_by_options::<4>(&inputs_single, &opts, None).unwrap();
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    open_  = np.array([81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.qstick.indicator([open_, close], [14.0])
    print(outputs[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[o1, c1], [o2, c2], [o3, c3], [o4, c4]]
    outputs_list, states = tulip_rs.indicators.qstick.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.qstick.simd_by_options([open_, close], simd_options)
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const open_ = [81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18, 88.00, 87.30];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.qstick.indicator([open_, close], [14]);
    console.log('QStick(14):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.qstick.indicator([open_.slice(0, n), close.slice(0, n)], [14]);
    const continued = state2.batchIndicator([open_.slice(n), close.slice(n)]);
    console.log('Continued QStick:', continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...open_], [...close]],
        [open_.map(v => v * 1.1), close.map(v => v * 1.1)],
        [open_.map(v => v * 0.9), close.map(v => v * 0.9)],
        [open_.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.qstick.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.qstick.simdByOptions([open_, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## Pivot Point — `pivotpoint`

Classic floor-trader pivot points calculated from the previous bar's high, low, and close. Provides a central pivot level plus two support and two resistance levels.

**Inputs:** `[high, low, close]` | **Options:** none | **Outputs:** `[pivot, r1, s1, r2, s2]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::pivotpoint::indicator;

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _) = indicator(&inputs, &[], None).unwrap();

    println!("Pivot: {:?}", outputs[0]);
    println!("R1:    {:?}", outputs[1]);
    println!("S1:    {:?}", outputs[2]);
    println!("R2:    {:?}", outputs[3]);
    println!("S2:    {:?}", outputs[4]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::pivotpoint::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    // results[i][0] = pivot, [1] = R1, [2] = S1, [3] = R2, [4] = S2 for asset i
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

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

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1, c1], [h2, l2, c2], [h3, l3, c3], [h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.pivotpoint.simd_by_assets(simd_inputs, [])
    # outputs_list[i][0] = pivot, [1] = R1, [2] = S1, [3] = R2, [4] = S2 for asset i
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

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

    ### SIMD

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.pivotpoint.simdByAssets(simdInputs, []);
    // results[i][0] = pivot, [1] = R1, [2] = S1, [3] = R2, [4] = S2 for asset i
    results.forEach((out, i) => console.log(`Asset ${i + 1} Pivot:`, out[0], 'R1:', out[1], 'S1:', out[2]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
