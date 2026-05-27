# VWMA — Volume Weighted Moving Average

Moving average weighted by trading volume so that high-volume bars have more influence on the average than low-volume bars.

**Inputs:** `[real, volume]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[vwma]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::vwma::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                      3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0_f64];

    let inputs = [close.as_slice(), volume.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("VWMA(14): {:?}", outputs[0]);

    // State continuation
    let partial_close  = close[..8].to_vec();
    let partial_volume = volume[..8].to_vec();
    let inputs2 = [partial_close.as_slice(), partial_volume.as_slice()];
    let (outputs2, mut state) = indicator(&inputs2, &[14.0], None).unwrap();
    println!("Partial VWMA: {:?}", outputs2[0]);

    let new_close  = close[8..].to_vec();
    let new_volume = volume[8..].to_vec();
    let new_inputs = [new_close.as_slice(), new_volume.as_slice()];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued VWMA: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                       3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0], dtype=np.float64)

    outputs, state = tulip_rs.indicators.vwma.indicator([close, volume], [14.0])
    print("VWMA(14):", outputs[0])

    # State continuation
    partial_close  = close[:8]
    partial_volume = volume[:8]
    outputs2, state = tulip_rs.indicators.vwma.indicator([partial_close, partial_volume], [14.0])
    new_close  = close[8:]
    new_volume = volume[8:]
    continued = state.batch_indicator([new_close, new_volume])
    print("Continued VWMA:", continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close  = [81.59, 81.06, 82.87, 83.00, 83.61,
                    83.15, 82.84, 83.99, 84.55, 84.36,
                    85.53, 86.54, 86.89, 87.77, 87.29];
    const volume = [5653100, 6447400, 7690900, 3831400, 4455100,
                    3798000, 3936200, 4732000, 4841300, 3915300,
                    6830800, 6694100, 5293600, 7985800, 4807900];

    const [outputs, state] = ti.vwma.indicator([close, volume], [14]);
    console.log('VWMA(14):', outputs[0]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.vwma.indicator([close.slice(0, n), volume.slice(0, n)], [14]);
    const continued = state2.batchIndicator([close.slice(n), volume.slice(n)]);
    console.log('Continued VWMA:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets (each with close + volume) in parallel:

    ```rust
    use tulip_rs::indicators::vwma::indicator_by_assets;

    let a1_close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a1_vol    = vec![5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                         3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0_f64];
    let a2_close  = a1_close.iter().map(|x| x + 5.0).collect::<Vec<_>>();
    let a2_vol    = a1_vol.clone();
    let a3_close  = a1_close.iter().map(|x| x - 5.0).collect::<Vec<_>>();
    let a3_vol    = a1_vol.clone();
    let a4_close  = a1_close.iter().map(|x| x * 1.02).collect::<Vec<_>>();
    let a4_vol    = a1_vol.clone();

    let inputs: [&[&[f64]; 2]; 4] = [
        &[a1_close.as_slice(), a1_vol.as_slice()],
        &[a2_close.as_slice(), a2_vol.as_slice()],
        &[a3_close.as_slice(), a3_vol.as_slice()],
        &[a4_close.as_slice(), a4_vol.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::vwma::indicator_by_options;

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let volume = vec![5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                      3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice(), volume.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                       3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0], dtype=np.float64)

    a1_close, a1_vol = close,          volume
    a2_close, a2_vol = close + 5.0,    volume
    a3_close, a3_vol = close - 5.0,    volume
    a4_close, a4_vol = close * 1.02,   volume

    simd_inputs = [
        [a1_close, a1_vol],
        [a2_close, a2_vol],
        [a3_close, a3_vol],
        [a4_close, a4_vol],
    ]
    outputs_list, states = tulip_rs.indicators.vwma.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)
    volume = np.array([5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0,
                       3798000.0, 3936200.0, 4732000.0, 4841300.0, 3915300.0], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.vwma.simd_by_options([close, volume], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [[...close], [...volume]],
        [close.map(v => v * 1.1), volume.map(v => v * 1.1)],
        [close.map(v => v * 0.9), volume.map(v => v * 0.9)],
        [close.map(v => v * 1.02), volume.map(v => v * 1.02)],
    ];
    const [results] = ti.vwma.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[5], [10], [14], [20]];
    const [results] = ti.vwma.simdByOptions([close, volume], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
