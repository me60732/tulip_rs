# MD — Mean Deviation — `md`

The mean of the absolute deviations of each bar from the rolling mean over `period` bars. Similar to standard deviation but uses absolute rather than squared differences.

**Inputs:** `[real]` | **Options:** `[period]` | **Outputs:** `[md]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::md::{Md, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];
    let (outputs, mut state) = Md::indicator(&[close.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial = close[..8].to_vec();
    let (outputs2, mut state) = Md::indicator(&[partial.as_slice()], &[14.0], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_close = vec![85.53_f64];
    let continued = state.batch_indicator(&[new_close.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[MD_INPUTS] = {close};
    double options[MD_OPTIONS] = {14.0}; // period

    /* Full computation */
    CIndicatorResult r = md_indicator(inputs, 10, options, NULL, 0);
    /* r.outputs[0] -> the MD(14) series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    md_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = md_indicator(inputs, 8, options, NULL, 0);
    double new_close[] = {85.53};
    const double *new_inputs[MD_INPUTS] = {new_close};
    CBatchResult b = md_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> MD values for just the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    md_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}
    options := []float64{14.0}

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Md.Indicator(close, options, nil)
    fmt.Println(res.Rows[0]) // MD(14) values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Md.Indicator(close[:8], options, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(close[8:], nil)
    fmt.Println(batch.Rows[0]) // continued MD values
    batch.Close()
    st2.Close()
    ```

=== "Java"

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Md;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    double[] options = {14.0};

    // Full computation — output rows are zero-copy views, valid until close().
    Outcome oc = Md.indicator(new double[][] {close}, options);
    try (Result res = oc.result(); State st = oc.state()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // MD(14) values
    }

    // Partial computation + state continuation.
    int n = 8;
    Outcome p = Md.indicator(new double[][] {java.util.Arrays.copyOfRange(close, 0, n)}, options);
    try (Result pr = p.result(); State st = p.state()) {
        Result br = st.batch(new double[][] {java.util.Arrays.copyOfRange(close, n, 10)});
        try (br) {
            System.out.println(java.util.Arrays.toString(br.toDoubleArray(0))); // continued MD
        }
    }
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.md.indicator([close], [14.0])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const close = Float64Array.from([81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29]);

    const [outputs, state] = ti.md.indicator([close], [14]);
    console.log('MD(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.md.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued MD:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const close = [81.59, 81.06, 82.87, 83.00, 83.61,
                   83.15, 82.84, 83.99, 84.55, 84.36,
                   85.53, 86.54, 86.89, 87.77, 87.29];

    const [outputs, state] = ti.md.indicator([close], [14]);
    console.log('MD(14):', outputs[0]);

    // State continuation
    const [, state2] = ti.md.indicator([close.slice(0, -5)], [14]);
    const continued = state2.batchIndicator([close.slice(-5)]);
    console.log('Continued MD:', continued[0]);
    ```

### Optional Outputs

=== "Rust"

    `md` exposes 1 optional output: `sma`. Pass a boolean mask as the third argument — one `bool` per optional output, in order.

    ```rust
    use tulip_rs::indicators::md::{Md, Indicator, TIndicatorState};

    let close = vec![81.59, 81.06, 82.87, 83.00, 83.61,
                     83.15, 82.84, 83.99, 84.55, 84.36_f64];

    let mask = [true]; // one per optional output
    let (outputs, _state) = Md::indicator(&[close.as_slice()], &[10.0], Some(&mask)).unwrap();

    let md  = &outputs[0]; // md (primary)
    let sma = &outputs[1]; // sma (optional — requested)
    ```

=== "C"

    `md` exposes 1 optional output: `sma`. Pass a boolean mask as the third argument.

    ```c
    #include "tulip_rs_ffi.h"

    double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    const double *inputs[MD_INPUTS] = {close};
    double options[MD_OPTIONS] = {10.0}; // period
    bool optional_outputs[1] = {true}; // sma

    CIndicatorResult r = md_indicator(inputs, 10, options, optional_outputs, 1);
    /* r.outputs[0] -> MD(10), r.outputs[1] -> SMA (optional) */
    tulip_ffi_result_free(r);
    md_state_free(r.state);
    ```

=== "Go"

    `md` exposes 1 optional output: `sma`. Pass a boolean mask as the third argument.

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    mask := []bool{true} // one per optional output (sma)
    res, _st, _ := indicators.Md.Indicator(close, []float64{10.0}, mask)

    md := res.Rows[0] // md (primary)
    sma := res.Rows[1] // sma (optional — requested)
    res.Close()
    ```

=== "Java"

    `md` exposes 1 optional output: `sma`. Pass a boolean mask as the third argument — one `boolean` per optional output.

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Md;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};
    boolean[] mask = {true}; // sma
    Outcome oc = Md.indicator(new double[][] {close}, new double[] {10.0}, mask);
    try (Result res = oc.result()) {
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(0))); // md (primary)
        System.out.println(java.util.Arrays.toString(res.toDoubleArray(1))); // sma (optional — requested)
    }
    oc.state().close();
    ```

=== "Python"

    ```python
    import numpy as np
    import tulip_rs

    close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

    outputs, state = tulip_rs.indicators.md.indicator(
        [close], [10.0],
        optional_outputs=[True],
    )

    md  = outputs[0]  # md (primary)
    sma = outputs[1]  # sma (optional — requested)
    ```

=== "Node.js"

    `md` exposes 1 optional output: `sma`.

    ```javascript
    const [allOut] = ti.md.indicator([close], [14], [true]);
    const md  = allOut[0]; // primary
    const sma = allOut[1]; // optional 0: sma
    ```

=== "WASM"

    The WASM API is identical to Node.js — pass the boolean mask as the third argument.

    ```javascript
    const [allOut] = ti.md.indicator([close], [14], [true]);
    const md  = allOut[0]; // primary
    const sma = allOut[1]; // optional 0: sma
    ```
### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::md::{Md, Indicator};

    let inputs: [&[&[f64]; 1]; 4] = [&[a1.as_slice()], &[a2.as_slice()], &[a3.as_slice()], &[a4.as_slice()]];
    let results = Md::indicator_by_assets::<4>(&inputs, &[14.0], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same period applied to 4 assets in parallel (N must be 2/4/8/16):

    ```c
    double a1[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double a2[] = {81.59*1.2, 81.06*1.2, 82.87*1.2, 83.00*1.2, 83.61*1.2, 83.15*1.2, 82.84*1.2, 83.99*1.2, 84.55*1.2, 84.36*1.2};
    double a3[] = {90.0+0.5*0+81.59*0.1, 90.0+0.5*1+81.06*0.1, 90.0+0.5*2+82.87*0.1, 90.0+0.5*3+83.00*0.1, 90.0+0.5*4+83.61*0.1, 90.0+0.5*5+83.15*0.1, 90.0+0.5*6+82.84*0.1, 90.0+0.5*7+83.99*0.1, 90.0+0.5*8+84.55*0.1, 90.0+0.5*9+84.36*0.1};
    double a4[] = {100.0-0.3*0+81.59*0.05, 100.0-0.3*1+81.06*0.05, 100.0-0.3*2+82.87*0.05, 100.0-0.3*3+83.00*0.05, 100.0-0.3*4+83.61*0.05, 100.0-0.3*5+83.15*0.05, 100.0-0.3*6+82.84*0.05, 100.0-0.3*7+83.99*0.05, 100.0-0.3*8+84.55*0.05, 100.0-0.3*9+84.36*0.05};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[MD_INPUTS] = {a1};
    const double *asset2[MD_INPUTS] = {a2};
    const double *asset3[MD_INPUTS] = {a3};
    const double *asset4[MD_INPUTS] = {a4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = md_simd_by_assets(simd_inputs, 4, 10, options, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        md_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```c
    double o7[] = {7.0}, o14[] = {14.0}, o21[] = {21.0}, o28[] = {28.0};
    const double *const simd_opts[4] = {o7, o14, o21, o28};

    CSimdResult r = md_simd_by_options(inputs, 10, simd_opts, 4, NULL, 0);
    /* r.outputs[i] -> results for option set i (periods 7/14/21/28) */
    for (uintptr_t i = 0; i < r.num_results; i++) md_state_free(r.states[i]);
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    a1 := []float64{81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36}

    // Reuse the same data for assets 2–4 in this example
    a2, a3, a4 := a1, a1, a1

    assets := [][indicators.MdInputs][]float64{{a1}, {a2}, {a3}, {a4}}
    sim, _ := indicators.Md.SimdByAssets(assets, []float64{14.0}, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```go
    close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                       83.15, 82.84, 83.99, 84.55, 84.36}

    sim2, _ := indicators.Md.SimdByOptions(close, [][]float64{{7}, {14}, {21}, {28}}, nil)
    for i, lanes := range sim2.Results {
        fmt.Printf("Period set %d: %v\n", i+1, lanes[0])
    }
    sim2.Close()
    ```

=== "Java"

    **By assets** — same period applied to 4 assets in parallel (N must be 2, 4, 8, or 16):

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Md;

    double[] a1 = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36};
    double[] a2 = {81.59*1.2, 81.06*1.2, 82.87*1.2, 83.00*1.2, 83.61*1.2, 83.15*1.2, 82.84*1.2, 83.99*1.2, 84.55*1.2, 84.36*1.2};
    double[] a3 = {90.0+0.5*0+81.59*0.1, 90.0+0.5*1+81.06*0.1, 90.0+0.5*2+82.87*0.1, 90.0+0.5*3+83.00*0.1, 90.0+0.5*4+83.61*0.1, 90.0+0.5*5+83.15*0.1, 90.0+0.5*6+82.84*0.1, 90.0+0.5*7+83.99*0.1, 90.0+0.5*8+84.55*0.1, 90.0+0.5*9+84.36*0.1};
    double[] a4 = {100.0-0.3*0+81.59*0.05, 100.0-0.3*1+81.06*0.05, 100.0-0.3*2+82.87*0.05, 100.0-0.3*3+83.00*0.05, 100.0-0.3*4+83.61*0.05, 100.0-0.3*5+83.15*0.05, 100.0-0.3*6+82.84*0.05, 100.0-0.3*7+83.99*0.05, 100.0-0.3*8+84.55*0.05, 100.0-0.3*9+84.36*0.05};

    // One entry per asset; each asset lists its INPUTS series.
    double[][][] assets = {{a1}, {a2}, {a3}, {a4}};
    try (SimdResult sim = Md.simdByAssets(assets, new double[] {14.0}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Asset %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }   // frees every lane state, then the SIMD buffers (contractual order)
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```java
    import org.tuliprs.*;
    import org.tuliprs.indicators.Md;

    double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                      83.15, 82.84, 83.99, 84.55, 84.36};

    try (SimdResult sim = Md.simdByOptions(new double[][] {close},
            new double[][] {{7}, {14}, {21}, {28}}, null)) {
        for (int i = 0; i < sim.numResults(); i++) {
            System.out.printf("Period set %d: %s%n", i + 1,
                java.util.Arrays.toString(sim.toDoubleArray(i, 0)));
        }
    }
    ```

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[a1], [a2], [a3], [a4]]
    outputs_list, states = tulip_rs.indicators.md.simd_by_assets(simd_inputs, [14.0])
    ```

    **By options** — same asset, N option sets in parallel:

    ```python
    simd_options = [[7.0], [14.0], [21.0], [28.0]]
    outputs_list, states = tulip_rs.indicators.md.simd_by_options([close], simd_options)
    ```

=== "Node.js"

    **By assets** — same period applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [[close.slice()], [close.map(v => v * 1.1)], [close.map(v => v * 0.9)], [close.map(v => v * 1.02)]];
    const [results] = ti.md.simdByAssets(simdInputs, [14]);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    **By options** — same asset, 4 different periods in parallel:

    ```javascript
    const simdOptions = [[7], [14], [21], [28]];
    const [results] = ti.md.simdByOptions([close], simdOptions);
    results.forEach((out, i) => console.log(`Period ${simdOptions[i][0]}:`, out[0]));
    ```
