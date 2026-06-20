#[cfg(test)]
mod tests {
    use tulip_rs::indicators::tr::indicator as rust_tr;
    use tulip_rs::indicators::vortex::{
        indicator as rust_vortex, indicator_by_assets, indicator_by_options, min_data,
        TIndicatorState,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    // Typical OHLC sample — high > close > low on every bar
    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    // Options: [period] — 4 × 15 = 60 bars satisfies all periods up to 50
    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [50.0]];

    /// Repeat the 15-bar sample four times → 60 bars.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
        }
        (high_vec, low_vec, close_vec)
    }

    fn get_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }

    // -------------------------------------------------------------------------
    // Vortex outputs:
    //   outputs[0] = vi_up
    //   outputs[1] = vi_down
    //   outputs[2] = tr (optional, enabled via Some(&[true]))
    //
    // The optional TR output has length n-1 (same as the standalone tr indicator).
    // The vortex TR values at indices [period..n-1] are computed by the main
    // calculation loop using the same calc_tr function as the standalone tr
    // indicator, so those positions are bit-for-bit identical.
    //
    // Indices 0..period of the vortex TR output are either uninitialised (index 0)
    // or filled by the warm-up phase with a different index offset, so only the
    // slice [period..] is compared against the standalone tr indicator.
    // -------------------------------------------------------------------------

    /// Verify output lengths and that the vortex optional TR output matches the
    /// standalone Rust TR indicator for the main computation range [period..].
    #[test]
    fn test_vortex_indicator() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let period = options[0] as usize;

            // Vortex with optional TR enabled
            let (vortex_outputs, _) = rust_vortex(&inputs, &options, Some(&[true]))
                .expect("Rust Vortex indicator failed");

            // Standalone Rust TR (options is empty — OPTIONS_WIDTH = 0)
            let (tr_outputs, _) = rust_tr(&inputs, &[], None).expect("Rust TR indicator failed");

            let n = high.len();
            let expected_main_len = n - period - 1; // output_length(n, options)
            let expected_tr_len = n - 1; // tr output_length

            // Output length checks
            assert_eq!(
                vortex_outputs[0].len(),
                expected_main_len,
                "vi_up length mismatch for options {:?}",
                options
            );
            assert_eq!(
                vortex_outputs[1].len(),
                expected_main_len,
                "vi_down length mismatch for options {:?}",
                options
            );
            assert_eq!(
                vortex_outputs[2].len(),
                expected_tr_len,
                "optional TR length mismatch for options {:?}",
                options
            );
            assert_eq!(
                tr_outputs[0].len(),
                expected_tr_len,
                "standalone TR length mismatch for options {:?}",
                options
            );

            // Compare the main-loop portion: vortex_tr[period..] must equal
            // standalone_tr[period..] (both cover bars period+1 .. n-1).
            for i in period..expected_tr_len {
                assert_eq!(
                    vortex_outputs[2][i], tr_outputs[0][i],
                    "TR mismatch at index {} for options {:?}: vortex={}, standalone={}",
                    i, options, vortex_outputs[2][i], tr_outputs[0][i]
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // Database: optional TR vs standalone TR (main-loop range)
    // -------------------------------------------------------------------------

    #[test]
    fn test_vortex_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let n = high.len();

            for options in OPTIONS_LIST {
                let period = options[0] as usize;
                if n < min_data(&options) {
                    continue; // skip stocks too short for this period
                }

                // Vortex with optional TR enabled
                let (vortex_outputs, _) = rust_vortex(&inputs, &options, Some(&[true]))
                    .expect("Rust Vortex indicator failed");

                // Standalone Rust TR
                let (tr_outputs, _) =
                    rust_tr(&inputs, &[], None).expect("Rust TR indicator failed");

                let expected_tr_len = n - 1;

                assert_eq!(
                    vortex_outputs[2].len(),
                    expected_tr_len,
                    "optional TR length mismatch for stock {} options {:?}",
                    stock_symbol,
                    options
                );
                assert_eq!(
                    tr_outputs[0].len(),
                    expected_tr_len,
                    "standalone TR length mismatch for stock {} options {:?}",
                    stock_symbol,
                    options
                );

                // Compare main-loop range: indices [period..n-1]
                for i in period..expected_tr_len {
                    assert_eq!(
                        vortex_outputs[2][i],
                        tr_outputs[0][i],
                        "TR mismatch at index {} for stock {} options {:?}: vortex={}, standalone={}",
                        i,
                        stock_symbol,
                        options,
                        vortex_outputs[2][i],
                        tr_outputs[0][i]
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Database: batch_indicator continuation vs full indicator (vi_up, vi_down)
    // -------------------------------------------------------------------------

    #[test]
    fn test_vortex_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                if n < min_data(&options) {
                    continue;
                }

                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

                // Full indicator on all data (reference)
                let (full_outputs, _) = rust_vortex(&inputs_rust, &options, None)
                    .expect("Rust Vortex indicator failed");

                // Streaming: seed on first chunk, then feed the rest in CHUNK_SIZE slices
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let seed_len = min_data(&options).max(CHUNK_SIZE);
                let chunk_inputs = [&high[..seed_len], &low[..seed_len], &close[..seed_len]];

                let (first_outputs, mut state) = rust_vortex(&chunk_inputs, &options, None)
                    .expect("Vortex indicator (seed) failed");

                for output_idx in 0..first_outputs.len() {
                    batch_full_outputs[output_idx].extend_from_slice(&first_outputs[output_idx]);
                }

                let mut high_chunks = high[seed_len..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[seed_len..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[seed_len..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_outputs = state
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("Vortex batch_indicator failed");
                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = state
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("Vortex batch_indicator (remainder) failed");
                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                // Compare vi_up (index 0) and vi_down (index 1)
                let band_names = ["vi_up", "vi_down"];
                for output_idx in 0..2 {
                    assert_eq!(
                        full_outputs[output_idx].len(),
                        batch_full_outputs[output_idx].len(),
                        "{} length mismatch for stock {} options {:?}",
                        band_names[output_idx],
                        stock_symbol,
                        options
                    );

                    for (i, (&full_val, &batch_val)) in full_outputs[output_idx]
                        .iter()
                        .zip(batch_full_outputs[output_idx].iter())
                        .enumerate()
                    {
                        assert_eq!(
                            full_val,
                            batch_val,
                            "{} state mismatch at index {} for stock {} options {:?}: full={}, batch={}",
                            band_names[output_idx],
                            i,
                            stock_symbol,
                            options,
                            full_val,
                            batch_val
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: outputs match scalar vortex per asset (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_vortex_simd_by_assets_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 3] = [&stock_data[0].1, &stock_data[0].2, &stock_data[0].3];
            let asset1: [&[f64]; 3] = [&stock_data[1].1, &stock_data[1].2, &stock_data[1].3];
            let asset2: [&[f64]; 3] = [&stock_data[2].1, &stock_data[2].2, &stock_data[2].3];
            let asset3: [&[f64]; 3] = [&stock_data[3].1, &stock_data[3].2, &stock_data[3].3];
            let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by assets Vortex failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) =
                    rust_vortex(&scalar_inputs, &options, None).expect("Rust Vortex failed");

                let simd_vi_up = &simd_results[asset_idx][0];
                let simd_vi_down = &simd_results[asset_idx][1];

                assert_eq!(
                    simd_vi_up.len(),
                    scalar_outputs[0].len(),
                    "vi_up length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    simd_vi_down.len(),
                    scalar_outputs[1].len(),
                    "vi_down length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&sv, &rv)) in simd_vi_up.iter().zip(scalar_outputs[0].iter()).enumerate() {
                    assert_eq!(sv, rv,
                        "vi_up mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
                for (i, (&sv, &rv)) in simd_vi_down
                    .iter()
                    .zip(scalar_outputs[1].iter())
                    .enumerate()
                {
                    assert_eq!(sv, rv,
                        "vi_down mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: outputs match scalar vortex per period (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_vortex_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by options Vortex failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                if high.len() < min_data(options) {
                    continue;
                }
                let (scalar_outputs, _) =
                    rust_vortex(&inputs, options, None).expect("Rust Vortex failed");

                let simd_vi_up = &simd_results[opt_idx][0];
                let simd_vi_down = &simd_results[opt_idx][1];

                assert_eq!(
                    simd_vi_up.len(),
                    scalar_outputs[0].len(),
                    "vi_up length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    simd_vi_down.len(),
                    scalar_outputs[1].len(),
                    "vi_down length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&sv, &rv)) in simd_vi_up.iter().zip(scalar_outputs[0].iter()).enumerate() {
                    assert_eq!(sv, rv,
                        "vi_up mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
                for (i, (&sv, &rv)) in simd_vi_down
                    .iter()
                    .zip(scalar_outputs[1].iter())
                    .enumerate()
                {
                    assert_eq!(sv, rv,
                        "vi_down mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: first 1000 bars via SIMD, rest via batch_indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_vortex_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 3] = [
                &stock_data[0].1[..FIRST_CHUNK],
                &stock_data[0].2[..FIRST_CHUNK],
                &stock_data[0].3[..FIRST_CHUNK],
            ];
            let asset1: [&[f64]; 3] = [
                &stock_data[1].1[..FIRST_CHUNK],
                &stock_data[1].2[..FIRST_CHUNK],
                &stock_data[1].3[..FIRST_CHUNK],
            ];
            let asset2: [&[f64]; 3] = [
                &stock_data[2].1[..FIRST_CHUNK],
                &stock_data[2].2[..FIRST_CHUNK],
                &stock_data[2].3[..FIRST_CHUNK],
            ];
            let asset3: [&[f64]; 3] = [
                &stock_data[3].1[..FIRST_CHUNK],
                &stock_data[3].2[..FIRST_CHUNK],
                &stock_data[3].3[..FIRST_CHUNK],
            ];
            let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by assets failed on first chunk");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let mut batch_vi_up = simd_first[asset_idx][0].clone();
                let mut batch_vi_down = simd_first[asset_idx][1].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("batch_indicator failed");
                    batch_vi_up.extend_from_slice(&chunk_outputs[0]);
                    batch_vi_down.extend_from_slice(&chunk_outputs[1]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_vi_up.extend_from_slice(&chunk_outputs[0]);
                    batch_vi_down.extend_from_slice(&chunk_outputs[1]);
                }

                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) =
                    rust_vortex(&scalar_inputs, &options, None).expect("Rust Vortex failed");

                assert_eq!(
                    batch_vi_up.len(),
                    scalar_outputs[0].len(),
                    "vi_up length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    batch_vi_down.len(),
                    scalar_outputs[1].len(),
                    "vi_down length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&bv, &rv)) in batch_vi_up.iter().zip(scalar_outputs[0].iter()).enumerate()
                {
                    assert_eq!(bv, rv,
                        "vi_up mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
                for (i, (&bv, &rv)) in batch_vi_down
                    .iter()
                    .zip(scalar_outputs[1].iter())
                    .enumerate()
                {
                    assert_eq!(bv, rv,
                        "vi_down mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: first 1000 bars via SIMD, rest via batch_indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_vortex_simd_by_options_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);

            let first_inputs = [
                &high[..FIRST_CHUNK],
                &low[..FIRST_CHUNK],
                &close[..FIRST_CHUNK],
            ];
            let (simd_first, mut states) =
                indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD by options failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch_vi_up = simd_first[opt_idx][0].clone();
                let mut batch_vi_down = simd_first[opt_idx][1].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("batch_indicator failed");
                    batch_vi_up.extend_from_slice(&chunk_outputs[0]);
                    batch_vi_down.extend_from_slice(&chunk_outputs[1]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_vi_up.extend_from_slice(&chunk_outputs[0]);
                    batch_vi_down.extend_from_slice(&chunk_outputs[1]);
                }

                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) =
                    rust_vortex(&scalar_inputs, options, None).expect("Rust Vortex failed");

                assert_eq!(
                    batch_vi_up.len(),
                    scalar_outputs[0].len(),
                    "vi_up length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    batch_vi_down.len(),
                    scalar_outputs[1].len(),
                    "vi_down length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&bv, &rv)) in batch_vi_up.iter().zip(scalar_outputs[0].iter()).enumerate()
                {
                    assert_eq!(bv, rv,
                        "vi_up mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
                for (i, (&bv, &rv)) in batch_vi_down
                    .iter()
                    .zip(scalar_outputs[1].iter())
                    .enumerate()
                {
                    assert_eq!(bv, rv,
                        "vi_down mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets optional outputs: TR must match scalar optional TR
    // -------------------------------------------------------------------------

    #[test]
    fn test_vortex_simd_by_assets_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 3] = [&stock_data[0].1, &stock_data[0].2, &stock_data[0].3];
            let asset1: [&[f64]; 3] = [&stock_data[1].1, &stock_data[1].2, &stock_data[1].3];
            let asset2: [&[f64]; 3] = [&stock_data[2].1, &stock_data[2].2, &stock_data[2].3];
            let asset3: [&[f64]; 3] = [&stock_data[3].1, &stock_data[3].2, &stock_data[3].3];
            let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &options, Some(&[true]))
                .expect("SIMD by-assets Vortex with optional outputs failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) = rust_vortex(&scalar_inputs, &options, Some(&[true]))
                    .expect("Scalar Vortex with optional outputs failed");

                // Primary outputs: vi_up [0], vi_down [1]
                for out_idx in 0..2 {
                    let simd_out = &simd_results[asset_idx][out_idx];
                    let scalar_out = &scalar_outputs[out_idx];
                    assert_eq!(
                        simd_out.len(), scalar_out.len(),
                        "output[{out_idx}] length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out).enumerate() {
                        assert_eq!(
                            sv, rv,
                            "output[{out_idx}] mismatch at index {i}: simd={sv}, scalar={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }

                // TR optional output (index 2)
                let simd_tr = &simd_results[asset_idx][2];
                let scalar_tr = &scalar_outputs[2];
                assert_eq!(
                    simd_tr.len(),
                    scalar_tr.len(),
                    "TR length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_tr.iter().zip(scalar_tr).enumerate() {
                    if sv != rv {
                        let start = i.saturating_sub(5);
                        let end = (i + 6).min(simd_tr.len());
                        println!(
                            "TR mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                            &simd_tr[start..end],
                            &scalar_tr[start..end]
                        );
                        panic!(
                            "TR mismatch at index {i}: simd={sv}, scalar={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
                println!(
                    "\u{2713} SIMD by-assets optional outputs match scalar for stock={stock_symbol}, options={options:?}"
                );
            }
        }
        println!("\u{2713} All SIMD by-assets Vortex optional output tests passed!");
    }

    // -------------------------------------------------------------------------
    // SIMD by-options optional outputs: TR must match scalar optional TR
    // -------------------------------------------------------------------------

    #[test]
    fn test_vortex_simd_by_options_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (_stock_symbol, stock_data) in data.iter().take(4) {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, Some(&[true]))
                .expect("SIMD by-options Vortex with optional outputs failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_outputs, _) = rust_vortex(&inputs, options, Some(&[true]))
                    .expect("Scalar Vortex with optional outputs failed");

                // Primary outputs: vi_up [0], vi_down [1]
                for out_idx in 0..2 {
                    let simd_out = &simd_results[opt_idx][out_idx];
                    let scalar_out = &scalar_outputs[out_idx];
                    assert_eq!(
                        simd_out.len(),
                        scalar_out.len(),
                        "output[{out_idx}] length mismatch: opt_idx={opt_idx}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out).enumerate() {
                        assert_eq!(
                            sv, rv,
                            "output[{out_idx}] mismatch at index {i}: simd={sv}, scalar={rv}, \
                             opt_idx={opt_idx}"
                        );
                    }
                }

                // TR optional output (index 2)
                let simd_tr = &simd_results[opt_idx][2];
                let scalar_tr = &scalar_outputs[2];
                assert_eq!(
                    simd_tr.len(),
                    scalar_tr.len(),
                    "TR length mismatch: opt_idx={opt_idx}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_tr.iter().zip(scalar_tr).enumerate() {
                    if sv != rv {
                        let start = i.saturating_sub(5);
                        let end = (i + 6).min(simd_tr.len());
                        println!(
                            "TR mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                            &simd_tr[start..end],
                            &scalar_tr[start..end]
                        );
                        panic!(
                            "TR mismatch at index {i}: simd={sv}, scalar={rv}, opt_idx={opt_idx}"
                        );
                    }
                }
                println!(
                    "\u{2713} SIMD by-options optional outputs match scalar for opt_idx={opt_idx}, options={options:?}"
                );
            }
        }
        println!("\u{2713} All SIMD by-options Vortex optional output tests passed!");
    }

    }
