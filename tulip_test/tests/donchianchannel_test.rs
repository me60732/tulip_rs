#[cfg(test)]
mod tests {
    use tulip_rs::indicators::donchianchannel::{DonchianChannel, Indicator, TIndicatorState, indicator_by_assets, indicator_by_options};
    use tulip_rs::indicators::max::Max;
    use tulip_rs::indicators::min::Min;
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

    // Options for Donchian Channel: [period]
    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [50.0]];

    fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
        }
        (high_vec, low_vec) // 60 bars — enough for period up to 50
    }

    fn get_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        (high, low)
    }

    // -------------------------------------------------------------------------
    // upper == max(high, period),  lower == min(low, period)
    //
    // Donchian Channel calls the same internal calc_max / calc_min functions as
    // the standalone max/min indicators, so the outputs are bit-for-bit identical
    // and we can use assert_eq! throughout.
    // -------------------------------------------------------------------------

    /// Verify that the upper band equals `Max::indicator(high, period)`, the lower band
    /// equals `Min::indicator(low, period)`, and the middle band equals `(upper+lower)/2`.
    #[test]
    fn test_donchianchannel_indicator() {
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            // Reference: standalone Rust max on high
            let (max_outputs, _) =
                Max::indicator(&[high.as_slice()], &options, None).expect("Rust MAX indicator failed");

            // Reference: standalone Rust min on low
            let (min_outputs, _) =
                Min::indicator(&[low.as_slice()], &options, None).expect("Rust MIN indicator failed");

            // Donchian Channel
            let (outputs, _) =
                DonchianChannel::indicator(&[high.as_slice(), low.as_slice()], &options, None)
                    .expect("Rust Donchian Channel indicator failed");

            // Output lengths must match (all three use min_data = period + 1)
            assert_eq!(
                outputs[0].len(),
                min_outputs[0].len(),
                "lower/min length mismatch for options {:?}",
                options
            );
            assert_eq!(
                outputs[2].len(),
                max_outputs[0].len(),
                "upper/max length mismatch for options {:?}",
                options
            );

            let n = outputs[0].len();

            // UPPER must equal max(high)
            for i in 0..n {
                assert_eq!(
                    outputs[2][i], max_outputs[0][i],
                    "UPPER mismatch at index {} for options {:?}: donchian={}, max={}",
                    i, options, outputs[2][i], max_outputs[0][i]
                );
            }

            // LOWER must equal min(low)
            for i in 0..n {
                assert_eq!(
                    outputs[0][i], min_outputs[0][i],
                    "LOWER mismatch at index {} for options {:?}: donchian={}, min={}",
                    i, options, outputs[0][i], min_outputs[0][i]
                );
            }

            // MIDDLE must equal (upper + lower) / 2
            for i in 0..n {
                let expected = (outputs[2][i] + outputs[0][i]) / 2.0;
                assert_eq!(
                    outputs[1][i], expected,
                    "MIDDLE mismatch at index {} for options {:?}: middle={}, (upper+lower)/2={}",
                    i, options, outputs[1][i], expected
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // Database: upper vs Max::indicator, lower vs Min::indicator, middle identity
    // -------------------------------------------------------------------------

    #[test]
    fn test_donchianchannel_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_arrays(stock_data);

            for options in OPTIONS_LIST {
                // Reference: standalone Rust max on high
                let (max_outputs, _) = Max::indicator(&[high.as_slice()], &options, None)
                    .expect("Rust MAX indicator failed");

                // Reference: standalone Rust min on low
                let (min_outputs, _) =
                    Min::indicator(&[low.as_slice()], &options, None).expect("Rust MIN indicator failed");

                // Donchian Channel
                let (outputs, _) =
                    DonchianChannel::indicator(&[high.as_slice(), low.as_slice()], &options, None)
                        .expect("Rust Donchian Channel indicator failed");

                let n = outputs[0].len();

                assert_eq!(
                    n,
                    min_outputs[0].len(),
                    "lower/min length mismatch for stock {} options {:?}",
                    stock_symbol,
                    options
                );
                assert_eq!(
                    n,
                    max_outputs[0].len(),
                    "upper/max length mismatch for stock {} options {:?}",
                    stock_symbol,
                    options
                );

                // UPPER must equal max(high)
                for i in 0..n {
                    assert_eq!(
                        outputs[2][i], max_outputs[0][i],
                        "UPPER mismatch at index {} for stock {} options {:?}: donchian={}, max={}",
                        i, stock_symbol, options, outputs[2][i], max_outputs[0][i]
                    );
                }

                // LOWER must equal min(low)
                for i in 0..n {
                    assert_eq!(
                        outputs[0][i], min_outputs[0][i],
                        "LOWER mismatch at index {} for stock {} options {:?}: donchian={}, min={}",
                        i, stock_symbol, options, outputs[0][i], min_outputs[0][i]
                    );
                }

                // MIDDLE must equal (upper + lower) / 2
                for i in 0..n {
                    let expected = (outputs[2][i] + outputs[0][i]) / 2.0;
                    assert_eq!(
                        outputs[1][i], expected,
                        "MIDDLE mismatch at index {} for stock {} options {:?}: middle={}, (upper+lower)/2={}",
                        i, stock_symbol, options, outputs[1][i], expected
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Database: batch_indicator vs full indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_donchianchannel_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice()];

                // Full indicator on all data (reference)
                let (full_outputs, _) = DonchianChannel::indicator(&inputs_rust, &options, None)
                    .expect("Rust Donchian Channel indicator failed");

                // Streaming: seed + chunked batch
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = DonchianChannel::min_data(&options).max(CHUNK_SIZE);
                let chunk_inputs = [&high[..min_data_val], &low[..min_data_val]];

                let (first_outputs, mut state) =
                    DonchianChannel::indicator(&chunk_inputs, &options, None)
                        .expect("Donchian Channel indicator failed");
                for output_idx in 0..first_outputs.len() {
                    batch_full_outputs[output_idx].extend_from_slice(&first_outputs[output_idx]);
                }

                let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                for (hc, lc) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                    let chunk_outputs = state
                        .batch_indicator(&[hc, lc], None)
                        .expect("Donchian Channel batch_indicator failed");
                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = state
                        .batch_indicator(&[high_rem, low_rem], None)
                        .expect("Donchian Channel batch_indicator (remainder) failed");
                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                // Compare every output band
                let band_names = ["lower", "middle", "upper"];
                for output_idx in 0..full_outputs.len() {
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
    // SIMD by-assets: lower == Min::indicator, upper == Max::indicator (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_donchianchannel_simd_by_assets_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low) = get_arrays(eod);
                (symbol.clone(), high, low)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 2] = [&stock_data[0].1, &stock_data[0].2];
            let asset1: [&[f64]; 2] = [&stock_data[1].1, &stock_data[1].2];
            let asset2: [&[f64]; 2] = [&stock_data[2].1, &stock_data[2].2];
            let asset3: [&[f64]; 2] = [&stock_data[3].1, &stock_data[3].2];
            let inputs_4: [&[&[f64]; 2]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by assets DC indicator failed");

            for (asset_idx, (stock_symbol, high, low)) in stock_data.iter().enumerate() {
                let (min_outputs, _) =
                    Min::indicator(&[low.as_slice()], &options, None).expect("Rust MIN failed");
                let (max_outputs, _) =
                    Max::indicator(&[high.as_slice()], &options, None).expect("Rust MAX failed");

                let simd_lower = &simd_results[asset_idx][0];
                let simd_upper = &simd_results[asset_idx][2];
                let min_line = &min_outputs[0];
                let max_line = &max_outputs[0];

                assert_eq!(
                    simd_lower.len(),
                    min_line.len(),
                    "lower/min length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    simd_upper.len(),
                    max_line.len(),
                    "upper/max length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&sv, &rv)) in simd_lower.iter().zip(min_line.iter()).enumerate() {
                    assert_eq!(
                    sv, rv,
                    "lower mismatch at index {i}: simd={sv}, min={rv}, stock={stock_symbol}, options={options:?}"
                                                );
                }
                for (i, (&sv, &rv)) in simd_upper.iter().zip(max_line.iter()).enumerate() {
                    assert_eq!(
                    sv, rv,
                    "upper mismatch at index {i}: simd={sv}, max={rv}, stock={stock_symbol}, options={options:?}"
                                                );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: lower == Min::indicator, upper == Max::indicator (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_donchianchannel_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice()];

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by options DC indicator failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (min_outputs, _) =
                    Min::indicator(&[low.as_slice()], options, None).expect("Rust MIN failed");
                let (max_outputs, _) =
                    Max::indicator(&[high.as_slice()], options, None).expect("Rust MAX failed");

                let simd_lower = &simd_results[opt_idx][0];
                let simd_upper = &simd_results[opt_idx][2];
                let min_line = &min_outputs[0];
                let max_line = &max_outputs[0];

                assert_eq!(
                    simd_lower.len(),
                    min_line.len(),
                    "lower/min length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    simd_upper.len(),
                    max_line.len(),
                    "upper/max length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&sv, &rv)) in simd_lower.iter().zip(min_line.iter()).enumerate() {
                    assert_eq!(
                    sv, rv,
                    "lower mismatch at index {i}: simd={sv}, min={rv}, stock={stock_symbol}, options={options:?}"
                                                );
                }
                for (i, (&sv, &rv)) in simd_upper.iter().zip(max_line.iter()).enumerate() {
                    assert_eq!(
                    sv, rv,
                    "upper mismatch at index {i}: simd={sv}, max={rv}, stock={stock_symbol}, options={options:?}"
                                                );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: first 1000 bars via SIMD, rest via batch_indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_donchianchannel_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low) = get_arrays(eod);
                (symbol.clone(), high, low)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 2] = [
                &stock_data[0].1[..FIRST_CHUNK],
                &stock_data[0].2[..FIRST_CHUNK],
            ];
            let asset1: [&[f64]; 2] = [
                &stock_data[1].1[..FIRST_CHUNK],
                &stock_data[1].2[..FIRST_CHUNK],
            ];
            let asset2: [&[f64]; 2] = [
                &stock_data[2].1[..FIRST_CHUNK],
                &stock_data[2].2[..FIRST_CHUNK],
            ];
            let asset3: [&[f64]; 2] = [
                &stock_data[3].1[..FIRST_CHUNK],
                &stock_data[3].2[..FIRST_CHUNK],
            ];
            let inputs_4: [&[&[f64]; 2]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by assets failed on first chunk");

            for (asset_idx, (stock_symbol, high, low)) in stock_data.iter().enumerate() {
                let mut batch_lower = simd_first[asset_idx][0].clone();
                let mut batch_upper = simd_first[asset_idx][2].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for (hc, lc) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[hc, lc], None)
                        .expect("batch_indicator failed");
                    batch_lower.extend_from_slice(&chunk_outputs[0]);
                    batch_upper.extend_from_slice(&chunk_outputs[2]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[high_rem, low_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_lower.extend_from_slice(&chunk_outputs[0]);
                    batch_upper.extend_from_slice(&chunk_outputs[2]);
                }

                let (min_outputs, _) =
                    Min::indicator(&[low.as_slice()], &options, None).expect("Rust MIN failed");
                let (max_outputs, _) =
                    Max::indicator(&[high.as_slice()], &options, None).expect("Rust MAX failed");
                let min_line = &min_outputs[0];
                let max_line = &max_outputs[0];

                assert_eq!(
                    batch_lower.len(),
                    min_line.len(),
                    "lower/min length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    batch_upper.len(),
                    max_line.len(),
                    "upper/max length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&bv, &rv)) in batch_lower.iter().zip(min_line.iter()).enumerate() {
                    assert_eq!(
                    bv, rv,
                    "lower mismatch at index {i}: simd+batch={bv}, min={rv}, stock={stock_symbol}, options={options:?}"
                                                );
                }
                for (i, (&bv, &rv)) in batch_upper.iter().zip(max_line.iter()).enumerate() {
                    assert_eq!(
                    bv, rv,
                    "upper mismatch at index {i}: simd+batch={bv}, max={rv}, stock={stock_symbol}, options={options:?}"
                                                );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: first 1000 bars via SIMD, rest via batch_indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_donchianchannel_simd_by_options_state_continuity() {
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
            let (high, low) = get_arrays(stock_data);

            let first_inputs = [&high[..FIRST_CHUNK], &low[..FIRST_CHUNK]];
            let (simd_first, mut states) =
                indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD by options failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch_lower = simd_first[opt_idx][0].clone();
                let mut batch_upper = simd_first[opt_idx][2].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for (hc, lc) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[hc, lc], None)
                        .expect("batch_indicator failed");
                    batch_lower.extend_from_slice(&chunk_outputs[0]);
                    batch_upper.extend_from_slice(&chunk_outputs[2]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[high_rem, low_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_lower.extend_from_slice(&chunk_outputs[0]);
                    batch_upper.extend_from_slice(&chunk_outputs[2]);
                }

                let (min_outputs, _) =
                    Min::indicator(&[low.as_slice()], options, None).expect("Rust MIN failed");
                let (max_outputs, _) =
                    Max::indicator(&[high.as_slice()], options, None).expect("Rust MAX failed");
                let min_line = &min_outputs[0];
                let max_line = &max_outputs[0];

                assert_eq!(
                    batch_lower.len(),
                    min_line.len(),
                    "lower/min length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    batch_upper.len(),
                    max_line.len(),
                    "upper/max length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&bv, &rv)) in batch_lower.iter().zip(min_line.iter()).enumerate() {
                    assert_eq!(
                    bv, rv,
                    "lower mismatch at index {i}: simd+batch={bv}, min={rv}, stock={stock_symbol}, options={options:?}"
                                                );
                }
                for (i, (&bv, &rv)) in batch_upper.iter().zip(max_line.iter()).enumerate() {
                    assert_eq!(
                    bv, rv,
                    "upper mismatch at index {i}: simd+batch={bv}, max={rv}, stock={stock_symbol}, options={options:?}"
                                                );
                }
            }
        }
    }

    }
