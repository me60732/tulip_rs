# Volume Indicators

AD, ADOSC, OBV, MFI, NVI, PVI, VOSC, KVO, EMV, and WAD all follow the universal TulipRS calling convention. Refer to the [Calling Convention](../getting_started.md#calling-convention) section for a primer.

---

## AD — Accumulation/Distribution

A cumulative indicator that uses price and volume to assess whether a security is being accumulated (bought) or distributed (sold).

**Inputs:** `[high, low, close, volume]` | **Options:** `[]` | **Outputs:** `[ad]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::ad::indicator;

    let high   = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low    = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // A/D line values

    // State continuation — feed new bars without reprocessing history
    let new_high   = vec![85.20_f64];
    let new_low    = vec![84.50_f64];
    let new_close  = vec![85.00_f64];
    let new_volume = vec![1550.0_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(),
          new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::ad::indicator_by_assets;

    let inputs: [&[&[f64]; 4]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice(), v4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.ad.indicator([high, low, close, volume], [])
    print(outputs[0])  # A/D line values

    # State continuation
    new_high   = np.array([85.20], dtype=np.float64)
    new_low    = np.array([84.50], dtype=np.float64)
    new_close  = np.array([85.00], dtype=np.float64)
    new_volume = np.array([1550.0], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close, new_volume])
    print(continued[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1, v1],
        [h2, l2, c2, v2],
        [h3, l3, c3, v3],
        [h4, l4, c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.ad.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.ad.indicator([high, low, close, volume], []);
    console.log('AD:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.ad.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued AD:', continued[0]);
    ```

    ### SIMD

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close], [...volume]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.ad.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## ADOSC — Accumulation/Distribution Oscillator

The difference between a short and long EMA of the A/D line, used to confirm price trends with volume.

**Inputs:** `[high, low, close, volume]` | **Options:** `[short_period, long_period]` | **Outputs:** `[adosc]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::adosc::indicator;

    let high   = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low    = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    // options: [short_period, long_period]
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[3.0, 10.0], None).unwrap();
    println!("{:?}", outputs[0]); // ADOSC values

    // State continuation — feed new bars without reprocessing history
    let new_high   = vec![85.20_f64];
    let new_low    = vec![84.50_f64];
    let new_close  = vec![85.00_f64];
    let new_volume = vec![1550.0_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(),
          new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### Optional Outputs

    `adosc` exposes 3 optional outputs: `short_ema`, `long_ema`, `ad`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::adosc::indicator;

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high   = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low    = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();
    let volume = vec![10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0_f64];

    let mask = [true, false, true];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()],
        &[6.0, 20.0],
        Some(&mask),
    ).unwrap();

    let adosc     = &outputs[0]; // adosc (primary)
    let short_ema = &outputs[1]; // short_ema (optional — requested)
    let long_ema  = &outputs[2]; // long_ema (optional — not requested)
    let ad        = &outputs[3]; // ad (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::adosc::indicator_by_assets;

    let inputs: [&[&[f64]; 4]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice(), v4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[3.0, 10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::adosc::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[2.0, 5.0], &[3.0, 10.0], &[5.0, 20.0], &[7.0, 28.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.adosc.indicator(
        [high, low, close, volume], [3.0, 10.0]
    )
    print(outputs[0])  # ADOSC values

    # State continuation
    new_high   = np.array([85.20], dtype=np.float64)
    new_low    = np.array([84.50], dtype=np.float64)
    new_close  = np.array([85.00], dtype=np.float64)
    new_volume = np.array([1550.0], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close, new_volume])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high   = close + 1.0
    low    = close - 1.0
    volume = np.array([10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.adosc.indicator(
        [high, low, close, volume], [6.0, 20.0],
        optional_outputs=[True, False, True],
    )

    adosc     = outputs[0]  # adosc (primary)
    short_ema = outputs[1]  # short_ema (optional — requested)
    long_ema  = outputs[2]  # long_ema (optional — not requested)
    ad        = outputs[3]  # ad (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1, v1],
        [h2, l2, c2, v2],
        [h3, l3, c3, v3],
        [h4, l4, c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.adosc.simd_by_assets(simd_inputs, [3.0, 10.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[2.0, 5.0], [3.0, 10.0], [5.0, 20.0], [7.0, 28.0]]
    outputs_list, states = tulip_rs.indicators.adosc.simd_by_options(
        [high, low, close, volume], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.adosc.indicator([high, low, close, volume], [3, 10]);
    console.log('ADOSC:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.adosc.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], [3, 10]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued ADOSC:', continued[0]);
    ```

    ### Optional Outputs

    `adosc` exposes 3 optional outputs: `short_ema`, `long_ema`, `ad`.

    ```javascript
    const [allOut] = ti.adosc.indicator([high, low, close, volume], [3, 10], [true, true, true]);
    const adosc    = allOut[0]; // primary
    const shortEma = allOut[1]; // optional 0: short_ema
    const longEma  = allOut[2]; // optional 1: long_ema
    const ad       = allOut[3]; // optional 2: ad
    ```

    ### SIMD

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close], [...volume]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.adosc.simdByAssets(simdInputs, [3, 10]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[2, 5], [3, 10], [5, 20], [7, 28]];
    const [results] = ti.adosc.simdByOptions([high, low, close, volume], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```

---

## OBV — On Balance Volume

Cumulative volume indicator: adds volume on up-days and subtracts on down-days. Divergence with price can signal reversals.

**Inputs:** `[real, volume]` | **Options:** `[]` | **Outputs:** `[obv]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::obv::indicator;

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    let inputs = [close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // OBV values

    // State continuation — feed new bars without reprocessing history
    let new_close  = vec![85.00_f64];
    let new_volume = vec![1550.0_f64];
    let continued = state.batch_indicator(
        &[new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::obv::indicator_by_assets;

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

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.obv.indicator([close, volume], [])
    print(outputs[0])  # OBV values

    # State continuation
    new_close  = np.array([85.00], dtype=np.float64)
    new_volume = np.array([1550.0], dtype=np.float64)
    continued = state.batch_indicator([new_close, new_volume])
    print(continued[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [c1, v1],
        [c2, v2],
        [c3, v3],
        [c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.obv.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.obv.indicator([close, volume], []);
    console.log('OBV:', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.obv.indicator([close.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([close.slice(n), volume.slice(n)]);
    console.log('Continued OBV:', continued[0]);
    ```

    ### SIMD

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...close], [...volume]],
        [close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.obv.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## MFI — Money Flow Index

A volume-weighted RSI. Values above 80 suggest overbought; below 20 oversold.

**Inputs:** `[high, low, close, volume]` | **Options:** `[period]` | **Outputs:** `[mfi]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::mfi::indicator;

    let high   = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low    = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("{:?}", outputs[0]); // MFI values

    // State continuation — feed new bars without reprocessing history
    let new_high   = vec![85.20_f64];
    let new_low    = vec![84.50_f64];
    let new_close  = vec![85.00_f64];
    let new_volume = vec![1550.0_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(),
          new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### Optional Outputs

    `mfi` exposes 1 optional output: `typprice`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::mfi::indicator;

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high   = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low    = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();
    let volume = vec![10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0_f64];

    let mask = [true];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()],
        &[14.0],
        Some(&mask),
    ).unwrap();

    let mfi      = &outputs[0]; // mfi (primary)
    let typprice = &outputs[1]; // typprice (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::mfi::indicator_by_assets;

    let inputs: [&[&[f64]; 4]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice(), v4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::mfi::indicator_by_options;

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Period {}: {:?}", opts[i][0], out[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.mfi.indicator([high, low, close, volume], [14.0])
    print(outputs[0])  # MFI values

    # State continuation
    new_high   = np.array([85.20], dtype=np.float64)
    new_low    = np.array([84.50], dtype=np.float64)
    new_close  = np.array([85.00], dtype=np.float64)
    new_volume = np.array([1550.0], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close, new_volume])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high   = close + 1.0
    low    = close - 1.0
    volume = np.array([10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.mfi.indicator(
        [high, low, close, volume], [14.0],
        optional_outputs=[True],
    )

    mfi      = outputs[0]  # mfi (primary)
    typprice = outputs[1]  # typprice (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1, v1],
        [h2, l2, c2, v2],
        [h3, l3, c3, v3],
        [h4, l4, c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.mfi.simd_by_assets(simd_inputs, [14.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.mfi.simd_by_options(
        [high, low, close, volume], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Period {simd_options[i][0]}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.mfi.indicator([high, low, close, volume], [14]);
    console.log('MFI(14):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.mfi.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], [14]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued MFI:', continued[0]);
    ```

    ### Optional Outputs

    `mfi` exposes 1 optional output: `typprice`.

    ```javascript
    const [allOut] = ti.mfi.indicator([high, low, close, volume], [14], [true]);
    const mfi      = allOut[0]; // primary
    const typprice = allOut[1]; // optional 0: typprice
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close], [...volume]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.mfi.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.mfi.simdByOptions([high, low, close, volume], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```

---

## NVI — Negative Volume Index

Tracks price changes on days when volume decreases, based on the theory that smart money trades on quiet days.

**Inputs:** `[real, volume]` | **Options:** `[]` | **Outputs:** `[nvi]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::nvi::indicator;

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    let inputs = [close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // NVI values

    // State continuation — feed new bars without reprocessing history
    let new_close  = vec![85.00_f64];
    let new_volume = vec![850.0_f64];
    let continued = state.batch_indicator(
        &[new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::nvi::indicator_by_assets;

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

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.nvi.indicator([close, volume], [])
    print(outputs[0])  # NVI values

    # State continuation
    new_close  = np.array([85.00], dtype=np.float64)
    new_volume = np.array([850.0], dtype=np.float64)
    continued = state.batch_indicator([new_close, new_volume])
    print(continued[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [c1, v1],
        [c2, v2],
        [c3, v3],
        [c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.nvi.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.nvi.indicator([close, volume], []);
    console.log('NVI:', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.nvi.indicator([close.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([close.slice(n), volume.slice(n)]);
    console.log('Continued NVI:', continued[0]);
    ```

    ### SIMD

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...close], [...volume]],
        [close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.nvi.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## PVI — Positive Volume Index

Tracks price changes on days when volume increases. Complements NVI.

**Inputs:** `[real, volume]` | **Options:** `[]` | **Outputs:** `[pvi]`

=== "Rust"

    ### Basic

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

    ### SIMD

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

    ### Basic

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

    ### SIMD

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

    ### Basic

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

---

## VOSC — Volume Oscillator

The percentage difference between two volume moving averages. Expanding volume oscillator supports the price trend.

**Inputs:** `[volume]` | **Options:** `[short_period, long_period]` | **Outputs:** `[vosc]`

=== "Rust"

    ### Basic

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

    ### Optional Outputs

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

    ### SIMD

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

    ### Basic

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

    ### Optional Outputs

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

    ### SIMD

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

    ### Basic

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

    ### Optional Outputs

    `vosc` exposes 2 optional outputs: `short_sma`, `long_sma`.

    ```javascript
    const [allOut] = ti.vosc.indicator([volume], [5, 10], [true, true]);
    const vosc     = allOut[0]; // primary
    const shortSma = allOut[1]; // optional 0: short_sma
    const longSma  = allOut[2]; // optional 1: long_sma
    ```

    ### SIMD

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

---

## KVO — Klinger Volume Oscillator

Identifies long-term money flow trends while remaining sensitive enough to detect short-term fluctuations.

**Inputs:** `[high, low, close, volume]` | **Options:** `[short_period, long_period]` | **Outputs:** `[kvo]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::kvo::indicator;

    let high   = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low    = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    // options: [short_period, long_period]
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[34.0, 55.0], None).unwrap();
    println!("{:?}", outputs[0]); // KVO values

    // State continuation — feed new bars without reprocessing history
    let new_high   = vec![85.20_f64];
    let new_low    = vec![84.50_f64];
    let new_close  = vec![85.00_f64];
    let new_volume = vec![1550.0_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(),
          new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### Optional Outputs

    `kvo` exposes 2 optional outputs: `short_ema`, `long_ema`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::kvo::indicator;

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high   = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low    = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();
    let volume = vec![10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0_f64];

    let mask = [true, false];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()],
        &[9.0, 26.0],
        Some(&mask),
    ).unwrap();

    let kvo       = &outputs[0]; // kvo (primary)
    let short_ema = &outputs[1]; // short_ema (optional — requested)
    let long_ema  = &outputs[2]; // long_ema (optional — not requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::kvo::indicator_by_assets;

    let inputs: [&[&[f64]; 4]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice(), v4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[34.0, 55.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::kvo::indicator_by_options;

    let opts: [&[f64; 2]; 4] = [&[13.0, 21.0], &[21.0, 34.0], &[34.0, 55.0], &[55.0, 89.0]];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    # options: [short_period, long_period]
    outputs, state = tulip_rs.indicators.kvo.indicator(
        [high, low, close, volume], [34.0, 55.0]
    )
    print(outputs[0])  # KVO values

    # State continuation
    new_high   = np.array([85.20], dtype=np.float64)
    new_low    = np.array([84.50], dtype=np.float64)
    new_close  = np.array([85.00], dtype=np.float64)
    new_volume = np.array([1550.0], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close, new_volume])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high   = close + 1.0
    low    = close - 1.0
    volume = np.array([10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.kvo.indicator(
        [high, low, close, volume], [9.0, 26.0],
        optional_outputs=[True, False],
    )

    kvo       = outputs[0]  # kvo (primary)
    short_ema = outputs[1]  # short_ema (optional — requested)
    long_ema  = outputs[2]  # long_ema (optional — not requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1, v1],
        [h2, l2, c2, v2],
        [h3, l3, c3, v3],
        [h4, l4, c4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.kvo.simd_by_assets(simd_inputs, [34.0, 55.0])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[13.0, 21.0], [21.0, 34.0], [34.0, 55.0], [55.0, 89.0]]
    outputs_list, states = tulip_rs.indicators.kvo.simd_by_options(
        [high, low, close, volume], simd_options
    )
    for i, out in enumerate(outputs_list):
        print(f"Option set {i+1}: {out[0]}")
    ```

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close  = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.kvo.indicator([high, low, close, volume], [34, 55]);
    console.log('KVO:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.kvo.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], [34, 55]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued KVO:', continued[0]);
    ```

    ### Optional Outputs

    `kvo` exposes 2 optional outputs: `short_ema`, `long_ema`.

    ```javascript
    const [allOut] = ti.kvo.indicator([high, low, close, volume], [34, 55], [true, true]);
    const kvo      = allOut[0]; // primary
    const shortEma = allOut[1]; // optional 0: short_ema
    const longEma  = allOut[2]; // optional 1: long_ema
    ```

    ### SIMD

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...high], [...low], [...close], [...volume]],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.kvo.simdByAssets(simdInputs, [34, 55]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[13, 21], [21, 34], [34, 55], [55, 89]];
    const [results] = ti.kvo.simdByOptions([high, low, close, volume], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1}:`, out[0]));
    ```

---

## EMV — Ease of Movement

Relates price change to volume, indicating how easily a price moves. High values suggest price is moving easily on low volume.

**Inputs:** `[high, low, volume]` | **Options:** `[]` | **Outputs:** `[emv]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::emv::indicator;

    let high   = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low    = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let volume = vec![1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                      900.0, 1500.0, 1800.0, 1000.0, 1700.0_f64];

    let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // EMV values

    // State continuation — feed new bars without reprocessing history
    let new_high   = vec![85.20_f64];
    let new_low    = vec![84.50_f64];
    let new_volume = vec![1550.0_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### Optional Outputs

    `emv` exposes 1 optional output: `medprice`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::emv::indicator;

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high   = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low    = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();
    let volume = vec![10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0_f64];

    let mask = [true];
    let (outputs, _state) = indicator(
        &[high.as_slice(), low.as_slice(), volume.as_slice()],
        &[],
        Some(&mask),
    ).unwrap();

    let emv      = &outputs[0]; // emv (primary)
    let medprice = &outputs[1]; // medprice (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::emv::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), v4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high   = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low    = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    volume = np.array([1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.emv.indicator([high, low, volume], [])
    print(outputs[0])  # EMV values

    # State continuation
    new_high   = np.array([85.20], dtype=np.float64)
    new_low    = np.array([84.50], dtype=np.float64)
    new_volume = np.array([1550.0], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_volume])
    print(continued[0])
    ```

    ### Optional Outputs

    ```python
    import numpy as np
    import tulip_rs

    close  = np.array([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    high   = close + 1.0
    low    = close - 1.0
    volume = np.array([10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.emv.indicator(
        [high, low, volume], [],
        optional_outputs=[True],
    )

    emv      = outputs[0]  # emv (primary)
    medprice = outputs[1]  # medprice (optional — requested)
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, v1],
        [h2, l2, v2],
        [h3, l3, v3],
        [h4, l4, v4],
    ]
    outputs_list, states = tulip_rs.indicators.emv.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low    = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.emv.indicator([high, low, volume], []);
    console.log('EMV:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.emv.indicator([high.slice(0, n), low.slice(0, n), volume.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), volume.slice(n)]);
    console.log('Continued EMV:', continued[0]);
    ```

    ### Optional Outputs

    `emv` exposes 1 optional output: `medprice`.

    ```javascript
    const [allOut] = ti.emv.indicator([high, low, volume], [], [true]);
    const emv      = allOut[0]; // primary
    const medprice = allOut[1]; // optional 0: medprice
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
    const [results] = ti.emv.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## WAD — Williams Accumulation/Distribution

A cumulative indicator that compares each close to the previous close to assess buying and selling pressure.

**Inputs:** `[high, low, close]` | **Options:** `[]` | **Outputs:** `[wad]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::wad::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, mut state) = indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]); // WAD values

    // State continuation — feed new bars without reprocessing history
    let new_high  = vec![85.20_f64];
    let new_low   = vec![84.50_f64];
    let new_close = vec![85.00_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(), new_close.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::wad::indicator_by_assets;

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];
    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.wad.indicator([high, low, close], [])
    print(outputs[0])  # WAD values

    # State continuation
    new_high  = np.array([85.20], dtype=np.float64)
    new_low   = np.array([84.50], dtype=np.float64)
    new_close = np.array([85.00], dtype=np.float64)
    continued = state.batch_indicator([new_high, new_low, new_close])
    print(continued[0])
    ```

    ### SIMD

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [
        [h1, l1, c1],
        [h2, l2, c2],
        [h3, l3, c3],
        [h4, l4, c4],
    ]
    outputs_list, states = tulip_rs.indicators.wad.simd_by_assets(simd_inputs, [])
    for i, asset_outputs in enumerate(outputs_list):
        print(f"Asset {i+1}: {asset_outputs[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    ### Basic

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.wad.indicator([high, low, close], []);
    console.log('WAD:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.wad.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued WAD:', continued[0]);
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
    const [results] = ti.wad.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
