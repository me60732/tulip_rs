# Oscillators

Oscillators measure momentum and overbought/oversold conditions, typically oscillating between fixed bounds or around a centre line. Multi-input indicators require high, low, and/or close price arrays in addition to the close.

---

## RSI — Relative Strength Index

Measures the speed and magnitude of price changes, oscillating between 0 and 100. Values above 70 suggest overbought conditions; values below 30 suggest oversold conditions.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[rsi]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::rsi::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("RSI(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial RSI: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued RSI: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::rsi::indicator_by_assets;

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
    use tulip_rs::indicators::rsi::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];

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

    outputs, state = tulip_rs.indicators.rsi.indicator([close], [14.0])
    print("RSI(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.rsi.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued RSI:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.rsi.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.rsi.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## MACD — Moving Average Convergence Divergence

Shows the relationship between two EMAs of different periods. The histogram visualises the difference between the MACD line and its signal line, highlighting momentum shifts.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[fast_period, slow_period, signal_period]` &nbsp;|&nbsp; **Outputs:** `[macd, signal, histogram]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::macd::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Options: [fast_period, slow_period, signal_period]
    let (outputs, _state) = indicator(&[close.as_slice()], &[12.0, 26.0, 9.0], None).unwrap();
    println!("MACD line:  {:?}", outputs[0]);
    println!("Signal:     {:?}", outputs[1]);
    println!("Histogram:  {:?}", outputs[2]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[12.0, 26.0, 9.0], None).unwrap();
    println!("Partial MACD: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued MACD:      {:?}", continued[0]);
    println!("Continued Signal:    {:?}", continued[1]);
    println!("Continued Histogram: {:?}", continued[2]);
    ```

    ### SIMD

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::macd::indicator_by_assets;

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

    let results = indicator_by_assets::<4>(&inputs, &[12.0, 26.0, 9.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} MACD: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Signal: {:?}", i + 1, asset_outputs[1]);
        println!("Asset {} Histogram: {:?}", i + 1, asset_outputs[2]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::macd::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[6.0,  13.0,  5.0],
        &[12.0, 26.0,  9.0],
        &[19.0, 39.0, 14.0],
        &[24.0, 52.0, 18.0],
    ];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} MACD:      {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Signal:    {:?}", i + 1, opt_outputs[1]);
        println!("Option set {} Histogram: {:?}", i + 1, opt_outputs[2]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # Options: [fast_period, slow_period, signal_period]
    outputs, state = tulip_rs.indicators.macd.indicator([close], [12.0, 26.0, 9.0])
    print("MACD line: ", outputs[0])
    print("Signal:    ", outputs[1])
    print("Histogram: ", outputs[2])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.macd.indicator([partial], [12.0, 26.0, 9.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued MACD:      ", continued[0])
    print("Continued Signal:    ", continued[1])
    print("Continued Histogram: ", continued[2])
    ```

    ### SIMD

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.macd.simd_by_assets(simd_inputs, [12.0, 26.0, 9.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} MACD:      {out[0]}")
        print(f"Asset {i + 1} Signal:    {out[1]}")
        print(f"Asset {i + 1} Histogram: {out[2]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [6.0,  13.0,  5.0],
        [12.0, 26.0,  9.0],
        [19.0, 39.0, 14.0],
        [24.0, 52.0, 18.0],
    ]
    outputs_list, states = tulip_rs.indicators.macd.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} MACD:      {out[0]}")
        print(f"Option set {i + 1} Signal:    {out[1]}")
        print(f"Option set {i + 1} Histogram: {out[2]}")
    ```

---

## Stochastic Oscillator

Compares a security's closing price to its price range over a given period. %K is the raw stochastic value; %D is a smoothed moving average of %K.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[k_period, k_slowing_period, d_period]` &nbsp;|&nbsp; **Outputs:** `[stoch_k, stoch_d]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::stoch::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    // Options: [k_period, k_slowing_period, d_period]
    let (outputs, _state) = indicator(&inputs, &[14.0, 3.0, 3.0], None).unwrap();
    println!("Stoch %K: {:?}", outputs[0]);
    println!("Stoch %D: {:?}", outputs[1]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = indicator(&inputs2, &[14.0, 3.0, 3.0], None).unwrap();
    println!("Partial %K: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued %K: {:?}", continued[0]);
    println!("Continued %D: {:?}", continued[1]);
    ```

    ### SIMD

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::stoch::indicator_by_assets;

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    // Reuse the same data for assets 2–4 in this example
    let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0, 3.0, 3.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} %K: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} %D: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::stoch::indicator_by_options;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[5.0,  3.0, 3.0],
        &[9.0,  3.0, 3.0],
        &[14.0, 3.0, 3.0],
        &[21.0, 3.0, 3.0],
    ];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} %K: {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} %D: {:?}", i + 1, opt_outputs[1]);
    }
    ```

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

    # Options: [k_period, k_slowing_period, d_period]
    outputs, state = tulip_rs.indicators.stoch.indicator([high, low, close], [14.0, 3.0, 3.0])
    print("Stoch %K:", outputs[0])
    print("Stoch %D:", outputs[1])

    # State continuation
    outputs2, state = tulip_rs.indicators.stoch.indicator([high[:8], low[:8], close[:8]], [14.0, 3.0, 3.0])
    continued = state.batch_indicator([high[8:], low[8:], close[8:]])
    print("Continued %K:", continued[0])
    print("Continued %D:", continued[1])
    ```

    ### SIMD

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    h1, l1, c1 = high,        low,        close
    h2, l2, c2 = high + 0.5,  low + 0.5,  close + 0.5
    h3, l3, c3 = high - 0.5,  low - 0.5,  close - 0.5
    h4, l4, c4 = high * 1.01, low * 1.01, close * 1.01

    simd_inputs = [[h1, l1, c1], [h2, l2, c2], [h3, l3, c3], [h4, l4, c4]]
    outputs_list, states = tulip_rs.indicators.stoch.simd_by_assets(simd_inputs, [14.0, 3.0, 3.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} %K: {out[0]}")
        print(f"Asset {i + 1} %D: {out[1]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [5.0,  3.0, 3.0],
        [9.0,  3.0, 3.0],
        [14.0, 3.0, 3.0],
        [21.0, 3.0, 3.0],
    ]
    outputs_list, states = tulip_rs.indicators.stoch.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} %K: {out[0]}")
        print(f"Option set {i + 1} %D: {out[1]}")
    ```

---

## StochRSI — Stochastic RSI

Applies the Stochastic Oscillator formula to RSI values rather than price, producing an extremely sensitive momentum indicator that oscillates between 0 and 1.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[stochrsi]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::stochrsi::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("StochRSI(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial StochRSI: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued StochRSI: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::stochrsi::indicator_by_assets;

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
    use tulip_rs::indicators::stochrsi::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];

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

    outputs, state = tulip_rs.indicators.stochrsi.indicator([close], [14.0])
    print("StochRSI(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.stochrsi.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued StochRSI:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.stochrsi.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.stochrsi.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## Williams %R

Momentum indicator measuring the current close relative to the highest high over `period` bars, scaled to a range of -100 to 0. Values near 0 indicate overbought conditions; values near -100 indicate oversold conditions.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[willr]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::willr::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[14.0], None).unwrap();
    println!("Williams %R(14): {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = indicator(&inputs2, &[14.0], None).unwrap();
    println!("Partial Williams %R: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued Williams %R: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::willr::indicator_by_assets;

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::willr::indicator_by_options;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

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

    outputs, state = tulip_rs.indicators.willr.indicator([high, low, close], [14.0])
    print("Williams %R(14):", outputs[0])

    # State continuation
    outputs2, state = tulip_rs.indicators.willr.indicator([high[:8], low[:8], close[:8]], [14.0])
    continued = state.batch_indicator([high[8:], low[8:], close[8:]])
    print("Continued Williams %R:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [
        [high,        low,        close],
        [high + 0.5,  low + 0.5,  close + 0.5],
        [high - 0.5,  low - 0.5,  close - 0.5],
        [high * 1.01, low * 1.01, close * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.willr.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.willr.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## CCI — Commodity Channel Index

Measures how far the typical price deviates from its simple moving average, normalised by mean absolute deviation. Values above +100 suggest overbought conditions; values below -100 suggest oversold conditions.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[cci]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::cci::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[20.0], None).unwrap();
    println!("CCI(20): {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = indicator(&inputs2, &[20.0], None).unwrap();
    println!("Partial CCI: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued CCI: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::cci::indicator_by_assets;

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[20.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::cci::indicator_by_options;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[10.0], &[14.0], &[20.0], &[30.0]];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

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

    outputs, state = tulip_rs.indicators.cci.indicator([high, low, close], [20.0])
    print("CCI(20):", outputs[0])

    # State continuation
    outputs2, state = tulip_rs.indicators.cci.indicator([high[:8], low[:8], close[:8]], [20.0])
    continued = state.batch_indicator([high[8:], low[8:], close[8:]])
    print("Continued CCI:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [
        [high,        low,        close],
        [high + 0.5,  low + 0.5,  close + 0.5],
        [high - 0.5,  low - 0.5,  close - 0.5],
        [high * 1.01, low * 1.01, close * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.cci.simd_by_assets(simd_inputs, [20.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[10.0], [14.0], [20.0], [30.0]]
    outputs_list, states = tulip_rs.indicators.cci.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## CMO — Chande Momentum Oscillator

Calculates momentum as the difference between the sum of gains and the sum of losses over `period` bars, scaled by their total. Oscillates between -100 and +100.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[cmo]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::cmo::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("CMO(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial CMO: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued CMO: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::cmo::indicator_by_assets;

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
    use tulip_rs::indicators::cmo::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[7.0], &[14.0], &[21.0], &[28.0]];

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

    outputs, state = tulip_rs.indicators.cmo.indicator([close], [14.0])
    print("CMO(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.cmo.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued CMO:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.cmo.simd_by_assets(simd_inputs, [14.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.cmo.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## Ultimate Oscillator

Combines momentum from three different time periods (short, medium, and long) to reduce the false signals that arise from using any single timeframe alone.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[short_period, medium_period, long_period]` &nbsp;|&nbsp; **Outputs:** `[ultosc]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::ultosc::indicator;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // Options: [short_period, medium_period, long_period]
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[7.0, 14.0, 28.0], None).unwrap();
    println!("Ultimate Oscillator: {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = indicator(&inputs2, &[7.0, 14.0, 28.0], None).unwrap();
    println!("Partial Ultimate Oscillator: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued Ultimate Oscillator: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ultosc::indicator_by_assets;

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let c1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let h2 = h1.clone(); let l2 = l1.clone(); let c2 = c1.clone();
    let h3 = h1.clone(); let l3 = l1.clone(); let c3 = c1.clone();
    let h4 = h1.clone(); let l4 = l1.clone(); let c4 = c1.clone();

    let inputs: [&[&[f64]; 3]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[7.0, 14.0, 28.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::ultosc::indicator_by_options;

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 3]; 4] = [
        &[7.0,  14.0, 28.0],
        &[5.0,  10.0, 20.0],
        &[10.0, 20.0, 40.0],
        &[4.0,  8.0,  16.0],
    ];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

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

    # Options: [short_period, medium_period, long_period]
    outputs, state = tulip_rs.indicators.ultosc.indicator([high, low, close], [7.0, 14.0, 28.0])
    print("Ultimate Oscillator:", outputs[0])

    # State continuation
    outputs2, state = tulip_rs.indicators.ultosc.indicator([high[:8], low[:8], close[:8]], [7.0, 14.0, 28.0])
    continued = state.batch_indicator([high[8:], low[8:], close[8:]])
    print("Continued Ultimate Oscillator:", continued[0])
    ```

    ### SIMD

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [
        [high,        low,        close],
        [high + 0.5,  low + 0.5,  close + 0.5],
        [high - 0.5,  low - 0.5,  close - 0.5],
        [high * 1.01, low * 1.01, close * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.ultosc.simd_by_assets(simd_inputs, [7.0, 14.0, 28.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [
        [7.0,  14.0, 28.0],
        [5.0,  10.0, 20.0],
        [10.0, 20.0, 40.0],
        [4.0,  8.0,  16.0],
    ]
    outputs_list, states = tulip_rs.indicators.ultosc.simd_by_options([high, low, close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1}: {out[0]}")
    ```

---

## AO — Awesome Oscillator

Measures market momentum as the difference between a 5-period and 34-period simple moving average of each bar's midpoint `(high + low) / 2`. No options are required.

**Inputs:** `[high, low]` &nbsp;|&nbsp; **Options:** `[]` (none) &nbsp;|&nbsp; **Outputs:** `[ao]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::ao::indicator;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                    90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                    93.80, 94.20, 94.60, 95.00, 95.30_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                    89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                    93.10, 93.50, 93.90, 94.30, 94.60_f64];

    // AO takes no options — pass an empty slice
    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[], None).unwrap();
    println!("AO: {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..30], &low[..30]];
    let (outputs2, mut state) = indicator(&inputs2, &[], None).unwrap();
    println!("Partial AO: {:?}", outputs2[0]);

    let new_inputs = [&high[30..], &low[30..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued AO: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::ao::indicator_by_assets;

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                  85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                  90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                  93.80, 94.20, 94.60, 95.00, 95.30_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                  84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                  89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                  93.10, 93.50, 93.90, 94.30, 94.60_f64];
    let h2 = h1.clone(); let l2 = l1.clone();
    let h3 = h1.clone(); let l3 = l1.clone();
    let h4 = h1.clone(); let l4 = l1.clone();

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    # AO requires at least 34 bars (34-period SMA)
    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                     90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                     93.80, 94.20, 94.60, 95.00, 95.30], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60], dtype=np.float64)

    # AO takes no options — pass an empty list
    outputs, state = tulip_rs.indicators.ao.indicator([high, low], [])
    print("AO:", outputs[0])

    # State continuation
    outputs2, state = tulip_rs.indicators.ao.indicator([high[:30], low[:30]], [])
    continued = state.batch_indicator([high[30:], low[30:]])
    print("Continued AO:", continued[0])
    ```

    ### SIMD

    **By assets** — applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87, 88.10, 88.50, 89.00, 89.40, 89.80,
                     90.10, 90.50, 91.00, 91.50, 91.80, 92.00, 92.40, 92.80, 93.10, 93.50,
                     93.80, 94.20, 94.60, 95.00, 95.30], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01, 87.50, 87.90, 88.30, 88.70, 89.10,
                     89.40, 89.80, 90.20, 90.60, 91.00, 91.30, 91.70, 92.10, 92.40, 92.80,
                     93.10, 93.50, 93.90, 94.30, 94.60], dtype=np.float64)

    simd_inputs = [
        [high,        low],
        [high + 0.5,  low + 0.5],
        [high - 0.5,  low - 0.5],
        [high * 1.01, low * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.ao.simd_by_assets(simd_inputs, [])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1}: {out[0]}")
    ```

    _This indicator has no options, so by-options SIMD does not apply._

---

## Fisher Transform

Converts prices into a Gaussian normal distribution. Sharp moves in the Fisher value can signal potential price reversals; the signal line is a one-bar lag of the Fisher line.

**Inputs:** `[high, low]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[fisher, fisher_signal]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::fisher::indicator;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, _state) = indicator(&inputs, &[10.0], None).unwrap();
    println!("Fisher:        {:?}", outputs[0]);
    println!("Fisher Signal: {:?}", outputs[1]);

    // State continuation
    let inputs2 = [&high[..10], &low[..10]];
    let (outputs2, mut state) = indicator(&inputs2, &[10.0], None).unwrap();
    println!("Partial Fisher: {:?}", outputs2[0]);

    let new_inputs = [&high[10..], &low[10..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued Fisher:        {:?}", continued[0]);
    println!("Continued Fisher Signal: {:?}", continued[1]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::fisher::indicator_by_assets;

    let h1 = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                  85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let l1 = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                  84.03, 85.39, 85.76, 87.17, 87.01_f64];
    let h2 = h1.clone(); let l2 = l1.clone();
    let h3 = h1.clone(); let l3 = l1.clone();
    let h4 = h1.clone(); let l4 = l1.clone();

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];

    let results = indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} Fisher:        {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Fisher Signal: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::fisher::indicator_by_options;

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                    85.90, 86.58, 86.98, 88.00, 87.87_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                    84.03, 85.39, 85.76, 87.17, 87.01_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];
    let inputs = [high.as_slice(), low.as_slice()];
    let results = indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} Fisher:        {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Fisher Signal: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    outputs, state = tulip_rs.indicators.fisher.indicator([high, low], [10.0])
    print("Fisher:        ", outputs[0])
    print("Fisher Signal: ", outputs[1])

    # State continuation
    outputs2, state = tulip_rs.indicators.fisher.indicator([high[:10], low[:10]], [10.0])
    continued = state.batch_indicator([high[10:], low[10:]])
    print("Continued Fisher:        ", continued[0])
    print("Continued Fisher Signal: ", continued[1])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    simd_inputs = [
        [high,        low],
        [high + 0.5,  low + 0.5],
        [high - 0.5,  low - 0.5],
        [high * 1.01, low * 1.01],
    ]
    outputs_list, states = tulip_rs.indicators.fisher.simd_by_assets(simd_inputs, [10.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Fisher:        {out[0]}")
        print(f"Asset {i + 1} Fisher Signal: {out[1]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    high = np.array([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00,
                     85.90, 86.58, 86.98, 88.00, 87.87], dtype=np.float64)
    low  = np.array([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11,
                     84.03, 85.39, 85.76, 87.17, 87.01], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.fisher.simd_by_options([high, low], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} Fisher:        {out[0]}")
        print(f"Option set {i + 1} Fisher Signal: {out[1]}")
    ```

---

## FOSC — Forecast Oscillator

Measures the percentage difference between the current price and the linear regression forecast value for that bar, showing how much prices deviate from their projected trend.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[fosc]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::fosc::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("FOSC(14): {:?}", outputs[0]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("Partial FOSC: {:?}", outputs2[0]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued FOSC: {:?}", continued[0]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::fosc::indicator_by_assets;

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
    use tulip_rs::indicators::fosc::indicator_by_options;

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

    outputs, state = tulip_rs.indicators.fosc.indicator([close], [14.0])
    print("FOSC(14):", outputs[0])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.fosc.indicator([partial], [14.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued FOSC:", continued[0])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.fosc.simd_by_assets(simd_inputs, [14.0])
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
    outputs_list, states = tulip_rs.indicators.fosc.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Period set {i + 1}: {out[0]}")
    ```

---

## MSW — Mesa Sine Wave

Fits a sine wave to the recent price data over `period` bars. The crossover of the sine and lead (phase-advanced) lines can signal cycle turns and potential entry/exit points.

**Inputs:** `[real]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[msw_sine, msw_lead]`

=== "Rust"

    ### Basic

    ```rust
    use tulip_rs::indicators::msw::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let (outputs, _state) = indicator(&[close.as_slice()], &[10.0], None).unwrap();
    println!("MSW Sine: {:?}", outputs[0]);
    println!("MSW Lead: {:?}", outputs[1]);

    // State continuation
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = indicator(&[partial.as_slice()], &[10.0], None).unwrap();
    println!("Partial MSW Sine: {:?}", outputs2[0]);
    println!("Partial MSW Lead: {:?}", outputs2[1]);

    let new_close = close[8..].to_vec();
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued MSW Sine: {:?}", continued[0]);
    println!("Continued MSW Lead: {:?}", continued[1]);
    ```

    ### SIMD

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::msw::indicator_by_assets;

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

    let results = indicator_by_assets::<4>(&inputs, &[10.0], None).unwrap();
    for (i, asset_outputs) in results.0.iter().enumerate() {
        println!("Asset {} Sine: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} Lead: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::msw::indicator_by_options;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[5.0], &[10.0], &[14.0], &[20.0]];

    let results = indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.0.iter().enumerate() {
        println!("Option set {} Sine: {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} Lead: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "Python"

    ### Basic

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.msw.indicator([close], [10.0])
    print("MSW Sine:", outputs[0])
    print("MSW Lead:", outputs[1])

    # State continuation
    partial = close[:8]
    outputs2, state = tulip_rs.indicators.msw.indicator([partial], [10.0])
    new_close = close[8:]
    continued = state.batch_indicator([new_close])
    print("Continued MSW Sine:", continued[0])
    print("Continued MSW Lead:", continued[1])
    ```

    ### SIMD

    **By assets** — same period applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 5.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.msw.simd_by_assets(simd_inputs, [10.0])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} Sine: {out[0]}")
        print(f"Asset {i + 1} Lead: {out[1]}")
    ```

    **By options** — same asset, N different periods in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    simd_options = [[5.0], [10.0], [14.0], [20.0]]
    outputs_list, states = tulip_rs.indicators.msw.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} Sine: {out[0]}")
        print(f"Option set {i + 1} Lead: {out[1]}")
    ```
