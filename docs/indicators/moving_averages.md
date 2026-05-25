# Moving Averages

Moving averages smooth price series to reveal underlying trends and reduce noise. All of the indicators on this page accept a single `real` input (typically the closing price) unless otherwise noted.

---

## SMA — Simple Moving Average

The arithmetic mean of the last `period` values. The simplest and most widely used smoothing method.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[sma]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::sma::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Full computation
    let (outputs, _state) = indicator(&[close.as_slice()], &[5.0], None).unwrap();
    println!("SMA(5): {:?}", outputs[0]);

    // Partial computation + state continuation
    let partial = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99];
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[5.0], None).unwrap();
    println!("Partial SMA: {:?}", outputs2[0]);

    let new_close = vec![84.55, 84.36_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued SMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::sma::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[5.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::sma::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[50.0], &[100.0], &[200.0], &[300.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Full computation
    outputs, state = tulip_rs.indicators.sma.indicator([close], [5.0])
    print("SMA(5):", outputs[0])

    # Partial computation + state continuation
    partial = close[:-2]
    outputs2, state = tulip_rs.indicators.sma.indicator([partial], [5.0])
    print("Partial SMA:", outputs2[0])

    new_close = close[-2:]
    continued = state.batch_indicator([new_close])
    print("Continued SMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    a1 = close
    a2 = close + 5.0
    a3 = close - 5.0
    a4 = close * 1.02

    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.sma.simd_by_assets(simd_inputs, [5.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[50.0], [100.0], [200.0], [300.0]]
    outputs_list, states = tulip_rs.indicators.sma.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## EMA — Exponential Moving Average

Weighted moving average that gives more weight to recent prices via an exponential decay factor. Responds to new data faster than SMA.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[ema]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::ema::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("EMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial EMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued EMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ema::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::ema::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.ema.indicator([close], [14.0])
    print("EMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.ema.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued EMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.ema.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.ema.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## WMA — Weighted Moving Average

Moving average where each bar is weighted linearly, the most recent bar receiving the highest weight. Reacts faster than SMA but slower than EMA.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[wma]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::wma::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("WMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial WMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued WMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::wma::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::wma::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.wma.indicator([close], [14.0])
    print("WMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.wma.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued WMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.wma.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.wma.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## DEMA — Double Exponential Moving Average

Reduces EMA lag by applying a second EMA and combining the results: `2 * EMA - EMA(EMA)`. Tracks price more closely than a standard EMA of the same period.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[dema]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::dema::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("DEMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial DEMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued DEMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::dema::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::dema::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.dema.indicator([close], [14.0])
    print("DEMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.dema.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued DEMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.dema.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.dema.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## TEMA — Triple Exponential Moving Average

Further reduces lag with three EMA layers: `3 * EMA - 3 * EMA(EMA) + EMA(EMA(EMA))`. Reacts to price changes more quickly than DEMA at the cost of additional sensitivity to noise.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[tema]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::tema::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("TEMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial TEMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued TEMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::tema::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::tema::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.tema.indicator([close], [14.0])
    print("TEMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.tema.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued TEMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.tema.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.tema.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## TRIMA — Triangular Moving Average

A double-smoothed SMA (the SMA of an SMA), placing more weight on the middle of the lookback window and producing a very smooth output.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[trima]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::trima::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("TRIMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial TRIMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued TRIMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::trima::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::trima::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.trima.indicator([close], [14.0])
    print("TRIMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.trima.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued TRIMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.trima.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.trima.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## HMA — Hull Moving Average

Uses weighted moving averages of different periods to dramatically reduce lag while maintaining smoothness. Developed by Alan Hull.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[hma]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::hma::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("HMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial HMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued HMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::hma::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::hma::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.hma.indicator([close], [14.0])
    print("HMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.hma.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued HMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.hma.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.hma.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## ZLEMA — Zero Lag Exponential Moving Average

Adjusts the input data to compensate for EMA lag before applying the EMA, resulting in a moving average that closely tracks the current price without the typical delay.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[zlema]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::zlema::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("ZLEMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial ZLEMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued ZLEMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::zlema::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::zlema::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.zlema.indicator([close], [14.0])
    print("ZLEMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.zlema.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued ZLEMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.zlema.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.zlema.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## KAMA — Kaufman Adaptive Moving Average

Adapts its smoothing speed based on the market's efficiency ratio — fast-moving in trending markets and slow-moving in choppy, sideways markets.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[kama]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::kama::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("KAMA(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial KAMA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued KAMA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::kama::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::kama::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.kama.indicator([close], [14.0])
    print("KAMA(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.kama.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued KAMA:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.kama.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.kama.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## VIDYA — Variable Index Dynamic Average

Similar to KAMA but uses the Chande Momentum Oscillator as its efficiency measure. The three options control the short and long CMO periods and the base smoothing constant alpha.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[short_period, long_period, alpha]` &nbsp;|&nbsp; **Outputs:** `[vidya]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::vidya::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Options: [short_period, long_period, alpha]
    let (outputs, _state) = indicator(&[close.as_slice()], &[2.0, 5.0, 0.2], None).unwrap();
    println!("VIDYA: {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[2.0, 5.0, 0.2], None).unwrap();
    println!("Partial VIDYA: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued VIDYA: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::vidya::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[2.0, 5.0, 0.2], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::vidya::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[2.0, 5.0, 0.2],
        &[3.0, 7.0, 0.3],
        &[4.0, 9.0, 0.4],
        &[5.0, 11.0, 0.5],
    ];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Options: [short_period, long_period, alpha]
    outputs, state = tulip_rs.indicators.vidya.indicator([close], [2.0, 5.0, 0.2])
    print("VIDYA:", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.vidya.indicator([partial], [2.0, 5.0, 0.2])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued VIDYA:", continued[0])
    ```

    ### SIMD

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.vidya.simd_by_assets(simd_inputs, [2.0, 5.0, 0.2])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [2.0, 5.0, 0.2],
        [3.0, 7.0, 0.3],
        [4.0, 9.0, 0.4],
        [5.0, 11.0, 0.5],
    ]
    outputs_list, states = tulip_rs.indicators.vidya.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1}: {out[0]}")
    ```

---

## VWMA — Volume Weighted Moving Average

Moving average weighted by trading volume so that high-volume bars have more influence on the average than low-volume bars.

**Inputs:** `[real, volume]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[vwma]`

=== "Rust"

    ### Basic

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

    ### SIMD

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

    ### Basic

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

    ### SIMD

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

---

## Wilders — Wilder's Smoothing

The smoothing method developed by J. Welles Wilder, used internally by RSI, ATR, and ADX. Equivalent to an EMA with `alpha = 1 / period`. Provides a smooth, slowly-adapting average suitable for computing directional strength.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[wilders]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::wilders::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("Wilders(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial Wilders: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued Wilders: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::wilders::indicator_by_assets;

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![72.10, 72.85, 73.40, 73.00, 74.20, 74.85, 75.10, 75.60, 76.00, 76.50_f64];
    let a3 = vec![55.30, 55.80, 56.10, 56.40, 56.90, 57.20, 57.50, 57.80, 58.10, 58.40_f64];
    let a4 = vec![100.1, 100.5, 101.0, 101.3, 101.8, 102.0, 102.5, 103.0, 103.3, 103.8_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::wilders::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.wilders.indicator([close], [14.0])
    print("Wilders(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.wilders.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued Wilders:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.wilders.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.wilders.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```
