#[cfg(test)]
mod tests {
    use tulip_rs::indicators::chaikinmf::{
        ChaikinMf, Indicator, IndicatorByOptions, TIndicatorState,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

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
    const VOLUME: [f64; 15] = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [10.0], [14.0], [20.0]];

    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        let mut volume_vec = VOLUME.to_vec();
        for _ in 0..5 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
            volume_vec.extend_from_slice(&VOLUME);
        }
        (high_vec, low_vec, close_vec, volume_vec)
    }

    fn get_hlcv_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
        (high, low, close, volume)
    }

    /// Basic sanity check: no NaN or Inf in output, correct output length.
    #[test]
    fn test_chaikinmf_indicator() {
        let (high, low, close, volume) = expand_inputs();
        let inputs = [
            high.as_slice(),
            low.as_slice(),
            close.as_slice(),
            volume.as_slice(),
        ];

        for options in OPTIONS_LIST {
            let (outputs, _) =
                ChaikinMf::indicator(&inputs, &options, None).expect("Chaikin MF indicator failed");

            assert_eq!(
                outputs.len(),
                1,
                "Expected 1 output vector, got {}",
                outputs.len()
            );

            let cmf = &outputs[0];

            for (i, &val) in cmf.iter().enumerate() {
                assert!(
                    !val.is_nan(),
                    "CMF has NaN at index {}: options={:?}",
                    i,
                    options
                );
                assert!(
                    !val.is_infinite(),
                    "CMF has Inf at index {}: val={}, options={:?}",
                    i,
                    val,
                    options
                );
                // CMF should be bounded roughly in [-1, 1]
                assert!(
                    val >= -1.1 && val <= 1.1,
                    "CMF value out of expected range [-1, 1] at index {}: val={}, options={:?}",
                    i,
                    val,
                    options
                );
            }

            println!(
                "✓ Chaikin MF indicator ok: {} output values, options={:?}",
                cmf.len(),
                options
            );
        }
    }

    /// Run on real database data — checks all-finite values for every stock and period.
    #[test]
    fn test_chaikinmf_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            for options in OPTIONS_LIST {
                if high.len() < ChaikinMf::min_data(&options) {
                    continue;
                }

                let (outputs, _) = ChaikinMf::indicator(&inputs, &options, None)
                    .expect("Chaikin MF indicator failed");

                let cmf = &outputs[0];

                for (i, &val) in cmf.iter().enumerate() {
                    assert!(
                        val.is_finite(),
                        "CMF has NaN/Inf at index {}: val={}, options={:?}, stock={}",
                        i,
                        val,
                        options,
                        stock_symbol
                    );
                }
            }

            println!(
                "✓ Chaikin MF database test passed for stock {}",
                stock_symbol
            );
        }
    }

    /// State continuity test: chunked `batch_indicator` output must exactly match
    /// the full one-shot `indicator` output.
    #[test]
    fn test_chaikinmf_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let inputs_rust = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            for options in OPTIONS_LIST {
                if high.len() < ChaikinMf::min_data(&options) {
                    continue;
                }

                // Full one-shot output.
                let (full_outputs, _) = ChaikinMf::indicator(&inputs_rust, &options, None)
                    .expect("Failed to run Chaikin MF indicator on full data");

                let min_data_val = ChaikinMf::min_data(&options).max(CHUNK_SIZE);
                let mut batch_full_output: Vec<f64> = Vec::new();

                if high.len() <= min_data_val {
                    // Data fits in a single chunk — just use the full output directly.
                    batch_full_output.extend_from_slice(&full_outputs[0]);
                } else {
                    // First chunk: warm up the state.
                    let chunk_inputs = [
                        &high[..min_data_val],
                        &low[..min_data_val],
                        &close[..min_data_val],
                        &volume[..min_data_val],
                    ];
                    let (first_outputs, mut state) =
                        ChaikinMf::indicator(&chunk_inputs, &options, None)
                            .expect("Failed to run Chaikin MF on first chunk");
                    batch_full_output.extend_from_slice(&first_outputs[0]);

                    // Remaining data in CHUNK_SIZE chunks.
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut volume_chunks = volume[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (((hc, lc), cc), vc) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                        .zip(volume_chunks.by_ref())
                    {
                        let chunk_inputs = [hc, lc, cc, vc];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("Chaikin MF batch_indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }

                    // Final remainder (if any).
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();
                    let volume_rem = volume_chunks.remainder();

                    if !high_rem.is_empty() {
                        let chunk_inputs = [high_rem, low_rem, close_rem, volume_rem];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("Chaikin MF batch_indicator failed on remainder");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }
                }

                // Length check.
                assert_eq!(
                    full_outputs[0].len(),
                    batch_full_output.len(),
                    "Output length mismatch for stock {} with options {:?}: full={}, batch={}",
                    stock_symbol,
                    options,
                    full_outputs[0].len(),
                    batch_full_output.len()
                );

                // Value check — must be bit-exact (same code path, no approximations).
                for (i, (&full_val, &batch_val)) in full_outputs[0]
                    .iter()
                    .zip(batch_full_output.iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "CMF mismatch at index {}: full={}, batch={}, stock={}, options={:?}",
                        i, full_val, batch_val, stock_symbol, options
                    );
                }
            }

            println!(
                "✓ Chaikin MF state continuity test passed for stock {}",
                stock_symbol
            );
        }

        println!("✓ All Chaikin MF state continuity tests passed!");
    }

    // =========================================================================
    // SIMD by-assets: outputs match scalar (database)
    // =========================================================================

    #[test]
    fn test_chaikinmf_simd_by_assets_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close, volume) = get_hlcv_arrays(eod);
                (symbol.clone(), high, low, close, volume)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 4] = [
                &stock_data[0].1,
                &stock_data[0].2,
                &stock_data[0].3,
                &stock_data[0].4,
            ];
            let asset1: [&[f64]; 4] = [
                &stock_data[1].1,
                &stock_data[1].2,
                &stock_data[1].3,
                &stock_data[1].4,
            ];
            let asset2: [&[f64]; 4] = [
                &stock_data[2].1,
                &stock_data[2].2,
                &stock_data[2].3,
                &stock_data[2].4,
            ];
            let asset3: [&[f64]; 4] = [
                &stock_data[3].1,
                &stock_data[3].2,
                &stock_data[3].3,
                &stock_data[3].4,
            ];
            let inputs_4: [&[&[f64]; 4]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_results, _) = ChaikinMf::indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by-assets ChaikinMF failed");

            for (asset_idx, (stock_symbol, high, low, close, volume)) in
                stock_data.iter().enumerate()
            {
                let scalar_inputs = [
                    high.as_slice(),
                    low.as_slice(),
                    close.as_slice(),
                    volume.as_slice(),
                ];
                let (scalar_outputs, _) = ChaikinMf::indicator(&scalar_inputs, &options, None)
                    .expect("Rust ChaikinMF failed");

                let simd_cmf = &simd_results[asset_idx][0];
                let scalar_cmf = &scalar_outputs[0];

                assert_eq!(
                    simd_cmf.len(),
                    scalar_cmf.len(),
                    "CMF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_cmf.iter().zip(scalar_cmf.iter()).enumerate() {
                    assert_eq!(sv, rv,
                        "CMF mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }

            println!("✓ SIMD by-assets vs scalar ChaikinMF ok for options={options:?}");
        }
    }

    // =========================================================================
    // SIMD by-options: outputs match scalar (database)
    // =========================================================================

    #[test]
    fn test_chaikinmf_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            let (simd_results, _) = ChaikinMf::indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by-options ChaikinMF failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                if high.len() < ChaikinMf::min_data(options) {
                    continue;
                }
                let (scalar_outputs, _) =
                    ChaikinMf::indicator(&inputs, options, None).expect("Rust ChaikinMF failed");

                let simd_cmf = &simd_results[opt_idx][0];
                let scalar_cmf = &scalar_outputs[0];

                assert_eq!(
                    simd_cmf.len(),
                    scalar_cmf.len(),
                    "CMF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_cmf.iter().zip(scalar_cmf.iter()).enumerate() {
                    assert_eq!(sv, rv,
                        "CMF mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }

            println!("✓ SIMD by-options vs scalar ChaikinMF ok for stock={stock_symbol}");
        }
    }

    // =========================================================================
    // SIMD by-assets: first FIRST_CHUNK bars via SIMD, rest via batch_indicator
    // =========================================================================

    #[test]
    fn test_chaikinmf_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close, volume) = get_hlcv_arrays(eod);
                (symbol.clone(), high, low, close, volume)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 4] = [
                &stock_data[0].1[..FIRST_CHUNK],
                &stock_data[0].2[..FIRST_CHUNK],
                &stock_data[0].3[..FIRST_CHUNK],
                &stock_data[0].4[..FIRST_CHUNK],
            ];
            let asset1: [&[f64]; 4] = [
                &stock_data[1].1[..FIRST_CHUNK],
                &stock_data[1].2[..FIRST_CHUNK],
                &stock_data[1].3[..FIRST_CHUNK],
                &stock_data[1].4[..FIRST_CHUNK],
            ];
            let asset2: [&[f64]; 4] = [
                &stock_data[2].1[..FIRST_CHUNK],
                &stock_data[2].2[..FIRST_CHUNK],
                &stock_data[2].3[..FIRST_CHUNK],
                &stock_data[2].4[..FIRST_CHUNK],
            ];
            let asset3: [&[f64]; 4] = [
                &stock_data[3].1[..FIRST_CHUNK],
                &stock_data[3].2[..FIRST_CHUNK],
                &stock_data[3].3[..FIRST_CHUNK],
                &stock_data[3].4[..FIRST_CHUNK],
            ];
            let inputs_4: [&[&[f64]; 4]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_first, mut states) =
                ChaikinMf::indicator_by_assets::<4>(&inputs_4, &options, None)
                    .expect("SIMD by-assets failed on first chunk");

            for (asset_idx, (stock_symbol, high, low, close, volume)) in
                stock_data.iter().enumerate()
            {
                let mut batch_cmf = simd_first[asset_idx][0].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for (((hc, lc), cc), vc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                    .zip(volume_chunks.by_ref())
                {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[hc, lc, cc, vc], None)
                        .expect("batch_indicator failed");
                    batch_cmf.extend_from_slice(&chunk_outputs[0]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                let volume_rem = volume_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem, volume_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_cmf.extend_from_slice(&chunk_outputs[0]);
                }

                let scalar_inputs = [
                    high.as_slice(),
                    low.as_slice(),
                    close.as_slice(),
                    volume.as_slice(),
                ];
                let (scalar_outputs, _) = ChaikinMf::indicator(&scalar_inputs, &options, None)
                    .expect("Rust ChaikinMF failed");

                assert_eq!(
                    batch_cmf.len(),
                    scalar_outputs[0].len(),
                    "CMF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_cmf.iter().zip(scalar_outputs[0].iter()).enumerate() {
                    assert_eq!(bv, rv,
                        "CMF mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }

            println!("✓ SIMD by-assets state continuity ok for options={options:?}");
        }
    }

    // =========================================================================
    // SIMD by-options: first FIRST_CHUNK bars via SIMD, rest via batch_indicator
    // =========================================================================

    #[test]
    fn test_chaikinmf_simd_by_options_state_continuity() {
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
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);

            let first_inputs = [
                &high[..FIRST_CHUNK],
                &low[..FIRST_CHUNK],
                &close[..FIRST_CHUNK],
                &volume[..FIRST_CHUNK],
            ];
            let (simd_first, mut states) =
                ChaikinMf::indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD by-options failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch_cmf = simd_first[opt_idx][0].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for (((hc, lc), cc), vc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                    .zip(volume_chunks.by_ref())
                {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[hc, lc, cc, vc], None)
                        .expect("batch_indicator failed");
                    batch_cmf.extend_from_slice(&chunk_outputs[0]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                let volume_rem = volume_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem, volume_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_cmf.extend_from_slice(&chunk_outputs[0]);
                }

                let scalar_inputs = [
                    high.as_slice(),
                    low.as_slice(),
                    close.as_slice(),
                    volume.as_slice(),
                ];
                let (scalar_outputs, _) = ChaikinMf::indicator(&scalar_inputs, options, None)
                    .expect("Rust ChaikinMF failed");

                assert_eq!(
                    batch_cmf.len(),
                    scalar_outputs[0].len(),
                    "CMF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_cmf.iter().zip(scalar_outputs[0].iter()).enumerate() {
                    assert_eq!(bv, rv,
                        "CMF mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }

            println!("✓ SIMD by-options state continuity ok for stock={stock_symbol}");
        }
    }
}
