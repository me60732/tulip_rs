#[cfg(test)]
mod tests {
    use tulip_rs::indicators::elderray::{
        indicator as rust_elderray, indicator_by_assets, indicator_by_options, min_data,
        TIndicatorState,
    };
    use tulip_rs::indicators::ema::indicator as rust_ema;
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

    // Options: [period]
    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [50.0]];

    /// Expand sample data by repetition — 4 × 15 = 60 bars (enough for period 50).
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
    // Elder-ray outputs:
    //   outputs[0] = bull  = high[period+i] - ema[i]
    //   outputs[1] = bear  = low[period+i]  - ema[i]
    //   outputs[2] = ema   (optional, enabled via Some(&[true]))
    //
    // The optional EMA is computed from close using the same init_state + cycle
    // as the standalone ema::indicator, so the outputs are bit-for-bit identical.
    // -------------------------------------------------------------------------

    /// Verify that the optional EMA output equals the standalone Rust EMA, that
    /// bull = high - ema and bear = low - ema, and that bull != bear (since
    /// high != low across all sample bars).
    #[test]
    fn test_elderray_indicator() {
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            let period = options[0] as usize;

            // Elder-ray with optional EMA enabled
            let (outputs, _) = rust_elderray(
                &[high.as_slice(), low.as_slice(), close.as_slice()],
                &options,
                Some(&[true]),
            )
            .expect("Rust Elder-ray indicator failed");

            // Standalone Rust EMA on close (same code path)
            let (ema_outputs, _) =
                rust_ema(&[close.as_slice()], &options, None).expect("Rust EMA indicator failed");

            let n = outputs[0].len();

            assert_eq!(
                n,
                ema_outputs[0].len(),
                "output length mismatch for options {:?}",
                options
            );
            assert_eq!(
                n,
                outputs[2].len(),
                "optional EMA length mismatch for options {:?}",
                options
            );

            for i in 0..n {
                // Optional EMA must match standalone Rust EMA exactly
                assert_eq!(
                    outputs[2][i], ema_outputs[0][i],
                    "EMA mismatch at index {} for options {:?}: elderray_ema={}, rust_ema={}",
                    i, options, outputs[2][i], ema_outputs[0][i]
                );

                // Bull = high[period+i] - ema[i]
                let expected_bull = high[period + i] - outputs[2][i];
                assert_eq!(
                    outputs[0][i], expected_bull,
                    "bull mismatch at index {} for options {:?}: bull={}, high-ema={}",
                    i, options, outputs[0][i], expected_bull
                );

                // Bear = low[period+i] - ema[i]
                let expected_bear = low[period + i] - outputs[2][i];
                assert_eq!(
                    outputs[1][i], expected_bear,
                    "bear mismatch at index {} for options {:?}: bear={}, low-ema={}",
                    i, options, outputs[1][i], expected_bear
                );

                // Bull and bear must differ since high != low on every bar
                assert_ne!(
                    outputs[0][i], outputs[1][i],
                    "bull == bear at index {} for options {:?} (high == low?)",
                    i, options
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // Database: same checks against real market data
    // -------------------------------------------------------------------------

    #[test]
    fn test_elderray_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);

            for options in OPTIONS_LIST {
                let period = options[0] as usize;

                // Elder-ray with optional EMA
                let (outputs, _) = rust_elderray(
                    &[high.as_slice(), low.as_slice(), close.as_slice()],
                    &options,
                    Some(&[true]),
                )
                .expect("Rust Elder-ray indicator failed");

                // Standalone Rust EMA
                let (ema_outputs, _) = rust_ema(&[close.as_slice()], &options, None)
                    .expect("Rust EMA indicator failed");

                let n = outputs[0].len();

                assert_eq!(
                    n,
                    ema_outputs[0].len(),
                    "output length mismatch for stock {} options {:?}",
                    stock_symbol,
                    options
                );

                for i in 0..n {
                    assert_eq!(
                        outputs[2][i],
                        ema_outputs[0][i],
                        "EMA mismatch at index {} for stock {} options {:?}: elderray_ema={}, rust_ema={}",
                        i, stock_symbol, options, outputs[2][i], ema_outputs[0][i]
                    );

                    let expected_bull = high[period + i] - outputs[2][i];
                    assert_eq!(
                        outputs[0][i], expected_bull,
                        "bull mismatch at index {} for stock {} options {:?}",
                        i, stock_symbol, options
                    );

                    let expected_bear = low[period + i] - outputs[2][i];
                    assert_eq!(
                        outputs[1][i], expected_bear,
                        "bear mismatch at index {} for stock {} options {:?}",
                        i, stock_symbol, options
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Database: batch_indicator must produce the same bull/bear as the full run
    // -------------------------------------------------------------------------

    #[test]
    fn test_elderray_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

                // Full indicator on all data (reference)
                let (full_outputs, _) = rust_elderray(&inputs_rust, &options, None)
                    .expect("Rust Elder-ray indicator failed");

                // Streaming: seed with the minimum required bars, then chunk
                let mut batch_full_outputs = vec![Vec::new(); 2]; // bull and bear only

                let min_data_val = min_data(&options).max(CHUNK_SIZE);
                let chunk_inputs = [
                    &high[..min_data_val],
                    &low[..min_data_val],
                    &close[..min_data_val],
                ];

                let (first_outputs, mut state) = rust_elderray(&chunk_inputs, &options, None)
                    .expect("Elder-ray indicator failed on first chunk");

                for output_idx in 0..2 {
                    batch_full_outputs[output_idx].extend_from_slice(&first_outputs[output_idx]);
                }

                let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_outputs = state
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("Elder-ray batch_indicator failed");
                    for output_idx in 0..2 {
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
                        .expect("Elder-ray batch_indicator (remainder) failed");
                    for output_idx in 0..2 {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                let band_names = ["bull", "bear"];
                for output_idx in 0..2 {
                    assert_eq!(
                        full_outputs[output_idx].len(),
                        batch_full_outputs[output_idx].len(),
                        "{} band length mismatch for stock {} options {:?}",
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
                            full_val, batch_val,
                            "{} band state mismatch at index {} for stock {} options {:?}: full={}, batch={}",
                            band_names[output_idx], i, stock_symbol, options, full_val, batch_val
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: bull/bear must match regular Elder-ray (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_elderray_simd_by_assets_vs_regular_database() {
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
                .expect("SIMD by-assets Elder-ray failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let (ref_outputs, _) = rust_elderray(
                    &[high.as_slice(), low.as_slice(), close.as_slice()],
                    &options,
                    None,
                )
                .expect("Rust Elder-ray failed");

                let simd_bull = &simd_results[asset_idx][0];
                let simd_bear = &simd_results[asset_idx][1];
                let ref_bull = &ref_outputs[0];
                let ref_bear = &ref_outputs[1];

                assert_eq!(
                    simd_bull.len(),
                    ref_bull.len(),
                    "bull length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    simd_bear.len(),
                    ref_bear.len(),
                    "bear length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&sv, &rv)) in simd_bull.iter().zip(ref_bull.iter()).enumerate() {
                    assert_eq!(
                        sv, rv,
                        "bull mismatch at index {i}: simd={sv}, ref={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
                for (i, (&sv, &rv)) in simd_bear.iter().zip(ref_bear.iter()).enumerate() {
                    assert_eq!(
                        sv, rv,
                        "bear mismatch at index {i}: simd={sv}, ref={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: bull/bear must match regular Elder-ray (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_elderray_simd_by_options_vs_regular_database() {
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
                .expect("SIMD by-options Elder-ray failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (ref_outputs, _) =
                    rust_elderray(&inputs, options, None).expect("Rust Elder-ray failed");

                let simd_bull = &simd_results[opt_idx][0];
                let simd_bear = &simd_results[opt_idx][1];
                let ref_bull = &ref_outputs[0];
                let ref_bear = &ref_outputs[1];

                assert_eq!(
                    simd_bull.len(),
                    ref_bull.len(),
                    "bull length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    simd_bear.len(),
                    ref_bear.len(),
                    "bear length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&sv, &rv)) in simd_bull.iter().zip(ref_bull.iter()).enumerate() {
                    assert_eq!(
                        sv, rv,
                        "bull mismatch at index {i}: simd={sv}, ref={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
                for (i, (&sv, &rv)) in simd_bear.iter().zip(ref_bear.iter()).enumerate() {
                    assert_eq!(
                        sv, rv,
                        "bear mismatch at index {i}: simd={sv}, ref={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: first 1000 bars via SIMD, rest via batch_indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_elderray_simd_by_assets_state_continuity() {
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
                .expect("SIMD by-assets failed on first chunk");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let mut batch_bull = simd_first[asset_idx][0].clone();
                let mut batch_bear = simd_first[asset_idx][1].clone();

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
                    batch_bull.extend_from_slice(&chunk_outputs[0]);
                    batch_bear.extend_from_slice(&chunk_outputs[1]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_bull.extend_from_slice(&chunk_outputs[0]);
                    batch_bear.extend_from_slice(&chunk_outputs[1]);
                }

                let (ref_outputs, _) = rust_elderray(
                    &[high.as_slice(), low.as_slice(), close.as_slice()],
                    &options,
                    None,
                )
                .expect("Rust Elder-ray failed");
                let ref_bull = &ref_outputs[0];
                let ref_bear = &ref_outputs[1];

                assert_eq!(
                    batch_bull.len(),
                    ref_bull.len(),
                    "bull length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    batch_bear.len(),
                    ref_bear.len(),
                    "bear length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&bv, &rv)) in batch_bull.iter().zip(ref_bull.iter()).enumerate() {
                    assert_eq!(
                        bv, rv,
                        "bull mismatch at index {i}: simd+batch={bv}, ref={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
                for (i, (&bv, &rv)) in batch_bear.iter().zip(ref_bear.iter()).enumerate() {
                    assert_eq!(
                        bv, rv,
                        "bear mismatch at index {i}: simd+batch={bv}, ref={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: first 1000 bars via SIMD, rest via batch_indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_elderray_simd_by_options_state_continuity() {
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
                    .expect("SIMD by-options failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch_bull = simd_first[opt_idx][0].clone();
                let mut batch_bear = simd_first[opt_idx][1].clone();

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
                    batch_bull.extend_from_slice(&chunk_outputs[0]);
                    batch_bear.extend_from_slice(&chunk_outputs[1]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_bull.extend_from_slice(&chunk_outputs[0]);
                    batch_bear.extend_from_slice(&chunk_outputs[1]);
                }

                let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (ref_outputs, _) =
                    rust_elderray(&inputs, options, None).expect("Rust Elder-ray failed");
                let ref_bull = &ref_outputs[0];
                let ref_bear = &ref_outputs[1];

                assert_eq!(
                    batch_bull.len(),
                    ref_bull.len(),
                    "bull length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    batch_bear.len(),
                    ref_bear.len(),
                    "bear length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&bv, &rv)) in batch_bull.iter().zip(ref_bull.iter()).enumerate() {
                    assert_eq!(
                        bv, rv,
                        "bull mismatch at index {i}: simd+batch={bv}, ref={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
                for (i, (&bv, &rv)) in batch_bear.iter().zip(ref_bear.iter()).enumerate() {
                    assert_eq!(
                        bv, rv,
                        "bear mismatch at index {i}: simd+batch={bv}, ref={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets optional outputs: EMA must match scalar optional EMA
    // -------------------------------------------------------------------------

    #[test]
    fn test_elderray_simd_by_assets_optional_outputs() {
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
                .expect("SIMD by-assets Elder-ray with optional outputs failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) = rust_elderray(&scalar_inputs, &options, Some(&[true]))
                    .expect("Scalar Elder-ray with optional outputs failed");

                // Primary outputs: bull [0], bear [1]
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

                // EMA optional output (index 2)
                let simd_ema = &simd_results[asset_idx][2];
                let scalar_ema = &scalar_outputs[2];
                assert_eq!(
                    simd_ema.len(),
                    scalar_ema.len(),
                    "EMA length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_ema.iter().zip(scalar_ema).enumerate() {
                    if sv != rv {
                        let start = i.saturating_sub(5);
                        let end = (i + 6).min(simd_ema.len());
                        println!(
                            "EMA mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                            &simd_ema[start..end], &scalar_ema[start..end]
                        );
                        panic!(
                            "EMA mismatch at index {i}: simd={sv}, scalar={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
                println!(
                    "\u{2713} SIMD by-assets optional outputs match scalar for stock={stock_symbol}, options={options:?}"
                );
            }
        }
        println!("\u{2713} All SIMD by-assets Elder-ray optional output tests passed!");
    }

    // -------------------------------------------------------------------------
    // SIMD by-options optional outputs: EMA must match scalar optional EMA
    // -------------------------------------------------------------------------

    #[test]
    fn test_elderray_simd_by_options_optional_outputs() {
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
                .expect("SIMD by-options Elder-ray with optional outputs failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_outputs, _) = rust_elderray(&inputs, options, Some(&[true]))
                    .expect("Scalar Elder-ray with optional outputs failed");

                // Primary outputs: bull [0], bear [1]
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

                // EMA optional output (index 2)
                let simd_ema = &simd_results[opt_idx][2];
                let scalar_ema = &scalar_outputs[2];
                assert_eq!(
                    simd_ema.len(),
                    scalar_ema.len(),
                    "EMA length mismatch: opt_idx={opt_idx}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_ema.iter().zip(scalar_ema).enumerate() {
                    if sv != rv {
                        let start = i.saturating_sub(5);
                        let end = (i + 6).min(simd_ema.len());
                        println!(
                            "EMA mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                            &simd_ema[start..end], &scalar_ema[start..end]
                        );
                        panic!(
                            "EMA mismatch at index {i}: simd={sv}, scalar={rv}, opt_idx={opt_idx}"
                        );
                    }
                }
                println!(
                    "\u{2713} SIMD by-options optional outputs match scalar for opt_idx={opt_idx}, options={options:?}"
                );
            }
        }
        println!("✓ All SIMD by-options Elder-ray optional output tests passed!");
    }

    }
