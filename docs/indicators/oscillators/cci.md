# CCI — Commodity Channel Index

Measures how far the typical price deviates from its simple moving average, normalised by mean absolute deviation. Values above +100 suggest overbought conditions; values below -100 suggest oversold conditions.

**Inputs:** `[high, low, close]` &nbsp;|&nbsp; **Options:** `[period]` &nbsp;|&nbsp; **Outputs:** `[cci]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::cci::{Cci, TIndicatorState, Indicator};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = Cci::indicator(&inputs, &[20.0], None).unwrap();
    println!("CCI(20): {:?}", outputs[0]);

    // State continuation
    let inputs2 = [&high[..8], &low[..8], &close[..8]];
    let (outputs2, mut state) = Cci::indicator(&inputs2, &[20.0], None).unwrap();
    println!("Partial CCI: {:?}", outputs2[0]);

    let new_inputs = [&high[8..], &low[8..], &close[8..]];
    let continued = state.batch_indicator(&new_inputs, None).unwrap();
    println!("Continued CCI: {:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double options[CCI_OPTIONS] = {20.0}; // period
    const double *inputs[CCI_INPUTS] = {high, low, close};

    /* Full computation (check r.error == C_INDICATOR_ERROR_OK in real code) */
    CIndicatorResult r = cci_indicator(inputs, 10, options, NULL, 0);
    tulip_ffi_result_free(r);
    cci_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = cci_indicator(inputs, 8, options, NULL, 0);
    const double *rest_inputs[CCI_INPUTS] = {high+8, low+8, close+8};
    CBatchResult b = cci_batch(p.state, rest_inputs, 2, NULL, 0);
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    cci_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{20.0} // period

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Cci.Indicator(high, low, close, options, nil)
    fmt.Println(res.Rows[0]) // CCI(20) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Cci.Indicator(high[:8], low[:8], close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:], close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued CCI values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Cci;

    double[] high  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double[] low   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {20.0}; // period

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Cci.indicator(new double[][] {high, low, close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // CCI(20) values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Cci.indicator(new double[][] {
        java.util.Arrays.copyOfRange(high, 0, n),
        java.util.Arrays.copyOfRange(low, 0, n),
        java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {
            java.util.Arrays.copyOfRange(high, n, 10),
            java.util.Arrays.copyOfRange(low, n, 10),
            java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued CCI values
        }
    }
    ```

=== "Python"

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

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high  = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low   = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);
    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.cci.indicator([high, low, close], [20]);
    console.log('CCI(20):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.cci.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [20]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued CCI:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high  = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low   = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];
    const close = [81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.cci.indicator([high, low, close], [20]);
    console.log('CCI(20):', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.cci.indicator([high.slice(0, n), low.slice(0, n), close.slice(0, n)], [20]);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n), close.slice(n)]);
    console.log('Continued CCI:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `cci` exposes 3 optional outputs: `sma`, `md`, `typprice`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::cci::{Cci, TIndicatorState, Indicator};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true, true, true]; // one per optional output
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let (outputs, _state) = Cci::indicator(&inputs, &[20.0], Some(&mask)).unwrap();

    let cci      = &outputs[0]; // cci (primary)
    let sma      = &outputs[1]; // sma (optional — requested)
    let md       = &outputs[2]; // md (optional — requested)
    let typprice = &outputs[3]; // typprice (optional — requested)
    ```

=== "C"

    `cci` exposes 3 optional outputs: `sma`, `md`, `typprice`.

    **By assets**

    ```c
    #include <tulip_rs_ffi.h>

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    double options[CCI_OPTIONS] = {20.0};
    const double *inputs[CCI_INPUTS] = {high, low, close};
    bool optional_outputs[3] = {true, true, true}; // sma, md, typprice

    CIndicatorResult r = cci_indicator(inputs, 10, options,
                                       optional_outputs, 3);

    double *cci      = r.outputs[0]; // primary
    double *sma      = r.outputs[1]; // optional 0: sma
    double *md       = r.outputs[2]; // optional 1: md
    double *typprice = r.outputs[3]; // optional 2: typprice

    tulip_ffi_result_free(r);
    cci_state_free(r.state);
    ```

    **By options**

    ```c
    #include <tulip_rs_ffi.h>

    double high[]  = {82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]   = {81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    double o20[] = {20.0}, o14[] = {14.0}, o10[] = {10.0}, o5[] = {5.0};
    const double *const simd_opts[4] = {o20, o14, o10, o5};

    #define EXPANDED_LEN (10 * 20)
    double high_exp[EXPANDED_LEN], low_exp[EXPANDED_LEN], close_exp[EXPANDED_LEN];
    for (int i = 0; i < 20; i++) {
        for (int j = 0; j < 10; j++) {
            high_exp[i*10+j]  = high[j];
            low_exp[i*10+j]   = low[j];
            close_exp[i*10+j] = close[j];
        }
    }
    const double *expanded_inputs[CCI_INPUTS] = {high_exp, low_exp, close_exp};

    bool optional_outputs[3] = {true, true, true};

    CSimdResult r = cci_simd_by_options(expanded_inputs, EXPANDED_LEN,
                                        simd_opts, 4, optional_outputs, 3);

    for (uintptr_t i = 0; i < r.num_results; i++) {
        double *cci      = r.outputs[i][0];
        double *sma      = r.outputs[i][1];
        double *md       = r.outputs[i][2];
        double *typprice = r.outputs[i][3];
        cci_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{20.0} // period

    // Request all optional outputs (sma, md, typprice)
    res, st, _ := indicators.Cci.Indicator(high, low, close, options, []bool{true, true, true})
    fmt.Println(res.Rows[0]) // cci (primary)
    fmt.Println(res.Rows[1]) // "sma" (optional — requested)
    fmt.Println(res.Rows[2]) // "md" (optional — requested)
    fmt.Println(res.Rows[3]) // "typprice" (optional — requested)
    res.Close()
    st.Close()
    ```

=== "Java"

    `cci` exposes 3 optional outputs: `sma`, `md`, `typprice`. Pass a boolean mask as the third argument — one `boolean` per optional output, in order.

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Cci;

    // (high/low/close series as in the Basic tab)
    boolean[] mask = {true, true, true}; // sma, md, typprice
    Outcome oc = Cci.indicator(new double[][] {high, low, close}, new double[] {20.0}, mask);
    try (Result res = oc.result()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // sma
    }
    oc.state().close();
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    high  = np.array([82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00], dtype=np.float64)
    low   = np.array([81.29, 80.64, 81.31, 82.65, 83.07,
                      83.11, 82.49, 82.30, 84.15, 84.11], dtype=np.float64)
    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.cci.indicator(
        [high, low, close], [20.0],
        optional_outputs=[True, True, True],
    )

    cci      = outputs[0]  # cci (primary)
    sma      = outputs[1]  # sma (optional — requested)
    md       = outputs[2]  # md (optional — requested)
    typprice = outputs[3]  # typprice (optional — requested)
    ```

=== "Node.js"

    `cci` exposes 3 optional outputs: `sma`, `md`, `typprice`.

    ```javascript
    const [allOut] = ti.cci.indicator([high, low, close], [20], [true, true, true]);
    const cci      = allOut[0]; // primary
    const sma      = allOut[1]; // optional 0: sma
    const md       = allOut[2]; // optional 1: md
    const typprice = allOut[3]; // optional 2: typprice

    // Request only sma
    const [partial] = ti.cci.indicator([high, low, close], [20], [true, false, false]);
    ```


=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.cci.indicator([high, low, close], [20], [true, true, true]);
    const cci      = allOut[0]; // primary
    const sma      = allOut[1]; // optional 0: sma
    const md       = allOut[2]; // optional 1: md
    const typprice = allOut[3]; // optional 2: typprice

    // Request only sma
    const [partial] = ti.cci.indicator([high, low, close], [20], [true, false, false]);
    ```
### SIMD

=== "Rust"

    **By assets** — same period applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::cci::{Cci, Indicator};

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

    let results = Cci::indicator_by_assets::<4>(&inputs, &[20.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```rust
    use tulip_rs::indicators::cci::{Cci, IndicatorByOptions};

    let high  = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low   = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11_f64];
    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 1]; 4] = [&[10.0], &[14.0], &[20.0], &[30.0]];
    let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
    let results = Cci::indicator_by_options::<4>(&inputs, &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Period set {}: {:?}", i + 1, opt_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel:

    ```c
    #include <tulip_rs_ffi.h>

    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double c1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    double h2[10], l2[10], c2[10];
    for (int i = 0; i < 10; i++) { h2[i] = h1[i]*1.1; l2[i] = l1[i]*1.1; c2[i] = c1[i]*1.1; }

    double h3[10], l3[10], c3[10];
    for (int i = 0; i < 10; i++) { h3[i] = 90+i*0.5+h1[i]*0.1; l3[i] = 90+i*0.5+l1[i]*0.1; c3[i] = 90+i*0.5+c1[i]*0.1; }

    double h4[10], l4[10], c4[10];
    for (int i = 0; i < 10; i++) { h4[i] = 100-i*0.3+h1[i]*0.05; l4[i] = 100-i*0.3+l1[i]*0.05; c4[i] = 100-i*0.3+c1[i]*0.05; }

    const double *const asset1[CCI_INPUTS] = {h1, l1, c1};
    const double *const asset2[CCI_INPUTS] = {h2, l2, c2};
    const double *const asset3[CCI_INPUTS] = {h3, l3, c3};
    const double *const asset4[CCI_INPUTS] = {h4, l4, c4};

    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};
    double options[CCI_OPTIONS] = {20.0};

    CSimdResult r = cci_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Asset %zu: [", i+1);
        for (uintptr_t j = 0; j < r.output_lens[i][0]; j++) {
            printf("%.4f", r.outputs[i][0][j]);
            if (j+1 < r.output_lens[i][0]) printf(", ");
        }
        printf("]\n");
        cci_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    #include <tulip_rs_ffi.h>

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    double o10[] = {10.0}, o14[] = {14.0}, o20[] = {20.0}, o30[] = {30.0};
    const double *const simd_opts[4] = {o10, o14, o20, o30};

    #define EXPANDED_LEN (10 * 20)
    double high_exp[EXPANDED_LEN], low_exp[EXPANDED_LEN], close_exp[EXPANDED_LEN];
    for (int i = 0; i < 20; i++) {
        for (int j = 0; j < 10; j++) {
            high_exp[i*10+j] = high[j];
            low_exp[i*10+j]  = low[j];
            close_exp[i*10+j] = close[j];
        }
    }
    const double *expanded_inputs[CCI_INPUTS] = {high_exp, low_exp, close_exp};

    CSimdResult r = cci_simd_by_options(expanded_inputs, EXPANDED_LEN,
                                        simd_opts, 4, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        printf("Period %zu: [", simd_opts[i][0]);
        for (uintptr_t j = 0; j < r.output_lens[i][0]; j++) {
            printf("%.4f", r.outputs[i][0][j]);
            if (j+1 < r.output_lens[i][0]) printf(", ");
        }
        printf("]\n");
        cci_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}
    c1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    h2, l2, c2 := h1, l1, c1
    h3, l3, c3 := h1, l1, c1
    h4, l4, c4 := h1, l1, c1

    assets := [][indicators.CciInputs][]float64{{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}}
    sim, _ := indicators.Cci.SimdByAssets(assets, []float64{20.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85,
                      83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11}
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Cci.SimdByOptions(high, low, close, [][]float64{{10}, {14}, {20}, {30}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same period applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Cci;

    double[] h1 = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double[] l1 = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double[] c1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};

    // Reuse the same data for assets 2–4 in this example
    double[] h2 = h1; double[] l2 = l1; double[] c2 = c1;
    double[] h3 = h1; double[] l3 = l1; double[] c3 = c1;
    double[] h4 = h1; double[] l4 = l1; double[] c4 = c1;

    // Each entry lists one asset's input series (h1..c4 as in the C tab).
    double[][][] assets = {{h1, l1, c1}, {h2, l2, c2}, {h3, l3, c3}, {h4, l4, c4}};
    try (SimdResult sim = Cci.simdByAssets(assets, new double[] {20.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Cci;

    // (high/low/close series as in the Basic tab)
    try (SimdResult sim = Cci.simdByOptions(new double[][] {high, low, close},
            new double[][] {{10}, {14}, {20}, {30}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period set %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

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

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice(), close.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1), close.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9), close.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02), close.map(v => v * 1.02)],
    ];
    const [results] = ti.cci.simdByAssets(simdInputs, [20]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[10], [14], [20], [30]];
    const [results] = ti.cci.simdByOptions([high, low, close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
