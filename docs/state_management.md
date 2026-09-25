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

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[SMA_OPTIONS] = {5.0}; // period
    const double *inputs[SMA_INPUTS] = {close};

    /* Step 1: seed on historical data */
    CIndicatorResult r = sma_indicator(inputs, 8, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "sma_indicator failed: error=%d\n", r.error);
        return 1;
    }
    printf("History outputs[0]: [");
    for (uintptr_t i = 0; i < r.output_lens[0]; i++) {
        printf("%.4f", r.outputs[0][i]);
        if (i + 1 < r.output_lens[0]) printf(", ");
    }
    printf("]\n");

    void *state = r.state;
    tulip_ffi_result_free(r); /* outputs freed; state kept alive */

    /* Step 2: append new bars */
    double new_close[] = {85.53, 86.54};
    const double *new_inputs[SMA_INPUTS] = {new_close};
    CBatchResult b = sma_batch(state, new_inputs, 2, NULL, 0);
    if (b.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "sma_batch failed: error=%d\n", b.error);
        return 1;
    }
    printf("Continued outputs[0]: [");
    for (uintptr_t i = 0; i < b.output_lens[0]; i++) {
        printf("%.4f", b.outputs[0][i]);
        if (i + 1 < b.output_lens[0]) printf(", ");
    }
    printf("]\n");

    tulip_ffi_batch_result_free(b);
    sma_state_free(state); /* final cleanup after last batch */
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36}

    // --- Step 1: compute on historical data, capture state ---
    n := 8 // process first 8 bars
    res, st, err := indicators.Sma.Indicator(close[:n], []float64{5.0}, nil)
    if err != nil {
        fmt.Printf("sma_indicator failed: %v\n", err)
        return
    }
    defer res.Close()
    defer st.Close()

    fmt.Println("History outputs[0]:", tulip.AsFloat64(res.Rows[0]))

    // --- Step 2: feed new bars via state.Batch ---
    newClose := []float64{85.53, 86.54}
    continued, err := st.Batch(newClose, nil)
    if err != nil {
        fmt.Printf("sma_batch failed: %v\n", err)
        return
    }
    defer continued.Close()

    fmt.Println("Continued outputs[0]:", tulip.AsFloat64(continued.Rows[0]))
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Sma;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    // --- Step 1: compute on historical data, capture state ---
    int n = 8; // process first 8 bars
    Outcome oc = Sma.indicator(new double[][] {close}, new double[] {5.0});
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println("History outputs[0]: " + java.util.Arrays.toString(res.toDoubleArray(0)));
    }

    // --- Step 2: feed new bars via state.batch ---
    double[] newClose = {85.53, 86.54};
    Outcome p = Sma.indicator(new double[][] {java.util.Arrays.copyOfRange(close, 0, n)}, new double[] {5.0});
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {newClose});
        try (br) {
            System.out.println("Continued outputs[0]: " + java.util.Arrays.toString(br.toDoubleArray(0)));
        }
    }
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

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                                     83.15, 82.84, 83.99, 84.55, 84.36]);

    // Step 1: compute on historical data, capture state
    const n = 8;
    const [outputs, state] = ti.sma.indicator([close.slice(0, n)], [5]);
    console.log('History outputs:', outputs[0]);

    // Step 2: feed new bars via state.batchIndicator
    const newClose = Float64Array.from([85.53, 86.54]);
    const continued = state.batchIndicator([newClose]);
    console.log('Continued outputs:', continued[0]);
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

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    /* Assume close[] is a very long series */
    size_t chunk_size = 500;
    double options[SMA_OPTIONS] = {5.0}; // period

    /* Seed on the first chunk */
    CIndicatorResult r = sma_indicator(inputs, chunk_size, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "sma_indicator failed: error=%d\n", r.error);
        return 1;
    }

    void *state = r.state;
    uintptr_t first_len = r.output_lens[0]; /* read before freeing the result */
    tulip_ffi_result_free(r);

    /* Continue chunk by chunk */
    for (size_t start = chunk_size; start < total_len; start += chunk_size) {
        size_t this_chunk = (start + chunk_size <= total_len) ? chunk_size : total_len - start;
        const double *chunk_inputs[SMA_INPUTS] = {close + start};
        CBatchResult b = sma_batch(state, chunk_inputs, this_chunk, NULL, 0);
        if (b.error != C_INDICATOR_ERROR_OK) {
            fprintf(stderr, "sma_batch failed: error=%d\n", b.error);
            return 1;
        }

        /* Copy or process outputs[0] — it will be freed by tulip_ffi_batch_result_free */
        memcpy(all_outputs + (start - chunk_size + first_len),
               b.outputs[0], b.output_lens[0] * sizeof(double));

        tulip_ffi_batch_result_free(b);
    }

    sma_state_free(state); /* final cleanup after last batch */
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    close := /* ... very long series ... */ []float64{}
    chunkSize := 500
    period := 5.0

    // Seed on the first chunk
    res, st, err := indicators.Sma.Indicator(close[:chunkSize], []float64{period}, nil)
    if err != nil {
        fmt.Printf("sma_indicator failed: %v\n", err)
        return
    }
    defer res.Close()
    defer st.Close()

    allSMA := append([]float64(nil), tulip.AsFloat64(res.Rows[0])...)

    // Continue chunk by chunk
    for start := chunkSize; start < len(close); start += chunkSize {
        thisChunk := start + chunkSize
        if thisChunk > len(close) {
            thisChunk = len(close)
        }
        br, err := st.Batch(close[start:thisChunk], nil)
        if err != nil {
            fmt.Printf("sma_batch failed: %v\n", err)
            return
        }
        defer br.Close()

        // Copy rows out via tulip.AsFloat64 before Close (rows invalid after Close)
        allSMA = append(allSMA, tulip.AsFloat64(br.Rows[0])...)
    }

    fmt.Printf("Total output bars: %d\n", len(allSMA))
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Sma;

    double[] close = /* ... very long series ... */ new double[0];
    int chunkSize = 500;
    double period = 5.0;

    // Seed on the first chunk
    Outcome oc = Sma.indicator(new double[][] {java.util.Arrays.copyOfRange(close, 0, chunkSize)}, new double[] {period});
    try (Result res = oc.result(); State st = oc.state()) {
        double[] firstChunkOutput = res.toDoubleArray(0);
        System.out.println("First chunk output length: " + firstChunkOutput.length);
    }

    // Continue chunk by chunk
    for (int start = chunkSize; start < close.length; start += chunkSize) {
        int thisChunk = start + chunkSize;
        if (thisChunk > close.length) {
            thisChunk = close.length;
        }
        Outcome p = Sma.indicator(new double[][] {java.util.Arrays.copyOfRange(close, 0, chunkSize)}, new double[] {period});
        try (Result pr = p.result(); State st = p.state()) {
            Result br = st.batch(new double[][] {java.util.Arrays.copyOfRange(close, start, thisChunk)});
            try (br) {
                System.out.println("Chunk " + ((start/chunkSize)+1) + " output: " + java.util.Arrays.toString(br.toDoubleArray(0)));
            }
        }
    }

    System.out.println("Total output bars computed");
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

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = new Float64Array(/* very long series */);
    const chunkSize = 500;
    const period = 5;

    // Seed on the first chunk
    const [outputs, state] = ti.sma.indicator([close.slice(0, chunkSize)], [period]);
    const allSma = [...outputs[0]];

    // Continue chunk by chunk
    for (let start = chunkSize; start < close.length; start += chunkSize) {
        const chunk = close.slice(start, start + chunkSize);
        const result = state.batchIndicator([chunk]);
        allSma.push(...result[0]);
    }

    console.log(`Total output bars: ${allSma.length}`);
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

=== "C"

    ```c
    #include "tulip_rs_ffi.h"   /* also pulls in tulip_rs_ffi_state_ids.h */

    /* Serialise — the live state handle is read, not consumed */
    CBytes blob = tulip_state_serialize(C_INDICATOR_ID_ADX, C_STATE_FORMAT_BINCODE, state);
    if (blob.ptr == NULL) { /* handle error */ }

    /* Persist blob.ptr / blob.len to disk or a database ... */

    /* Restore — the blob is self-describing (it embeds the indicator name
       and format), so deserialisation takes no other arguments */
    void *restored = tulip_state_deserialize(blob.ptr, blob.len);
    tulip_ffi_bytes_free(blob);

    /* The restored handle behaves exactly like a fresh one */
    CBatchResult b = adx_batch(restored, new_inputs, n_new, NULL, 0);
    /* ... use b.outputs ... */
    tulip_ffi_batch_result_free(b);
    adx_state_free(restored);   /* each handle is freed exactly once */
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    // Serialise — Go genuinely supports both formats (C tab's limitation-warning does NOT apply)
    blob, err := st.Serialize(tulip.FormatBincode) // recommended: compact, handles NaN/Inf
    if err != nil {
        fmt.Printf("serialize failed: %v\n", err)
        return
    }
    fmt.Printf("bincode blob: %d bytes (indicator id 0x%08x)\n", len(blob), indicators.AdxID)

    // Persist blob to disk / database ...

    // Restore — use the indicator's DeserializeState function
    rs, err := indicators.Adx.DeserializeState(blob)
    if err != nil {
        fmt.Printf("deserialize failed: %v\n", err)
        return
    }
    defer rs.Close()

    // Continue from restored state
    br, err := rs.Batch(newClose, nil)
    if err != nil {
        fmt.Printf("batch failed: %v\n", err)
        return
    }
    defer br.Close()

    // FormatJSON too (human-readable; FFI rejects non-finite f64s)
    jsonBlob, err := st.Serialize(tulip.FormatJSON)
    if err != nil {
        fmt.Printf("json serialize failed: %v\n", err)
    } else {
        fmt.Printf("json blob: %d bytes\n", len(jsonBlob))
    }
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Sma;

    // Serialise — Java genuinely supports both formats (C tab's limitation-warning does NOT apply)
    Outcome oc = Sma.indicator(new double[][] {close}, new double[] {5.0});
    try (State st = oc.state()) {
        byte[] blob = st.serialize(Format.BINCODE); // recommended: compact, handles NaN/Inf
        System.out.printf("bincode blob: %d bytes (indicator id 0x%08x)%n", blob.length, Sma.ID);

        // Persist blob to disk / database ...

        // Restore — use the indicator's deserializeState function
        State rs = Sma.deserializeState(blob);
        try (rs) {
            double[] newClose = {87.10, 88.25};
            Result br = rs.batch(new double[][] {newClose});
            try (br) {
                System.out.println("Continued from restored state: " + java.util.Arrays.toString(br.toDoubleArray(0)));
            }
        }

        // FormatJSON too (human-readable; FFI rejects non-finite f64s)
        byte[] jsonBlob = st.serialize(Format.JSON);
        System.out.printf("json blob: %d bytes%n", jsonBlob.length);
        String jsonStr = new String(jsonBlob, java.nio.charset.StandardCharsets.UTF_8);
        System.out.println("JSON: " + jsonStr);
    }
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

=== "Node.js"

    ```javascript
    // Serialise
    const json = state.toJson();

    // Persist json to disk / database...

    // Restore
    const restored = ti.sma.State.fromJson(json);

    // Continue from restored state
    const newBars = Float64Array.from([87.10, 88.25]);
    const result = restored.batchIndicator([newBars]);
    ```

!!! tip "Bincode vs JSON, and in-process clone"
    `C_STATE_FORMAT_BINCODE` (0) round-trips every `f64` including NaN/Inf and is the recommended persistence format. `C_STATE_FORMAT_JSON` (1) emits human-readable `serde_json` but fails (null `CBytes`) if the state holds non-finite values. Corrupted or wrong-schema blobs are rejected with `NULL` from `tulip_state_deserialize` — never a mistyped handle. For an in-process deep copy without serde, use `tulip_state_clone(C_INDICATOR_ID_ADX, state)`; the clone is owned like a deserialised state and freed with `adx_state_free`.

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

=== "C"

    ```c
    #include "tulip_rs_ffi.h"
    #include "tulip_rs_ffi_counts.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[BBANDS_OPTIONS] = {20.0, 2.0}; // period, std_dev
    const double *inputs[BBANDS_INPUTS] = {close};

    /* Seed on historical data */
    CIndicatorResult r = bbands_indicator(inputs, n, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "bbands_indicator failed: error=%d\n", r.error);
        return 1;
    }

    /* outputs[0] = lower_band, outputs[1] = middle_band, outputs[2] = upper_band */
    printf("lower_band:  [");
    for (uintptr_t i = 0; i < r.output_lens[0]; i++) {
        printf("%.4f", r.outputs[0][i]);
        if (i + 1 < r.output_lens[0]) printf(", ");
    }
    printf("]\n");

    printf("middle_band: [");
    for (uintptr_t i = 0; i < r.output_lens[1]; i++) {
        printf("%.4f", r.outputs[1][i]);
        if (i + 1 < r.output_lens[1]) printf(", ");
    }
    printf("]\n");

    printf("upper_band:  [");
    for (uintptr_t i = 0; i < r.output_lens[2]; i++) {
        printf("%.4f", r.outputs[2][i]);
        if (i + 1 < r.output_lens[2]) printf(", ");
    }
    printf("]\n");

    void *state = r.state;
    tulip_ffi_result_free(r);

    /* Append new bars — all three outputs are extended together */
    double new_close[] = {85.53, 86.54};
    const double *new_inputs[BBANDS_INPUTS] = {new_close};
    CBatchResult b = bbands_batch(state, new_inputs, 2, NULL, 0);
    if (b.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "bbands_batch failed: error=%d\n", b.error);
        return 1;
    }

    printf("lower_band continued:  [");
    for (uintptr_t i = 0; i < b.output_lens[0]; i++) {
        printf("%.4f", b.outputs[0][i]);
        if (i + 1 < b.output_lens[0]) printf(", ");
    }
    printf("]\n");

    printf("middle_band continued: [");
    for (uintptr_t i = 0; i < b.output_lens[1]; i++) {
        printf("%.4f", b.outputs[1][i]);
        if (i + 1 < b.output_lens[1]) printf(", ");
    }
    printf("]\n");

    printf("upper_band continued:  [");
    for (uintptr_t i = 0; i < b.output_lens[2]; i++) {
        printf("%.4f", b.outputs[2][i]);
        if (i + 1 < b.output_lens[2]) printf(", ");
    }
    printf("]\n");

    tulip_ffi_batch_result_free(b);
    bbands_state_free(state); /* final cleanup */
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36}

    options := []float64{20.0, 2.0} // period, std_dev
    res, st, err := indicators.Bbands.Indicator(close[:n], options, nil)
    if err != nil {
        fmt.Printf("bbands_indicator failed: %v\n", err)
        return
    }
    defer res.Close()
    defer st.Close()

    lower  := tulip.AsFloat64(res.Rows[0]) // lower band (3 rows total)
    middle := tulip.AsFloat64(res.Rows[1]) // middle band
    upper  := tulip.AsFloat64(res.Rows[2]) // upper band

    fmt.Println("lower_band:", lower)
    fmt.Println("middle_band:", middle)
    fmt.Println("upper_band:", upper)

    // Continue — all three output series are extended together
    continued, err := st.Batch(newClose, nil)
    if err != nil {
        fmt.Printf("bbands_batch failed: %v\n", err)
        return
    }
    defer continued.Close()

    newLower  := tulip.AsFloat64(continued.Rows[0])
    newMiddle := tulip.AsFloat64(continued.Rows[1])
    newUpper  := tulip.AsFloat64(continued.Rows[2])

    fmt.Println("lower_band continued:", newLower)
    fmt.Println("middle_band continued:", newMiddle)
    fmt.Println("upper_band continued:", newUpper)
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Bbands;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    double[] options = {20.0, 2.0}; // period, std_dev
    Outcome oc = Bbands.indicator(new double[][] {close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        double[] lower  = res.toDoubleArray(0); // lower band (3 rows total)
        double[] middle = res.toDoubleArray(1); // middle band
        double[] upper  = res.toDoubleArray(2); // upper band

        System.out.println("lower_band: " + java.util.Arrays.toString(lower));
        System.out.println("middle_band: " + java.util.Arrays.toString(middle));
        System.out.println("upper_band: " + java.util.Arrays.toString(upper));

        // Continue — all three output series are extended together
        double[] newClose = {85.53, 86.54};
        Result br = st.batch(new double[][] {newClose});
        try (br) {
            double[] newLower  = br.toDoubleArray(0);
            double[] newMiddle = br.toDoubleArray(1);
            double[] newUpper  = br.toDoubleArray(2);

            System.out.println("lower_band continued: " + java.util.Arrays.toString(newLower));
            System.out.println("middle_band continued: " + java.util.Arrays.toString(newMiddle));
            System.out.println("upper_band continued: " + java.util.Arrays.toString(newUpper));
        }
    }
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

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    // options: [period, stddev_multiplier]
    const [outputs, state] = ti.bbands.indicator([close.slice(0, n)], [20, 2]);

    const lower  = outputs[0];
    const middle = outputs[1];
    const upper  = outputs[2];

    // Continue — all three output series are extended together
    const continued = state.batchIndicator([newClose]);
    const newLower  = continued[0];
    const newMiddle = continued[1];
    const newUpper  = continued[2];
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
