# State Management

## What is State?

Every TulipRS indicator returns **two values**: its output series and an `IndicatorState`. The state is a compact, serialisable snapshot of everything the indicator needs to continue computing on new bars — internal buffers, ring queues, running sums, and the current output index. Because state is fully serialisable (via `serde`), it can be stored to disk, transmitted over the network, or embedded in a database and restored later.

This design makes TulipRS well-suited for **streaming** and **incremental** pipelines: process history once, save the state, then cheaply append new bars as they arrive — without ever reprocessing the historical data.

---

## Basic Pattern

=== "Rust"

    ```rust
    use tulip_rs::indicators::sma::indicator;

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    // --- Step 1: compute on historical data, capture state ---
    let n = 8; // process first 8 bars
    let (outputs, mut state) = indicator(&[&close[..n]], &[5.0], None).unwrap();
    println!("History outputs: {:?}", outputs[0]);

    // --- Step 2: feed new bars via state.batch_indicator ---
    let new_close = vec![85.53_f64, 86.54];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("Continued outputs: {:?}", continued[0]);
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    # --- Step 1: compute on historical data, capture state ---
    n = 8  # process first 8 bars
    outputs, state = tulip_rs.indicators.sma.indicator([close[:n]], [5.0])
    print("History outputs:", outputs[0])

    # --- Step 2: feed new bars via state.batch_indicator ---
    new_close = np.array([85.53, 86.54], dtype=np.float64)
    continued = state.batch_indicator([new_close])
    print("Continued outputs:", continued[0])
    ```

!!! note
    `batch_indicator` accepts **new bars only** — it does not want the full history. Pass only the bars that arrived since the last call.

---

## Chunked Processing

For very long historical series, chunked processing lets you control memory usage by processing data in fixed-size windows:

=== "Rust"

    ```rust
    use tulip_rs::indicators::sma::indicator;

    let close: Vec<f64> = /* ... very long series ... */ vec![];
    let chunk_size = 500;
    let period = 5.0;

    // Seed on the first chunk
    let (mut all_outputs, mut state) =
        indicator(&[&close[..chunk_size]], &[period], None).unwrap();

    // Continue chunk by chunk
    for chunk in close[chunk_size..].chunks(chunk_size) {
        let result = state.batch_indicator(&[chunk], None).unwrap();
        all_outputs[0].extend_from_slice(&result[0]);
    }

    println!("Total output bars: {}", all_outputs[0].len());
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close: np.ndarray = np.array([...], dtype=np.float64)  # very long series
    chunk_size = 500
    period = 5.0

    # Seed on the first chunk
    outputs, state = tulip_rs.indicators.sma.indicator([close[:chunk_size]], [period])
    all_sma = list(outputs[0])

    # Continue chunk by chunk
    for start in range(chunk_size, len(close), chunk_size):
        chunk = close[start : start + chunk_size]
        result = state.batch_indicator([chunk])
        all_sma.extend(result[0])

    print(f"Total output bars: {len(all_sma)}")
    ```

---

## JSON Serialisation

State can be serialised to JSON for persistence and restored later. This is useful for saving indicator state to a database or cache.

=== "Rust"

    ```rust
    // Serialise
    let json = serde_json::to_string(&state).unwrap();

    // Persist json to disk / database ...

    // Restore
    let mut restored: IndicatorState = serde_json::from_str(&json).unwrap();

    // Continue from restored state
    let new_bars = vec![87.10_f64, 88.25];
    let result = restored.batch_indicator(&[new_bars.as_slice()], None).unwrap();
    ```

    Add `serde_json` to your `Cargo.toml`:

    ```toml
    [dependencies]
    serde_json = "1"
    ```

=== "Python"

    ```python
    # Serialise
    json_str = state.state_to_json()        # returns Optional[str]

    # Persist json_str to disk / database ...

    # Restore — use the indicator's restore function
    restored_state = tulip_rs.indicators.sma.restore_state(json_str)

    # Continue from restored state
    new_bars = np.array([87.10, 88.25], dtype=np.float64)
    result = restored_state.batch_indicator([new_bars])
    ```

!!! warning "State is indicator-, option-, and asset-specific"
    - **Indicator-specific** — a serialised state from `sma` cannot be loaded as a state for `ema` or any other indicator. Always restore into the same indicator type that produced the JSON.
    - **Option-specific** — the options used when the state was created are baked into the state. An EMA state created with `period=10` will always compute as a period-10 EMA. If you need a different period, run a fresh `indicator` call.
    - **Asset-specific** — a state captures the internal buffers for one particular price series. You cannot reuse the same state object to continue computation on a different asset.

---

## Multi-Output Indicators

State works identically for indicators with multiple output series. Bollinger Bands, for example, returns three outputs (lower band, middle band, upper band):

=== "Rust"

    ```rust
    use tulip_rs::indicators::bbands::indicator;

    // options: [period, stddev_multiplier]
    let (outputs, mut state) = indicator(&[&close[..n]], &[20.0, 2.0], None).unwrap();

    let lower  = &outputs[0];
    let middle = &outputs[1];
    let upper  = &outputs[2];

    // Continue — all three output series are extended together
    let continued = state.batch_indicator(&[&new_close], None).unwrap();
    let new_lower  = &continued[0];
    let new_middle = &continued[1];
    let new_upper  = &continued[2];
    ```

=== "Python"

    ```python
    import tulip_rs

    # options: [period, stddev_multiplier]
    outputs, state = tulip_rs.indicators.bbands.indicator([close[:n]], [20.0, 2.0])

    lower  = outputs[0]
    middle = outputs[1]
    upper  = outputs[2]

    # Continue — all three output series are returned together
    continued = state.batch_indicator([new_close])
    new_lower  = continued[0]
    new_middle = continued[1]
    new_upper  = continued[2]
    ```

---

## State vs Full Recalculation

| Scenario | Recommendation |
|---|---|
| One-off analysis of a fixed dataset | Full recalculation — simpler code |
| Live feed appending 1–N bars at a time | **State** — avoids O(n) reprocessing each tick |
| Parameter sweep over many option sets | Full recalculation or SIMD by-options |
| Resuming after a process restart | **State + JSON serialisation** |
| Distributing computation across machines | **State + JSON serialisation** |
| Very long history, fixed period | Chunked processing with state |
