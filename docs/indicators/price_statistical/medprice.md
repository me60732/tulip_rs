# Median Price — `medprice`

`(High + Low) / 2` for each bar.

**Inputs:** `[high, low]` | **Options:** none | **Outputs:** `[medprice]`

### Basic

=== "Rust"

    ```rust
    use tulip_rs::indicators::medprice::{Medprice};

    let high = vec![82.15, 81.89, 83.03, 83.30, 83.85,
                    83.90, 83.33, 84.30, 84.84, 85.00_f64];
    let low  = vec![81.29, 80.64, 81.31, 82.65, 83.07,
                    83.11, 82.49, 82.30, 84.15, 84.11_f64];

    let inputs = [high.as_slice(), low.as_slice()];
    let (outputs, mut state) = Medprice::indicator(&inputs, &[], None).unwrap();
    println!("{:?}", outputs[0]);

    // State continuation — feed new bars without reprocessing history
    let partial_high   = high[..8].to_vec();
    let partial_low    = low[..8].to_vec();
    let (outputs2, mut state) = Medprice::indicator(&[partial_high.as_slice(), partial_low.as_slice()], &[], None).unwrap();
    println!("{:?}", outputs2[0]);

    let new_high   = vec![85.90_f64];
    let new_low    = vec![84.03_f64];
    let continued = state.batch_indicator(&[new_high.as_slice(), new_low.as_slice()], None).unwrap();
    println!("{:?}", continued[0]);
    ```

=== "C"

    ```c
    #include "tulip_rs_ffi.h"

    double high[] = {82.15, 81.89, 83.03, 83.30, 83.85,
                     83.90, 83.33, 84.30, 84.84, 85.00};
    double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07,
                     83.11, 82.49, 82.30, 84.15, 84.11};
    const double *inputs[MEDPRICE_INPUTS] = {high, low};
    /* MEDPRICE has 0 options (MEDPRICE_OPTIONS == 0) */

    /* Full computation */
    CIndicatorResult r = medprice_indicator(inputs, 10, NULL, NULL, 0);
    /* r.outputs[0] -> the MEDPRICE series, length r.output_lens[0] */
    tulip_ffi_result_free(r);
    medprice_state_free(r.state);

    /* Partial computation + state continuation */
    CIndicatorResult p = medprice_indicator(inputs, 8, NULL, NULL, 0);
    double new_high[] = {85.90};
    double new_low[]  = {84.03};
    const double *new_inputs[MEDPRICE_INPUTS] = {new_high, new_low};
    CBatchResult b = medprice_batch(p.state, new_inputs, 1, NULL, 0);
    /* b.outputs[0] -> MEDPRICE values for just the one new bar */
    tulip_ffi_batch_result_free(b);
    tulip_ffi_result_free(p);
    medprice_state_free(p.state);
    ```

=== "Go"

    ```go
    import "github.com/me60732/tulip_rs_go/indicators"

    high := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    low := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}

    // Full computation — Rows are zero-copy views, valid until Close.
    res, st, _ := indicators.Medprice.Indicator(high, low, nil, nil)
    fmt.Println(res.Rows[0]) // MedPrice values
    res.Close()
    st.Close()

    // Partial computation + state continuation.
    res2, st2, _ := indicators.Medprice.Indicator(high[:8], low[:8], nil, nil)
    res2.Close() // outputs consumed or closed; state stays live
    batch, _ := st2.Batch(high[8:], low[8:])
    fmt.Println(batch.Rows[0]) // continued MedPrice values
    batch.Close()
    st2.Close()
    ```

=== "Python"

    ```python
    outputs, state = tulip_rs.indicators.medprice.indicator([high, low], [])
    print(outputs[0])
    ```

=== "Node.js"

    ```javascript
    import * as ti from 'tulip-rs-node';

    const high = Float64Array.from([82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87]);
    const low  = Float64Array.from([81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01]);

    const [outputs, state] = ti.medprice.indicator([high, low], []);
    console.log('MedPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.medprice.indicator([high.slice(0, n), low.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued MedPrice:', continued[0]);
    ```

=== "WASM"

    ```javascript
    import { init } from 'tulip-rs-wasm';
    import * as ti from 'tulip-rs-wasm';

    await init(); // bundler resolves the WASM asset automatically

    const high = [82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98, 88.00, 87.87];
    const low  = [81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76, 87.17, 87.01];

    const [outputs, state] = ti.medprice.indicator([high, low], []);
    console.log('MedPrice:', outputs[0]);

    // State continuation
    const n = high.length - 5;
    const [, state2] = ti.medprice.indicator([high.slice(0, n), low.slice(0, n)], []);
    const continued = state2.batchIndicator([high.slice(n), low.slice(n)]);
    console.log('Continued MedPrice:', continued[0]);
    ```

### SIMD

=== "Rust"

    **By assets** — same options, N assets in parallel:

    ```rust
    use tulip_rs::indicators::medprice::{Medprice, Indicator};

    let inputs: [&[&[f64]; 2]; 4] = [
        &[h1.as_slice(), l1.as_slice()],
        &[h2.as_slice(), l2.as_slice()],
        &[h3.as_slice(), l3.as_slice()],
        &[h4.as_slice(), l4.as_slice()],
    ];
    let results = Medprice::indicator_by_assets::<4>(&inputs, &[], None).unwrap();
    for (i, asset_outputs) in results.iter().enumerate() {
        println!("Asset {}: {:?}", i + 1, asset_outputs[0]);
    }
    ```

=== "C"

    **By assets** — same no-options applied to 4 assets in parallel (N must be 2/4/8/16):

    ```c
    double h1[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00};
    double l1[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11};
    double h2[] = {82.15*1.2, 81.89*1.2, 83.03*1.2, 83.30*1.2, 83.85*1.2, 83.90*1.2, 83.33*1.2, 84.30*1.2, 84.84*1.2, 85.00*1.2};
    double l2[] = {81.29*1.2, 80.64*1.2, 81.31*1.2, 82.65*1.2, 83.07*1.2, 83.11*1.2, 82.49*1.2, 82.30*1.2, 84.15*1.2, 84.11*1.2};
    double h3[] = {90.0+0.5*0+82.15*0.1, 90.0+0.5*1+81.89*0.1, 90.0+0.5*2+83.03*0.1, 90.0+0.5*3+83.30*0.1, 90.0+0.5*4+83.85*0.1, 90.0+0.5*5+83.90*0.1, 90.0+0.5*6+83.33*0.1, 90.0+0.5*7+84.30*0.1, 90.0+0.5*8+84.84*0.1, 90.0+0.5*9+85.00*0.1};
    double l3[] = {90.0+0.5*0+81.29*0.1, 90.0+0.5*1+80.64*0.1, 90.0+0.5*2+81.31*0.1, 90.0+0.5*3+82.65*0.1, 90.0+0.5*4+83.07*0.1, 90.0+0.5*5+83.11*0.1, 90.0+0.5*6+82.49*0.1, 90.0+0.5*7+82.30*0.1, 90.0+0.5*8+84.15*0.1, 90.0+0.5*9+84.11*0.1};
    double h4[] = {100.0-0.3*0+82.15*0.05, 100.0-0.3*1+81.89*0.05, 100.0-0.3*2+83.03*0.05, 100.0-0.3*3+83.30*0.05, 100.0-0.3*4+83.85*0.05, 100.0-0.3*5+83.90*0.05, 100.0-0.3*6+83.33*0.05, 100.0-0.3*7+84.30*0.05, 100.0-0.3*8+84.84*0.05, 100.0-0.3*9+85.00*0.05};
    double l4[] = {100.0-0.3*0+81.29*0.05, 100.0-0.3*1+80.64*0.05, 100.0-0.3*2+81.31*0.05, 100.0-0.3*3+82.65*0.05, 100.0-0.3*4+83.07*0.05, 100.0-0.3*5+83.11*0.05, 100.0-0.3*6+82.49*0.05, 100.0-0.3*7+82.30*0.05, 100.0-0.3*8+84.15*0.05, 100.0-0.3*9+84.11*0.05};

    /* one [INPUTS]-long pointer array per asset */
    const double *asset1[MEDPRICE_INPUTS] = {h1, l1};
    const double *asset2[MEDPRICE_INPUTS] = {h2, l2};
    const double *asset3[MEDPRICE_INPUTS] = {h3, l3};
    const double *asset4[MEDPRICE_INPUTS] = {h4, l4};
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    /* MEDPRICE has no options (MEDPRICE_OPTIONS == 0), so pass NULL */
    CSimdResult r = medprice_simd_by_assets(simd_inputs, 4, 10, NULL, NULL, 0);
    for (uintptr_t i = 0; i < r.num_results; i++) {
        /* r.outputs[i][0] -> asset i's series, length r.output_lens[i][0] */
        medprice_state_free(r.states[i]);
    }
    tulip_ffi_simd_result_free(r);
    ```

=== "Go"

    **By assets** — same period applied to 4 assets in parallel (lane counts 2/4/8/16):

    ```go
    h1 := []float64{82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00}
    l1 := []float64{81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11}

    // Reuse the same data for assets 2–4 in this example
    h2, l2 := h1, l1
    h3, l3 := h1, l1
    h4, l4 := h1, l1

    assets := [][indicators.MedpriceInputs][]float64{{h1, l1}, {h2, l2}, {h3, l3}, {h4, l4}}
    sim, _ := indicators.Medprice.SimdByAssets(assets, nil, nil)
    for i, lanes := range sim.Results {
        fmt.Printf("Asset %d: %v\n", i+1, lanes[0])
    }
    sim.Close() // frees every lane state, then the SIMD buffers
    ```

    _This indicator has no options (MEDPRICE_OPTIONS == 0), so simd_by_options is not available._

=== "Python"

    **By assets** — same options, N assets in parallel (must be 2, 4, 8, or 16):

    ```python
    simd_inputs = [[h1, l1], [h2, l2], [h3, l3], [h4, l4]]
    outputs_list, states = tulip_rs.indicators.medprice.simd_by_assets(simd_inputs, [])
    ```

    _This indicator has no options, so by-options SIMD does not apply._

=== "Node.js"

    **By assets** — applied to 4 assets in parallel:

    ```javascript
    const simdInputs = [
        [high.slice(), low.slice()],
        [high.map(v => v * 1.1), low.map(v => v * 1.1)],
        [high.map(v => v * 0.9), low.map(v => v * 0.9)],
        [high.map(v => v * 1.02), low.map(v => v * 1.02)],
    ];
    const [results] = ti.medprice.simdByAssets(simdInputs, []);
    results.forEach((out, i) => console.log(`Asset ${i + 1}:`, out[0]));
    ```

    _This indicator has no options, so by-options SIMD does not apply._
