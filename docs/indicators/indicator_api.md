# Indicator API Reference

Every TulipRS indicator exposes a consistent set of functions beyond the core `indicator()` call. This page covers the metadata and utility functions that let you introspect an indicator's inputs, outputs, and data requirements at runtime.

---

## `info()` — Indicator Metadata

Every indicator module exports an `info()` function that returns a fully-populated `Info` struct describing the indicator. This is the canonical place to discover what an indicator needs and what it produces — without reading source code or docs.

```rust
pub struct Info {
    pub name:             &'static str,              // short identifier, e.g. "adosc"
    pub full_name:        &'static str,              // e.g. "Accumulation/Distribution Oscillator"
    pub indicator_type:   IndicatorType,             // Trend | Momentum | Volume | Volatility | Price | Cycle
    pub inputs:           &'static [&'static str],   // names of required input series
    pub options:          &'static [&'static str],   // names of option parameters, in order
    pub outputs:          &'static [&'static str],   // names of primary output series, in order
    pub optional_outputs: &'static [&'static str],   // names of optional output series, in order
    pub display_groups:   &'static [DisplayGroup],   // display pane groupings
}

pub struct DisplayGroup {
    pub id:           &'static str,                // machine-readable key, e.g. "emas"
    pub label:        &'static str,                // human-readable pane title, e.g. "AD EMAs"
    pub display_type: DisplayType,                 // Overlay | Indicator | Volume for this pane
    pub outputs:      &'static [&'static str],     // which outputs belong to this pane
}
```

### Usage

=== "Rust"

    ```rust
    use tulip_rs::indicators::adosc::{Adosc, Indicator};

    let meta = Adosc::INFO;

    println!("Name:             {}", meta.name);               // adosc
    println!("Full name:        {}", meta.full_name);          // Accumulation/Distribution Oscillator
    println!("Type:             {}", meta.indicator_type);     // Volume
    println!("Inputs:           {:?}", meta.inputs);           // ["high", "low", "close", "volume"]
    println!("Options:          {:?}", meta.options);          // ["short_period", "long_period"]
    println!("Outputs:          {:?}", meta.outputs);          // ["adosc"]
    println!("Optional outputs: {:?}", meta.optional_outputs); // ["short_ema", "long_ema", "ad"]
    for group in meta.display_groups {
        println!("  Group {}: {} ({:?})", group.id, group.label, group.display_type);
    }
    // Group adosc: ADOSC (Indicator)
    // Group emas: AD EMAs (Indicator)
    // Group ad: AD Line (Indicator)
    ```

=== "C"

    ```c
    #include <tulip_rs_ffi.h>

    CIndicatorInfo info = adosc_info();

    printf("Name:             %s\n", info.name);              // adosc
    printf("Full name:        %s\n", info.full_name);         // Accumulation/Distribution Oscillator
    printf("Inputs:           %u\n", (unsigned)info.inputs.len);           // 4
    printf("Options:          %u\n", (unsigned)info.options.len);          // 2
    printf("Outputs:          %u\n", (unsigned)info.outputs.len);          // 1
    printf("Optional outputs: %u\n", (unsigned)info.optional_outputs.len); // 3

    for (uintptr_t i = 0; i < info.display_groups.len; i++) {
        CDisplayGroup g = info.display_groups.ptr[i];
        printf("  Group %s: %s (%d)\n", g.id, g.label, g.display_type);
    }
    // Group adosc: ADOSC (Indicator)
    // Group emas: AD EMAs (Indicator)
    // Group ad: AD Line (Indicator)

    // Strings are leaked process-lifetime; read them, don't free
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    info := indicators.Adx.Info()
    // Info struct fields (all Go strings are copies; nothing to free):
    // {
    //   Name:            "adx",
    //   FullName:        "Average Directional Index",
    //   Type:            "Trend",       // from indicator_type
    //   Inputs:          []string{"high", "low", "close"},
    //   Options:         []string{"period"},
    //   Outputs:         []string{"adx"},
    //   OptionalOutputs: []string{"dx", "atr", "tr"},
    //   DisplayGroups:   []tulip.DisplayGroup{...}
    // }

    fmt.Printf("Name:             %s\n", info.Name)
    fmt.Printf("Full name:        %s\n", info.FullName)
    fmt.Printf("Type:             %s\n", info.Type)     // Trend | Momentum | Volume | Volatility | Price | Cycle
    fmt.Printf("Inputs:           %v\n", info.Inputs)
    fmt.Printf("Options:          %v\n", info.Options)
    fmt.Printf("Outputs:          %v\n", info.Outputs)
    fmt.Printf("Optional outputs: %v\n", info.OptionalOutputs)
    for _, g := range info.DisplayGroups {
        fmt.Printf("  Group %s: %s (%s)\n", g.ID, g.Label, g.DisplayType)
    }
    ```

=== "Python"

    `info()` returns a plain Python `dict`. Access fields with standard key lookup:

    ```python
    import tulip_rs

    meta = tulip_rs.indicators.adosc.info()

    print(meta["name"])              # adosc
    print(meta["full_name"])         # Accumulation/Distribution Oscillator
    print(meta["inputs"])            # ['high', 'low', 'close', 'volume']
    print(meta["options"])           # ['short_period', 'long_period']
    print(meta["outputs"])           # ['adosc']
    print(meta["optional_outputs"])  # ['short_ema', 'long_ema', 'ad']
    for group in meta["display_groups"]:
        print(group["id"], group["label"], group["display_type"])
    # adosc ADOSC Indicator
    # emas AD EMAs Indicator
    # ad AD Line Indicator
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const info = ti.adosc.info;
    console.log(info.name);              // adosc
    console.log(info.fullName);          // Accumulation/Distribution Oscillator
    console.log(info.inputs);            // ['high', 'low', 'close', 'volume']
    console.log(info.options);           // ['short_period', 'long_period']
    console.log(info.outputs);           // ['adosc']
    console.log(info.optionalOutputs);   // ['short_ema', 'long_ema', 'ad']
    console.log(info.displayGroups);
    // [
    //   { id: 'adosc', label: 'ADOSC', displayType: 'Indicator', outputs: ['adosc'] },
    //   { id: 'emas', label: 'AD EMAs', displayType: 'Indicator', outputs: ['short_ema', 'long_ema'] },
    //   { id: 'ad', label: 'AD Line', displayType: 'Indicator', outputs: ['ad'] }
    // ]
    ```

=== "WASM"

    The `info` property is a lazy getter on each `Indicator` instance — it is fetched from the WASM module on first access after `init()` has been called. The shape is identical to the Node.js binding.

    ```javascript
    import { init, adosc } from 'tulip-rs-wasm';

    await init();

    const info = adosc.info;
    console.log(info.name);              // adosc
    console.log(info.fullName);          // Accumulation/Distribution Oscillator
    console.log(info.inputs);            // ['high', 'low', 'close', 'volume']
    console.log(info.options);           // ['short_period', 'long_period']
    console.log(info.outputs);           // ['adosc']
    console.log(info.optionalOutputs);   // ['short_ema', 'long_ema', 'ad']
    console.log(info.displayGroups);
    // [
    //   { id: 'adosc', label: 'ADOSC', displayType: 'Indicator', outputs: ['adosc'] },
    //   { id: 'emas', label: 'AD EMAs', displayType: 'Indicator', outputs: ['short_ema', 'long_ema'] },
    //   { id: 'ad', label: 'AD Line', displayType: 'Indicator', outputs: ['ad'] }
    // ]
    ```

### What each field means

| Field | Description |
|---|---|
| `name` | The short identifier used to locate the module: `tulip_rs::indicators::<name>` |
| `full_name` | Human-readable name suitable for display in UIs or reports |
| `indicator_type` | Broad category — useful for filtering or grouping indicators |
| `display_groups` | One or more display pane groupings, each with an `id`, `label`, `display_type` (Overlay / Indicator), and the `outputs` it contains |
| `inputs` | Input series names, in the order they must be passed to `indicator()` |
| `options` | Option parameter names, in the order they must be passed to `indicator()` |
| `outputs` | Primary output series names. `outputs[i]` corresponds to `indicator_result[i]` |
| `optional_outputs` | Optional intermediate output series. See [Optional Outputs](#optional-outputs) below |

### Common use cases

- **Building dynamic UIs** — populate dropdowns, form labels, and axis titles without hardcoding strings.
- **Validation** — check `inputs.len()` and `options.len()` before constructing a call.
- **Introspection in tests** — confirm that the number of returned output vecs matches `outputs.len() + optional_outputs.len()`.
- **Auto-generating documentation** — iterate all indicator modules and call `info()` to produce a live reference table.

---

## Optional Outputs

Many indicators compute intermediate series as part of their normal calculation. Rather than discarding these values, TulipRS can return them alongside the primary outputs — at **no extra computation cost**, since they were calculated anyway.

This is a meaningful advantage over C Tulip and TA-Lib, which require a **separate function call** for each intermediate result, each re-reading the input data from scratch. TulipRS computes the primary output and every optional output in a **single pass** through the data. Depending on the indicator, requesting all optional outputs via TulipRS is **1.3× – 8.7× faster** than equivalent multi-call C code — see the [Optional Outputs benchmark](../benchmarks/optional-outputs.md) for full numbers per indicator.

Optional outputs are **off by default**. Requesting them never changes the primary output values; it only captures values that would otherwise be thrown away.

### Which optional outputs does an indicator have?

Reference `INFO` constant and inspect the `optional_outputs` field:

```rust
use tulip_rs::indicators::adx::{Adx, Indicator};

let meta = Adx::INFO;
println!("{:?}", meta.optional_outputs); // ["dx", "atr", "tr"]
```

Common examples:

| Indicator | Primary output | Optional outputs |
|---|---|---|
| `adosc` | `adosc` | `short_ema`, `long_ema`, `ad` |
| `adx` | `adx` | `dx`, `atr`, `tr` |
| `adxr` | `adxr` | `adx`, `dx`, `atr`, `tr` |
| `ao` | `ao` | `short_sma`, `long_sma`, `medprice` |
| `macd` | `macd` | *(primary outputs include signal and histogram)* |

### Requesting optional outputs

The third argument to `indicator()` is `optional_outputs: Option<&[bool]>`. Each element corresponds to one optional output, **in the same order as `info().optional_outputs`**:

- `None` — no optional outputs are returned (default; best performance when you don't need them).
- `Some(&[bool; N])` — a mask where `true` means "return this series" and `false` means "skip it".

=== "Rust"

    ```rust
    use tulip_rs::indicators::adosc::{Adosc, Indicator, TIndicatorState};

    let high  = vec![/* ... */];
    let low   = vec![/* ... */];
    let close = vec![/* ... */];
    let vol   = vec![/* ... */];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice(), vol.as_slice()];

    // Adosc::INFO.optional_outputs == ["short_ema", "long_ema", "ad"]
    //                                ^^^^^^^^^^^  ^^^^^^^^^^  ^^^^
    //                                index 0      index 1     index 2

    // Request only the AD line (index 2); skip short_ema and long_ema
    let mask = [false, false, true];
    let (outputs, state) = Adosc::indicator(&inputs, &[6.0, 20.0], Some(&mask)).unwrap();

    let adosc_line = &outputs[0]; // primary output — always present
    // outputs[1] and outputs[2] are empty (not requested)
    let ad_line    = &outputs[3]; // optional output at index 2 — present because mask[2] == true
    ```

    !!! note "Output vector layout"
        `outputs` always has length `outputs.len() + optional_outputs.len()` (from `info()`).
        Primary outputs come first (always populated), then optional outputs in declaration order
        (populated or empty depending on the mask).

=== "C"

    ```c
    #include <tulip_rs_ffi.h>

    const double *inputs[4] = {high, low, close, volume};
    const double options[2] = {6.0, 20.0}; // short_period, long_period

    // Adosc optional_outputs: ["short_ema", "long_ema", "ad"]
    //                         index 0       index 1    index 2

    bool optional_outputs[3] = {false, false, true}; // request AD line only (index 2)

    CIndicatorResult r = adosc_indicator(inputs, data_len, options,
                                         optional_outputs, 3);

    double *adosc_line = r.outputs[0]; // primary output — always present
    // outputs[1] and outputs[2] are empty (not requested)
    double *ad_line    = r.outputs[3]; // optional output at index 2

    tulip_ffi_result_free(r);
    adosc_state_free(r.state);
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    high  := []float64{/* ... */}
    low   := []float64{/* ... */}
    close := []float64{/* ... */}
    vol   := []float64{/* ... */}
    inputs := [][]float64{high, low, close, vol}

    // indicators.Adx.Info().OptionalOutputs == ["dx", "atr", "tr"]
    //                                         index 0  index 1  index 2

    // Request only the AD line (index 2); skip dx and atr
    mask := []bool{false, false, true}

    res, st, _ := indicators.Adx.Indicator(high, low, close, []float64{14.0}, mask)
    defer res.Close()
    defer st.Close()

    adxLine := tulip.AsFloat64(res.Rows[0]) // primary output — always present
    // res.Rows[1] and res.Rows[2] are empty (not requested)
    adLine  := tulip.AsFloat64(res.Rows[3]) // optional output at index 2
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([...], dtype=np.float64)
    low   = np.array([...], dtype=np.float64)
    close = np.array([...], dtype=np.float64)
    vol   = np.array([...], dtype=np.float64)

    # Request the AD line only (index 2 of optional_outputs)
    outputs, state = tulip_rs.indicators.adosc.indicator(
        [high, low, close, vol],
        [6.0, 20.0],
        optional_outputs=[False, False, True],
    )

    adosc_line = outputs[0]   # primary output
    ad_line    = outputs[3]   # optional output at index 2
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    // info().optionalOutputs == ['short_ema', 'long_ema', 'ad']
    //                             index 0       index 1    index 2

    // Request only the AD line (index 2); skip short_ema and long_ema
    const [outputs] = ti.adosc.indicator([high, low, close, volume], [6, 20], [false, false, true]);

    const adoscLine = outputs[0]; // primary output — always present
    // outputs[1] and outputs[2] are empty (not requested)
    const adLine    = outputs[3]; // optional output at index 2
    ```

### All optional outputs at once

Pass a mask of all `true` to capture every intermediate series:

=== "Rust"

    ```rust
    // adosc has 3 optional outputs
    let mask = [true, true, true];
    let (outputs, state) = adosc::indicator(&inputs, &[6.0, 20.0], Some(&mask)).unwrap();

    let adosc_line     = &outputs[0]; // adosc     (primary)
    let short_ema_line = &outputs[1]; // short_ema (optional 0)
    let long_ema_line  = &outputs[2]; // long_ema  (optional 1)
    let ad_line        = &outputs[3]; // ad        (optional 2)
    ```

=== "C"

    ```c
    // adosc has 3 optional outputs
    bool optional_outputs[3] = {true, true, true};
    CIndicatorResult r = adosc_indicator(inputs, data_len, options,
                                         optional_outputs, 3);

    double *adosc_line     = r.outputs[0]; // adosc     (primary)
    double *short_ema_line = r.outputs[1]; // short_ema (optional 0)
    double *long_ema_line  = r.outputs[2]; // long_ema  (optional 1)
    double *ad_line        = r.outputs[3]; // ad        (optional 2)

    tulip_ffi_result_free(r);
    adosc_state_free(r.state);
    ```

=== "Go"

    ```go
    // adosc has 3 optional outputs: short_ema, long_ema, ad
    mask := []bool{true, true, true}
    res, st, _ := indicators.Adosc.Indicator(high, low, close, volume, []float64{6.0, 20.0}, mask)
    defer res.Close()
    defer st.Close()

    adoscLine    := res.Rows[0] // adosc     (primary)
    shortEmaLine := res.Rows[1] // short_ema (optional 0)
    longEmaLine  := res.Rows[2] // long_ema  (optional 1)
    adLine       := res.Rows[3] // ad        (optional 2)
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.adosc.indicator(
        [high, low, close, vol],
        [6.0, 20.0],
        optional_outputs=[True, True, True],
    )

    adosc_line     = outputs[0]
    short_ema_line = outputs[1]
    long_ema_line  = outputs[2]
    ad_line        = outputs[3]
    ```

=== "Node.js"

    ```javascript
    // adosc has 3 optional outputs
    const [outputs] = ti.adosc.indicator([high, low, close, volume], [6, 20], [true, true, true]);

    const adoscLine    = outputs[0]; // adosc     (primary)
    const shortEmaLine = outputs[1]; // short_ema (optional 0)
    const longEmaLine  = outputs[2]; // long_ema  (optional 1)
    const adLine       = outputs[3]; // ad        (optional 2)
    ```

### Optional outputs in streaming mode

Optional output masks work the same way with `batch_indicator()`. Pass the same mask you used in the initial `indicator()` call:

=== "Rust"

    ```rust
    // Initial batch — request AD line
    let mask = [false, false, true];
    let (outputs, mut state) = Adosc::indicator(&inputs, &[6.0, 20.0], Some(&mask)).unwrap();

    // Continue streaming — same mask
    let new_inputs = [new_high.as_slice(), new_low.as_slice(), new_close.as_slice(), new_vol.as_slice()];
    let continued = state.batch_indicator(&new_inputs, Some(&mask)).unwrap();

    let new_adosc = &continued[0];
    let new_ad    = &continued[3];
    ```

=== "C"

    ```c
    // Initial batch — request AD line (index 2 of optional_outputs)
    bool initial_optional[3] = {false, false, true};
    CIndicatorResult r = adosc_indicator(inputs, data_len, options,
                                         initial_optional, 3);
    void *state = r.state;
    tulip_ffi_result_free(r); // outputs freed; state kept alive

    // Continue streaming — same mask (same as initial call)
    CBatchResult br = adosc_batch(state, new_inputs, new_data_len,
                                  initial_optional, 3);

    double *new_adosc = br.outputs[0];
    double *new_ad    = br.outputs[3];

    tulip_ffi_batch_result_free(br);
    adosc_state_free(state);
    ```

=== "Go"

    ```go
    // Initial call — request AD line (the third optional)
    mask := []bool{false, false, true}
    res, st, _ := indicators.Adosc.Indicator(high, low, close, volume, []float64{6.0, 20.0}, mask)
    res.Close() // outputs consumed or copied; the state stays live

    // Continue streaming — pass the same mask
    continued, _ := st.Batch(newHigh, newLow, newClose, newVol, mask)
    newAdosc := continued.Rows[0] // adosc (primary)
    newAd    := continued.Rows[1] // ad (only requested optionals are appended)
    continued.Close()
    st.Close()
    ```

=== "Node.js"

    ```javascript
    // Initial batch — request AD line
    const [outputs, state] = ti.adosc.indicator([high, low, close, volume], [6, 20], [false, false, true]);

    // Continue streaming — same mask
    const continued = state.batchIndicator([newHigh, newLow, newClose, newVol], [false, false, true]);

    const newAdosc = continued[0];
    const newAd    = continued[3];
    ```

    !!! note
        Pass the same boolean mask to `batchIndicator` that you used in the initial `indicator()` call.

### Optional outputs in SIMD mode

The SIMD functions `indicator_by_assets<N>` and `indicator_by_options<N>` accept exactly the same `optional_outputs: Option<&[bool]>` argument as the scalar `indicator()`, with identical semantics. The mask applies uniformly across all N assets or option sets, and each lane's output vector follows the same layout — primary outputs first, then optional outputs in declaration order.

The return type is `(Vec<Vec<Vec<f64>>>, Vec<IndicatorState>)`. Index the outer `Vec` to select an asset or option set; the inner `Vec<Vec<f64>>` is the same layout as the scalar `indicator()` return.

=== "Rust"

    ```rust
    use tulip_rs::indicators::adosc::{Adosc, Indicator, IndicatorByOptions};

    // indicator_by_assets: 4 assets, same options, same optional-output mask
    let mask = [false, false, true]; // request the AD line only
    let (all_outputs, _states) = Adosc::indicator_by_assets::<4>(
        &[&inputs_a, &inputs_b, &inputs_c, &inputs_d],
        &[6.0, 20.0],
        Some(&mask),
    ).unwrap();

    // all_outputs[0] is asset A's output Vec — same layout as scalar indicator()
    let adosc_a = &all_outputs[0][0]; // primary output
    let ad_a    = &all_outputs[0][3]; // optional output at index 2 (AD line)

    // indicator_by_options: 1 asset, 4 option sets, same optional-output mask
    let (all_outputs, _states) = Adosc::indicator_by_options::<4>(
        &inputs,
        &[&[3.0, 10.0], &[6.0, 20.0], &[12.0, 26.0], &[20.0, 50.0]],
        Some(&mask),
    ).unwrap();

    let adosc_set2 = &all_outputs[1][0]; // option set 1 primary output
    let ad_set2    = &all_outputs[1][3]; // option set 1 AD line
    ```

=== "C"

    ```c
    // SIMD by assets: 4 assets, same options, same optional-output mask
    bool optional_outputs[3] = {false, false, true}; // request the AD line only

    const double *const asset1[4] = {high_a, low_a, close_a, vol_a};
    const double *const asset2[4] = {high_b, low_b, close_b, vol_b};
    const double *const asset3[4] = {high_c, low_c, close_c, vol_c};
    const double *const asset4[4] = {high_d, low_d, close_d, vol_d};

    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = adosc_simd_by_assets(simd_inputs, 4, data_len,
                                         options, optional_outputs, 3);

    // all_outputs[0] is asset A — same layout as scalar indicator()
    double *adosc_a = r.outputs[0][0]; // primary output
    double *ad_a    = r.outputs[0][3]; // optional output at index 2 (AD line)

    for (uintptr_t i = 0; i < r.num_results; i++) adosc_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    // SIMD by assets: 4 assets, same options, same optional-output mask
    mask := []bool{false, false, true} // request the AD line only

    assets := [][indicators.AdxInputs][]float64{
        {high_a, low_a, close_a},
        {high_b, low_b, close_b},
        {high_c, low_c, close_c},
        {high_d, low_d, close_d},
    }

    sim, _ := indicators.Adx.SimdByAssets(assets, []float64{14.0}, mask)
    defer sim.Close()

    // sim.Results[0] is asset A — same layout as scalar indicator()
    adx_a := tulip.AsFloat64(sim.Results[0][0]) // primary output
    ad_a  := tulip.AsFloat64(sim.Results[0][3]) // optional output at index 2 (AD line)

    // SIMD by options: 1 asset, 4 option sets, same mask
    sim2, _ := indicators.Adx.SimdByOptions(high, low, close,
        [][]float64{{3.0}, {6.0}, {12.0}, {20.0}}, mask)
    defer sim2.Close()

    adx_set1 := tulip.AsFloat64(sim2.Results[0][0]) // option set 0 primary output
    ad_set1  := tulip.AsFloat64(sim2.Results[0][3]) // option set 0 AD line
    ```

=== "Python"

    ```python
    import tulip_rs

    # simd_by_assets: 4 assets, same options, same optional-output mask
    mask = [False, False, True]  # request the AD line only
    all_outputs, states = tulip_rs.indicators.adosc.simd_by_assets(
        [[high_a, low_a, close_a, vol_a],
         [high_b, low_b, close_b, vol_b],
         [high_c, low_c, close_c, vol_c],
         [high_d, low_d, close_d, vol_d]],
        [6.0, 20.0],
        optional_outputs=mask,
    )
    # all_outputs[0] is asset A — same layout as scalar indicator()
    adosc_a = all_outputs[0][0]  # primary output
    ad_a    = all_outputs[0][3]  # AD line

    # simd_by_options: 1 asset, 4 option sets, same mask
    all_outputs, states = tulip_rs.indicators.adosc.simd_by_options(
        [high, low, close, vol],
        [[3.0, 10.0], [6.0, 20.0], [12.0, 26.0], [20.0, 50.0]],
        optional_outputs=mask,
    )
    adosc_set2 = all_outputs[1][0]
    ad_set2    = all_outputs[1][3]
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    // simdByAssets: 4 assets, same options, same optional-output mask
    const mask = [false, false, true]; // request the AD line only
    const [allOutputs] = ti.adosc.simdByAssets(
        [[highA, lowA, closeA, volA],
         [highB, lowB, closeB, volB],
         [highC, lowC, closeC, volC],
         [highD, lowD, closeD, volD]],
        [6, 20],
        mask,
    );
    // allOutputs[0] is asset A — same layout as scalar indicator()
    const adoscA = allOutputs[0][0]; // primary output
    const adA    = allOutputs[0][3]; // AD line

    // simdByOptions: 1 asset, 4 option sets, same mask
    const [allOutputs2] = ti.adosc.simdByOptions(
        [high, low, close, volume],
        [[3, 10], [6, 20], [12, 26], [20, 50]],
        mask,
    );
    const adoscSet2 = allOutputs2[1][0];
    const adSet2    = allOutputs2[1][3];
    ```

!!! note
    The boolean mask is shared across all lanes — you cannot request different optional outputs for different assets or option sets in a single SIMD call.

### Performance note

Optional outputs are computed as part of the indicator's normal calculation loop — requesting them adds **zero algorithmic overhead**. The only cost is the memory allocation for the extra output vectors and the store instructions to write them. Passing `None` (or an all-`false` mask) allows the compiler to elide those stores entirely, which is why `None` is the default.

The performance difference between requesting all optional outputs and requesting none is documented in the [Benchmarks](../benchmarks/optional-outputs.md) page — typically 5–15% depending on the indicator.

---

## `min_data()` — Minimum Input Length

```rust
pub fn min_data(options: &[f64]) -> usize
```

Returns the **absolute minimum number of input bars** needed to produce at least one output bar. If you call `indicator()` with fewer bars than this, it returns `Err(IndicatorError::NotEnoughData)`.

The value depends on the indicator's options because period-based indicators require at least `period` bars to produce their first output.

=== "Rust"

    ```rust
    use tulip_rs::indicators::adx::{Adx, Indicator, TIndicatorState};

    // ADX with period = 14 needs at least 14*2 = 28 bars
    let minimum = Adx::min_data(&[14.0]);
    println!("Min data: {minimum}"); // 28

    // Check before calling
    if close.len() < minimum {
        eprintln!("Not enough data: have {}, need {}", close.len(), minimum);
    } else {
        let (outputs, state) = Adx::indicator(&[high.as_slice(), low.as_slice(), close.as_slice()], &[14.0], None).unwrap();
    }
    ```

=== "C"

    ```c
    #include <tulip_rs_ffi.h>

    const double options[1] = {14.0}; // period

    uintptr_t minimum = adx_min_data(options);
    printf("Min data: %zu\n", minimum); // 28

    if (data_len < minimum) {
        fprintf(stderr, "Not enough data: have %zu, need %zu\n", data_len, minimum);
    } else {
        const double *inputs[3] = {high, low, close};
        CIndicatorResult r = adx_indicator(inputs, data_len, options, NULL, 0);
        // ... use outputs ...
        tulip_ffi_result_free(r);
        adx_state_free(r.state);
    }
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
    )

    minimum := indicators.Adx.MinData([]float64{14.0})
    fmt.Printf("Min data: %d\n", minimum) // 28

    if len(close) < int(minimum) {
        fmt.Printf("Not enough data: have %d, need %d\n", len(close), minimum)
    } else {
        res, st, _ := indicators.Adx.Indicator(high, low, close, []float64{14.0}, nil)
        defer res.Close()
        defer st.Close()
    }
    ```

=== "Python"

    ```python
    import tulip_rs

    minimum = tulip_rs.indicators.adx.min_data([14.0])
    print(f"Min data: {minimum}")  # 28

    if len(close) < minimum:
        print(f"Not enough data: have {len(close)}, need {minimum}")
    else:
        outputs, state = tulip_rs.indicators.adx.indicator([high, low, close], [14.0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const minimum = ti.adx.minData([14]);
    console.log(`Min data: ${minimum}`); // 28

    if (close.length < minimum) {
        console.error(`Not enough data: have ${close.length}, need ${minimum}`);
    } else {
        const [outputs, state] = ti.adx.indicator([high, low, close], [14]);
    }
    ```

---

## Function Summary

| Function | Signature | Returns |
|---|---|---|
| `min_data()` | `(options: &[f64]) -> usize` | Minimum bars to get any output |
| `indicator()` | `(inputs, options, optional_outputs) -> Result<(Vec<Vec<f64>>, State), Error>` | Primary computation |
| `state.batch_indicator()` | `(inputs, optional_outputs) -> Result<Vec<Vec<f64>>, Error>` | Streaming continuation |
