# MAMA — MESA Adaptive Moving Average

MAMA (MESA Adaptive Moving Average) computes adaptive moving averages for price series.

## Inputs

- `real` (price series)

## Options

(None — the Go tab calls `Indicator(close, nil, nil)`)

## Outputs

- `mama` (MESA adaptive moving average)
- `fiba` (FIBA - second MESA component)

## Optional Outputs

The indicator exposes two optional outputs: `dc_period` and `alpha`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

---

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::mama::{Mama, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    // Options: [fast_limit, slow_limit]
    let (outputs, _state) = Mama::indicator(&[close.as_slice()], &[0.5, 0.05], None).unwrap();
    println!("MAMA:  {:?}", outputs[0]);
    println!("FAMA:  {:?}", outputs[1]);

    // State continuation
    let n = close.len() - 5;
    let partial = close[..n].to_vec();
    let (outputs2, mut state) = Mama::indicator(&[partial.as_slice()], &[0.5, 0.05], None).unwrap();
    println!("Partial MAMA: {:?}", outputs2[0]);

    let rest = close[n..].to_vec();
    let continued = state.batch_indicator(&[rest.as_slice()], None).unwrap();
    println!("Continued MAMA: {:?}", continued[0]);
    println!("Continued FAMA: {:?}", continued[1]);
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[MAMA_INPUTS] = {a1};
    const double *asset2[MAMA_INPUTS] = {a2};
    const double *asset3[MAMA_INPUTS] = {a3};
    const double *asset4[MAMA_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    bool optional_outputs[2] = {true, true}; // dc_period, alpha
    CSimdResult r = mama_simd_by_assets(simd_inputs, 4, 10, options, optional_outputs, 2);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's mama series, length r.output_lens[i][0] */
        /* r.outputs[i][1] -> asset i's fama */
        mama_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in one call:

    ```c
    double o03[] = {0.3}, o003[] = {0.03};
    double o04[] = {0.4}, o004[] = {0.04};
    double o05[] = {0.5}, o005[] = {0.05};
    double o06[] = {0.6}, o006[] = {0.06};

    const double *const simd_opts[4] = {
        (double[]){0.3, 0.03},
        (double[]){0.4, 0.04},
        (double[]){0.5, 0.05},
        (double[]){0.6, 0.06}
    };

    bool optional_outputs[2] = {true, true};
    CSimdResult r = mama_simd_by_options(inputs, 10, simd_opts, 4, optional_outputs, 2);
    /* r.outputs[i][0] -> asset i's mama for option set */
    for (uintptr_t i = 0; i < r.num_results; i++) mama_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}

    options := []float64{0.5, 0.05} // fast_limit, slow_limit

    res, st, _ := indicators.Mama.Indicator(close, options, nil)
    fmt.Printf("MAMA: %v\n", tulip.AsFloat64(res.Rows[0]))
    fmt.Printf("FAMA: %v\n", tulip.AsFloat64(res.Rows[1]))
    res.Close()
    st.Close()

    partial := close[:35]
    res2, state2, _ := indicators.Mama.Indicator(partial, options, nil)
    fmt.Printf("Partial MAMA: %v\n", tulip.AsFloat64(res2.Rows[0]))

    continued, _ := state2.Batch(close[35:], nil)
    fmt.Printf("Continued MAMA: %v\n", tulip.AsFloat64(continued.Rows[0]))
    continued.Close()
    state2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Mama;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double[] options = {0.5, 0.05}; // fast_limit, slow_limit

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Mama.indicator(new double[][] {close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // MAMA values
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // FAMA values
    }

    // Partial computation + state continuation.
    int n = 35;
    Outcome p = Mama.indicator(new double[][] {java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {java.util.Arrays.copyOfRange(close, n, 40)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued MAMA
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(1))); // continued FAMA
        }
    }
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.mama.indicator([close], [])
    print(outputs[0])  # mama values
    print(outputs[1])  # fiba values

    # State continuation
    continued = state.batch_indicator([np.array([85.20], dtype=np.float64)])
    print(continued[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.mama.indicator([close], []);
    console.log('MAMA:', outputs[0]);
    console.log('FIBA:', outputs[1]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.mama.indicator([close.slice(0, n)], []);
    const continued = state2.batchIndicator([close.slice(n)]);
    console.log('Continued MAMA:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.mama.indicator([close], []);
    console.log('MAMA:', outputs[0]);
    console.log('FIBA:', outputs[1]);

    // State continuation
    const n = close.length - 5;
    const [, state2] = ti.mama.indicator([close.slice(0, n)], []);
    const continued = state2.batchIndicator([close.slice(n)]);
    console.log('Continued MAMA:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `mama` exposes 2 optional outputs: `dc_period`, `alpha`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::mama::{Mama, Indicator, TIndicatorState};

    let close = vec![
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20_f64,
    ];

    let mask = [true, true]; // one per optional output
    let (outputs, _state) = Mama::indicator(&[close.as_slice()], &[0.5, 0.05], Some(&mask)).unwrap();

    let mama      = &outputs[0]; // mama (primary)
    let fama      = &outputs[1]; // fama (primary)
    let dc_period = &outputs[2]; // dc_period (optional — requested)
    let alpha     = &outputs[3]; // alpha (optional — requested)
    ```

=== "C"

    `mama` exposes 2 optional outputs: `dc_period`, `alpha`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                      85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                      88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                      90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double options[MAMA_OPTIONS] = {0.5, 0.05}; // fast_limit, slow_limit
    const double *inputs[MAMA_INPUTS] = {close};

    bool mask[2] = {true, true}; // one per optional output
    CIndicatorResult r = mama_indicator(inputs, 40, options, mask, 2);

    /* r.outputs[0] -> mama (primary) */
    /* r.outputs[1] -> fama (primary) */
    /* r.outputs[2] -> dc_period (optional — requested) */
    /* r.outputs[3] -> alpha (optional — requested) */

    tulip_ffi_result_free(r);
    mama_state_free(r.state);
    ```

=== "Go"

    ```go
    import (
        "fmt"
        "github.com/me60732/tulip_rs_go/indicators"
        "github.com/me60732/tulip_rs_go/tulip"
    )

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}

    options := []float64{0.5, 0.05} // fast_limit, slow_limit

    mask := []bool{true, true} // dc_period, alpha

    res, st, _ := indicators.Mama.Indicator(close, options, mask)
    fmt.Printf("MAMA: %v\n", tulip.AsFloat64(res.Rows[0]))
    fmt.Printf("FAMA: %v\n", tulip.AsFloat64(res.Rows[1]))
    fmt.Printf("dc_period: %v\n", tulip.AsFloat64(res.Rows[2]))
    fmt.Printf("alpha: %v\n", tulip.AsFloat64(res.Rows[3]))
    res.Close()
    st.Close()
    ```

=== "Java"

    `mama` exposes 2 optional outputs: `dc_period`, `alpha`. Pass a boolean mask as the third argument — one `boolean` per optional output, in order.

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Mama;

    // (close/options series as in the Basic tab)
    boolean[] mask = {true, true}; // dc_period, alpha
    Outcome oc = Mama.indicator(new double[][] {close}, new double[] {0.5, 0.05}, mask);
    try (Result res = oc.result()) {
        // row 0 = mama (primary), row 1 = fama (primary), row 2 = dc_period (optional), row 3 = alpha
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(2))); // dc_period
    }
    oc.state().close();
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    outputs, state = tulip_rs.indicators.mama.indicator(
        [close], [0.5, 0.05],
        optional_outputs=[True, True],
    )

    mama      = outputs[0]  # mama (primary)
    fama      = outputs[1]  # fama (primary)
    dc_period = outputs[2]  # dc_period (optional — requested)
    alpha     = outputs[3]  # alpha (optional — requested)
    ```

=== "Node.js"

    `mama` exposes 2 optional outputs: `dc_period`, `alpha`.

    ```javascript
    const [allOut] = ti.mama.indicator([close], [0.5, 0.05], [true, true]);
    const mama     = allOut[0]; // primary
    const fama     = allOut[1]; // primary
    const dcPeriod = allOut[2]; // optional 0: dc_period
    const alpha    = allOut[3]; // optional 1: alpha
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.mama.indicator([close], [0.5, 0.05], [true, true]);
    const mama     = allOut[0]; // primary
    const fama     = allOut[1]; // primary
    const dcPeriod = allOut[2]; // optional 0: dc_period
    const alpha    = allOut[3]; // optional 1: alpha
    ```

### SIMD

=== "Rust"

    **By assets** — same options applied to 4 assets in parallel:

    ```rust
    use tulip_rs::indicators::mama::{Mama, Indicator};

    let a1 = vec![81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let a2 = vec![86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36_f64];
    let a3 = vec![78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36_f64];
    let a4 = vec![83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05_f64];

    let inputs: [&[&[f64]; 1]; 4] = [
        &[a1.as_slice()],
        &[a2.as_slice()],
        &[a3.as_slice()],
        &[a4.as_slice()],
    ];

    let results = Mama::indicator_by_assets::<4>(&inputs, &[0.5, 0.05], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {} MAMA: {:?}", i + 1, asset_outputs[0]);
        println!("Asset {} FAMA: {:?}", i + 1, asset_outputs[1]);
    }
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```rust
    use tulip_rs::indicators::mama::{Mama, IndicatorByOptions};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let opts: [&[f64; 2]; 4] = [
        &[0.3, 0.03],
        &[0.4, 0.04],
        &[0.5, 0.05],
        &[0.6, 0.06],
    ];

    let results = Mama::indicator_by_options::<4>(&[close.as_slice()], &opts, None).unwrap();
    for (i, opt_outputs) in results.iter().enumerate() {
        println!("Option set {} MAMA: {:?}", i + 1, opt_outputs[0]);
        println!("Option set {} FAMA: {:?}", i + 1, opt_outputs[1]);
    }
    ```

=== "C"

    **By assets** — same options applied to 4 assets in one call (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double a3[] = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double a4[] = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[MAMA_INPUTS] = {a1};
    const double *asset2[MAMA_INPUTS] = {a2};
    const double *asset3[MAMA_INPUTS] = {a3};
    const double *asset4[MAMA_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    bool optional_outputs[2] = {true, true}; // dc_period, alpha
    CSimdResult r = mama_simd_by_assets(simd_inputs, 4, 10, options, optional_outputs, 2);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's mama series, length r.output_lens[i][0] */
        /* r.outputs[i][1] -> asset i's fama */
        mama_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different option sets in one call:

    ```c
    double o03[] = {0.3}, o003[] = {0.03};
    double o04[] = {0.4}, o004[] = {0.04};
    double o05[] = {0.5}, o005[] = {0.05};
    double o06[] = {0.6}, o006[] = {0.06};

    const double *const simd_opts[4] = {
        (double[]){0.3, 0.03},
        (double[]){0.4, 0.04},
        (double[]){0.5, 0.05},
        (double[]){0.6, 0.06}
    };

    bool optional_outputs[2] = {true, true};
    CSimdResult r = mama_simd_by_options(inputs, 10, simd_opts, 4, optional_outputs, 2);
    /* r.outputs[i][0] -> asset i's mama for option set */
    for (uintptr_t i = 0; i < r.num_results; i++) mama_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same options applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                       85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                       88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                       90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20}
    options := []float64{0.5, 0.05} // fast_limit, slow_limit

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := close, close, close

    assets := [][indicators.MamaInputs][]float64{{close}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Mama.SimdByAssets(assets, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d MAMA: %v\n", i+1, lanes[0])
        fmt.Printf("Asset %d FAMA: %v\n", i+1, lanes[1])
    }
    sim.Close()
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61}
    options := [][]float64{{0.3, 0.03}, {0.4, 0.04}, {0.5, 0.05}, {0.6, 0.06}}

    sim, _ := indicators.Mama.SimdByOptions(close, options, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Option set %d MAMA: %v\n", i+1, lanes[0])
        fmt.Printf("Option set %d FAMA: %v\n", i+1, lanes[1])
    }
    sim.Close()
    ```

=== "Java"

    **By assets** — same options applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Mama;

    double[] a1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
                   88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
                   90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20};
    double[] a2 = {86.59, 86.06, 87.87, 88.00, 88.61, 88.15, 87.84, 88.99, 89.55, 89.36};
    double[] a3 = {78.59, 78.06, 79.87, 80.00, 80.61, 80.15, 79.84, 80.99, 81.55, 81.36};
    double[] a4 = {83.22, 82.68, 84.53, 84.66, 85.28, 84.81, 84.50, 85.67, 86.24, 86.05};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1}, {a2}, {a3}, {a4}};
    try (SimdResult sim = Mama.simdByAssets(assets, new double[] {0.5, 0.05}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d MAMA: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0))); // mama
            System.out.printf("Asset %d FAMA: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1))); // fama
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Mama;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61};

    try (SimdResult sim = Mama.simdByOptions(new double[][] {close},
            new double[][] {{0.3, 0.03}, {0.4, 0.04}, {0.5, 0.05}, {0.6, 0.06}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Option set %d MAMA: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0))); // mama
            System.out.printf("Option set %d FAMA: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 1))); // fama
        }
    }
    ```

=== "Python"

    **By assets** — same options applied to N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_inputs = [[close], [close + 5.0], [close - 3.0], [close * 1.02]]
    outputs_list, states = tulip_rs.indicators.mama.simd_by_assets(simd_inputs, [0.5, 0.05])
    for i, out in enumerate(outputs_list):
        print(f"Asset {i + 1} MAMA: {out[0]}")
        print(f"Asset {i + 1} FAMA: {out[1]}")
    ```

    **By options** — same asset, N different option sets in parallel:

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36,
        85.53, 86.54, 86.89, 87.77, 87.29, 87.50, 88.10, 88.50, 87.90, 88.20,
        88.80, 89.10, 88.70, 89.30, 89.70, 90.10, 89.50, 90.20, 90.80, 91.10,
        90.50, 91.20, 91.80, 92.10, 91.50, 92.20, 92.80, 93.10, 92.50, 93.20,
    ], dtype=np.float64)

    simd_options = [
        [0.3, 0.03],
        [0.4, 0.04],
        [0.5, 0.05],
        [0.6, 0.06],
    ]
    outputs_list, states = tulip_rs.indicators.mama.simd_by_options([close], simd_options)
    for i, out in enumerate(outputs_list):
        print(f"Option set {i + 1} MAMA: {out[0]}")
        print(f"Option set {i + 1} FAMA: {out[1]}")
    ```

=== "Node.js"

    **By assets** — same options applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [close.slice()],
        [close.map(v => v + 5.0)],
        [close.map(v => v - 3.0)],
        [close.map(v => v * 1.02)],
    ];
    const [results] = ti.mama.simdByAssets(simdInputs, [0.5, 0.05]);
    results.forEach((out, i) => console.log(`Asset ${i + 1} MAMA:`, out[0], 'FAMA:', out[1]));
    ```

    **By options** — same asset, 4 different option sets in parallel:

    ```javascript
    const simdOptions = [[0.3, 0.03], [0.4, 0.04], [0.5, 0.05], [0.6, 0.06]];
    const [results] = ti.mama.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Option set ${i + 1} MAMA:`, out[0]));
    ```
