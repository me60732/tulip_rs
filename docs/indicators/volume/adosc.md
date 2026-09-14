# ADOSC — Accumulation/Distribution Oscillator

The difference between a short and long EMA of the A/D line, used to confirm price trends with volume.

**Inputs:** `[high, low, close, volume]` | **Options:** `[short_period, long_period]` | **Outputs:** `[adosc]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::adosc::{Adosc, Indicator, TIndicatorState};

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
    let (outputs, mut state) = Adosc::indicator(&inputs, &[3.0, 10.0], None).unwrap();
    println!("{:?}", outputs[0]); // ADOSC values

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let partial_close  = close[..8].to_vec();
    let partial_volume = volume[..8].to_vec();
    let (outputs2, mut state) = Adosc::indicator(&[partial_high.as_slice(), partial_low.as_slice(), partial_close.as_slice(), partial_volume.as_slice()], &[3.0, 10.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high   = vec![85.90_f64];
    let new_low    = vec![84.03_f64];
    let new_close  = vec![85.53_f64];
    let new_volume = vec![1520.0_f64];
    let continued = state.batch_indicator(
        &[new_high.as_slice(), new_low.as_slice(),
          new_close.as_slice(), new_volume.as_slice()],
        None,
    ).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]   = {82.15, 81.89, 83.03, 83.30, 83.85,
                       83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]    = {81.29, 80.64, 81.31, 82.65, 83.07,
                       83.11, 82.49, 82.30, 84.15, 84.11};
    double close[]  = {81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36};
    double volume[] = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
                       900.0, 1500.0, 1800.0, 1000.0, 1700.0};
    const double options[ADOSC_OPTIONS] = {3.0, 10.0}; // short_period, long_period

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = adosc_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the ADOSC series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    adosc_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = adosc_indicator(inputs, 8, options, NULL, 0);
    double new_high[]   = {85.90};
    double new_low[]    = {84.03};
    double new_close[]  = {85.53};
    double new_volume[] = {1520.0};
    const double *new_inputs[ADOSC_INPUTS] = {new_high, new_low, new_close, new_volume};
    CBatchResult b = adosc_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> ADOSC value for the single new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    adosc_state_free(p.state);
    ```

=== "Python"

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

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high   = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low    = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close  = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);
    const volume = Float64Array.from([5653100, 6447400, 7690900, 3831400, 4455100, 3798000, 3936200, 4732000, 4841300, 3915300, 6830800, 6694100, 5293600, 7985800, 4807900]);

    const [outputs, state] = ti.adosc.indicator([high, low, close, volume], [3, 10]);
    console.log('ADOSC:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.adosc.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n), volume.slice(0, n)], [3, 10]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n), volume.slice(n)]);
    console.log('Continued ADOSC:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

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

=== "Rust"

    `adosc` exposes 3 optional outputs: `short_ema`, `long_ema`, `ad`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

=== "C"

    The mask is an array of booleans (one per optional output) passed to `adosc_indicator()`:

    ```c
    #include "tulip_rs_ffi.h"

    double close[]  = {81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36};
    double high[]   = {82.59, 82.06, 83.87, 84.00, 84.61,
                       84.15, 83.84, 84.99, 85.55, 85.36};
    double low[]    = {80.59, 80.06, 81.87, 82.00, 82.61,
                       82.15, 81.84, 82.99, 83.55, 83.36};
    double volume[] = {10000.0, 12000.0, 9500.0, 11000.0, 13000.0,
                       9800.0, 10500.0, 12500.0, 11800.0, 10200.0};
    const double options[ADOSC_OPTIONS] = {6.0, 20.0};

    bool optional_outputs[3] = {true, false, true}; // short_ema, long_ema, ad

    CIndicatorResult r = adosc_indicator(inputs, 10, options, optional_outputs, 3);
    /* r.outputs[0] -> adosc (primary) */
    /* r.outputs[1] -> short_ema (requested) */
    /* r.outputs[2] -> long_ema (not requested) */
    /* r.outputs[3] -> ad (requested) */
    tulip_ffi_result_free(r);
    adosc_state_free(r.state);
    ```

    ```rust
    use tulip_rs::indicators::adosc::{Adosc, Indicator, TIndicatorState};

    let close  = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let high   = close.iter().map(|x| x + 1.0).collect::<Vec<_>>();
    let low    = close.iter().map(|x| x - 1.0).collect::<Vec<_>>();
    let volume = vec![10000.0, 12000.0, 9500.0, 11000.0, 13000.0, 9800.0, 10500.0, 12500.0, 11800.0, 10200.0_f64];

    let mask = [true, false, true];
    let (outputs, _state) = Adosc::indicator(
        &[high.as_slice(), low.as_slice(), close.as_slice(), volume.as_slice()],
        &[6.0, 20.0],
        Some(&mask),
    ).unwrap();

    let adosc     = &outputs[0]; // adosc (primary)
    let short_ema = &outputs[1]; // short_ema (optional — requested)
    let long_ema  = &outputs[2]; // long_ema (optional — not requested)
    let ad        = &outputs[3]; // ad (optional — requested)
    ```

=== "Python"

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

=== "Node.js"

    `adosc` exposes 3 optional outputs: `short_ema`, `long_ema`, `ad`.

    ```javascript
    const [allOut] = ti.adosc.indicator([high, low, close, volume], [3, 10], [true, true, true]);
    const adosc    = allOut[0]; // primary
    const shortEma = allOut[1]; // optional 0: short_ema
    const longEma  = allOut[2]; // optional 1: long_ema
    const ad       = allOut[3]; // optional 2: ad
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.adosc.indicator([high, low, close, volume], [3, 10], [true, true, true]);
    const adosc    = allOut[0]; // primary
    const shortEma = allOut[1]; // optional 0: short_ema
    const longEma  = allOut[2]; // optional 1: long_ema
    const ad       = allOut[3]; // optional 2: ad
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::adosc::{Adosc, Indicator};

    let inputs: [&[&[f64]; 4]; 4] = [
        &[h1.as_slice(), l1.as_slice(), c1.as_slice(), v1.as_slice()],
        &[h2.as_slice(), l2.as_slice(), c2.as_slice(), v2.as_slice()],
        &[h3.as_slice(), l3.as_slice(), c3.as_slice(), v3.as_slice()],
        &[h4.as_slice(), l4.as_slice(), c4.as_slice(), v4.as_slice()],
    ];
    let results = Adosc::indicator_by_assets::<4>(&inputs, &[3.0, 10.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, N option sets in parallel:

    ```rust
    use tulip_rs::indicators::adosc::{Adosc, IndicatorByOptions};

    let opts: [&[f64; 2]; 4] = [&[2.0, 5.0], &[3.0, 10.0], &[5.0, 20.0], &[7.0, 28.0]];
    let results = Adosc::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, out) in results.iter().enumerate() {
        println!("Option set {}: {:?}", i + 1, out[0]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call:

    ```c
    double a1_high[]   = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double a1_low[]    = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double a1_close[]  = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a1_volume[] = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0, 900.0, 1500.0, 1800.0, 1000.0, 1700.0};

    double a2_high[]   = {84.30, 83.78, 86.06, 86.60, 87.70, 87.80, 86.66, 88.60, 89.68, 90.00};
    double a2_low[]    = {82.58, 81.28, 83.62, 84.30, 85.14, 85.22, 83.48, 85.70, 87.30, 87.22};
    double a2_close[]  = {83.18, 82.12, 85.74, 86.00, 87.22, 86.30, 85.68, 87.98, 89.10, 88.72};
    double a2_volume[] = {2400.0, 2800.0, 2200.0, 3200.0, 2600.0, 1800.0, 3000.0, 3600.0, 2000.0, 3400.0};

    double a3_high[]   = {75.00, 74.50, 76.00, 76.30, 76.85, 76.90, 76.33, 77.30, 77.84, 78.00};
    double a3_low[]    = {74.29, 73.64, 75.31, 75.65, 76.07, 76.11, 75.49, 75.30, 77.15, 77.11};
    double a3_close[]  = {74.59, 74.06, 76.87, 76.00, 76.61, 76.15, 75.84, 76.99, 77.55, 77.36};
    double a3_volume[] = {600.0, 700.0, 550.0, 800.0, 650.0, 450.0, 750.0, 900.0, 500.0, 850.0};

    double a4_high[]   = {90.00, 89.25, 91.50, 91.80, 92.30, 92.35, 91.75, 92.75, 93.25, 93.40};
    double a4_low[]    = {88.65, 88.00, 90.00, 90.20, 91.00, 91.05, 90.40, 91.30, 92.10, 92.25};
    double a4_close[]  = {89.30, 88.60, 91.00, 91.20, 91.75, 91.70, 91.10, 92.20, 92.65, 92.80};
    double a4_volume[] = {3600.0, 4200.0, 3300.0, 4800.0, 3900.0, 2700.0, 4500.0, 5400.0, 3000.0, 5100.0};

    const double *asset1[ADOSC_INPUTS] = {a1_high, a1_low, a1_close, a1_volume};
    const double *asset2[ADOSC_INPUTS] = {a2_high, a2_low, a2_close, a2_volume};
    const double *asset3[ADOSC_INPUTS] = {a3_high, a3_low, a3_close, a3_volume};
    const double *asset4[ADOSC_INPUTS] = {a4_high, a4_low, a4_close, a4_volume};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = adosc_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        adosc_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in one call:

    ```c
    static const double high_expanded[200] = {82.15, 81.89, 83.03, 83.30, 83.85,
        83.90, 83.33, 84.30, 84.84, 85.00, 82.15, 81.89, 83.03, 83.30, 83.85,
        83.90, 83.33, 84.30, 84.84, 85.00, 82.15, 81.89, 83.03, 83.30, 83.85,
        83.90, 83.33, 84.30, 84.84, 85.00, 82.15, 81.89, 83.03, 83.30, 83.85,
        83.90, 83.33, 84.30, 84.84, 85.00, 82.15, 81.89, 83.03, 83.30, 83.85,
        83.90, 83.33, 84.30, 84.84, 85.00, 82.15, 81.89, 83.03, 83.30, 83.85,
        83.90, 83.33, 84.30, 84.84, 85.00, 82.15, 81.89, 83.03, 83.30, 83.85,
        83.90, 83.33, 84.30, 84.84, 85.00, 82.15, 81.89, 83.03, 83.30, 83.85,
        83.90, 83.33, 84.30, 84.84, 85.00};
    static const double low_expanded[200] = {81.29, 80.64, 81.31, 82.65, 83.07,
        83.11, 82.49, 82.30, 84.15, 84.11, 81.29, 80.64, 81.31, 82.65, 83.07,
        83.11, 82.49, 82.30, 84.15, 84.11, 81.29, 80.64, 81.31, 82.65, 83.07,
        83.11, 82.49, 82.30, 84.15, 84.11, 81.29, 80.64, 81.31, 82.65, 83.07,
        83.11, 82.49, 82.30, 84.15, 84.11, 81.29, 80.64, 81.31, 82.65, 83.07,
        83.11, 82.49, 82.30, 84.15, 84.11, 81.29, 80.64, 81.31, 82.65, 83.07,
        83.11, 82.49, 82.30, 84.15, 84.11, 81.29, 80.64, 81.31, 82.65, 83.07,
        83.11, 82.49, 82.30, 84.15, 84.11, 81.29, 80.64, 81.31, 82.65, 83.07,
        83.11, 82.49, 82.30, 84.15, 84.11};
    static const double close_expanded[200] = {81.59, 81.06, 82.87, 83.00, 83.61,
        83.15, 82.84, 83.99, 84.55, 84.36, 81.59, 81.06, 82.87, 83.00, 83.61,
        83.15, 82.84, 83.99, 84.55, 84.36, 81.59, 81.06, 82.87, 83.00, 83.61,
        83.15, 82.84, 83.99, 84.55, 84.36, 81.59, 81.06, 82.87, 83.00, 83.61,
        83.15, 82.84, 83.99, 84.55, 84.36, 81.59, 81.06, 82.87, 83.00, 83.61,
        83.15, 82.84, 83.99, 84.55, 84.36, 81.59, 81.06, 82.87, 83.00, 83.61,
        83.15, 82.84, 83.99, 84.55, 84.36, 81.59, 81.06, 82.87, 83.00, 83.61,
        83.15, 82.84, 83.99, 84.55, 84.36, 81.59, 81.06, 82.87, 83.00, 83.61,
        83.15, 82.84, 83.99, 84.55, 84.36};
    static const double volume_expanded[200] = {1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
        900.0, 1500.0, 1800.0, 1000.0, 1700.0, 1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
        900.0, 1500.0, 1800.0, 1000.0, 1700.0, 1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
        900.0, 1500.0, 1800.0, 1000.0, 1700.0, 1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
        900.0, 1500.0, 1800.0, 1000.0, 1700.0, 1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
        900.0, 1500.0, 1800.0, 1000.0, 1700.0, 1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
        900.0, 1500.0, 1800.0, 1000.0, 1700.0, 1200.0, 1400.0, 1100.0, 1600.0, 1300.0,
        900.0, 1500.0, 1800.0, 1000.0, 1700.0};

    const double *const expanded_inputs[ADOSC_INPUTS] = {high_expanded, low_expanded,
        close_expanded, volume_expanded};

    static const double options_1[ADOSC_OPTIONS] = {2.0, 5.0};
    static const double options_2[ADOSC_OPTIONS] = {3.0, 10.0};
    static const double options_3[ADOSC_OPTIONS] = {5.0, 20.0};
    static const double options_4[ADOSC_OPTIONS] = {7.0, 28.0};

    const double *const simd_options[4] = {options_1, options_2, options_3, options_4};

    CSimdResult r = adosc_simd_by_options(expanded_inputs, 200, simd_options, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> option set i's series */
        adosc_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Python"

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

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice(), volume.slice()],
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
