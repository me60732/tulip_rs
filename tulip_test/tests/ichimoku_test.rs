#[cfg(test)]
mod tests {
    use tulip_rs::indicators::donchianchannel::indicator as rust_donchianchannel;
    use tulip_rs::indicators::ichimoku::{indicator as rust_ichimoku, min_data, TIndicatorState};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const FIRST_CHUNK: usize = 1000;

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

    // [short_period, long_period] — standard Ichimoku settings and an alternative
    const OPTIONS_LIST: [[f64; 2]; 2] = [[9.0, 26.0], [7.0, 22.0]];

    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        // Repeat to reach ≥195 bars — enough for [9, 26] (needs ~78) and [7, 22] (needs ~66)
        for _ in 0..20 {
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
    // Core identity: Ichimoku lines are Donchian Channel midlines
    //
    // conversion     == donchian(short_period).middle  [outputs[1]]
    // base           == donchian(long_period).middle   [outputs[1]]
    // leading_span_b == donchian(long_period * 2).middle [outputs[1]]
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_vs_donchian() {
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            let short_period = options[0] as usize;
            let long_period = options[1] as usize;
            let ultra_period = long_period * 2;

            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (outputs, _) =
                rust_ichimoku(&inputs, &options, None).expect("Ichimoku indicator failed");

            // Reference Donchian channels — middle band is outputs[1]
            let (don_short, _) =
                rust_donchianchannel(&[high.as_slice(), low.as_slice()], &[options[0]], None)
                    .expect("Donchian short failed");
            let (don_long, _) =
                rust_donchianchannel(&[high.as_slice(), low.as_slice()], &[options[1]], None)
                    .expect("Donchian long failed");
            let (don_ultra, _) = rust_donchianchannel(
                &[high.as_slice(), low.as_slice()],
                &[ultra_period as f64],
                None,
            )
            .expect("Donchian ultra failed");

            let conversion = &outputs[0];
            let base = &outputs[1];
            let span_b = &outputs[3];

            // conversion == donchian(short_period) middle
            assert_eq!(
                conversion.len(),
                don_short[1].len(),
                "conversion length mismatch for options {:?}: ichimoku={}, donchian_short={}",
                options,
                conversion.len(),
                don_short[1].len()
            );
            for i in 0..conversion.len() {
                if conversion[i] != don_short[1][i] {
                    let start = if i >= 10 { i - 10 } else { 0 };
                    let end = if i < conversion.len() - 10 {
                        i + 10
                    } else {
                        conversion.len()
                    };
                    println!(
                        "Test failed at index {}:\nIchimoku conversion = {:?},\nDonchian short middle = {:?}, Options = {:?}",
                        i, &conversion[start..end], &don_short[1][start..end], options
                    );
                    panic!(
                        "conversion mismatch at index {} for options {:?} (short_period={}): ichimoku={}, donchian={}",
                        i, options, short_period, conversion[i], don_short[1][i]
                    );
                }
            }

            // base == donchian(long_period) middle
            assert_eq!(
                base.len(),
                don_long[1].len(),
                "base length mismatch for options {:?}: ichimoku={}, donchian_long={}",
                options,
                base.len(),
                don_long[1].len()
            );
            for i in 0..base.len() {
                if base[i] != don_long[1][i] {
                    let start = if i >= 10 { i - 10 } else { 0 };
                    let end = if i < base.len() - 10 {
                        i + 10
                    } else {
                        base.len()
                    };
                    println!(
                        "Test failed at index {}:\nIchimoku base = {:?},\nDonchian long middle = {:?}, Options = {:?}",
                        i, &base[start..end], &don_long[1][start..end], options
                    );
                    panic!(
                        "base mismatch at index {} for options {:?} (long_period={}): ichimoku={}, donchian={}",
                        i, options, long_period, base[i], don_long[1][i]
                    );
                }
            }

            // leading_span_b == donchian(long_period * 2) middle
            let n = span_b.len().min(don_ultra[1].len());
            assert!(
                span_b.len() >= don_ultra[1].len(),
                "leading_span_b ({}) should be >= donchian_ultra middle ({}) for options {:?}",
                span_b.len(),
                don_ultra[1].len(),
                options
            );
            for i in 0..n {
                if span_b[i] != don_ultra[1][i] {
                    let start = if i >= 10 { i - 10 } else { 0 };
                    let end = if i < n - 10 { i + 10 } else { n };
                    println!(
                        "Test failed at index {}:\nIchimoku span_b = {:?},\nDonchian ultra middle = {:?}, Options = {:?}",
                        i, &span_b[start..end], &don_ultra[1][start..end], options
                    );
                    panic!(
                        "leading_span_b mismatch at index {} for options {:?} (ultra_period={}): ichimoku={}, donchian={}",
                        i, options, ultra_period, span_b[i], don_ultra[1][i]
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Identity: leading_span_a == (conversion + base) / 2
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_span_a_identity() {
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (outputs, _) =
                rust_ichimoku(&inputs, &options, None).expect("Ichimoku indicator failed");

            let conversion = &outputs[0];
            let base = &outputs[1];
            let span_a = &outputs[2];

            let n = span_a.len();
            let conv_offset = conversion.len() - n;
            let base_offset = base.len() - n;

            for i in 0..n {
                let expected = (conversion[conv_offset + i] + base[base_offset + i]) / 2.0;
                if span_a[i] != expected {
                    let start = if i >= 10 { i - 10 } else { 0 };
                    let end = if i < n - 10 { i + 10 } else { n };
                    println!(
                        "Test failed at index {}:\nIchimoku span_a = {:?},\nConversion (aligned) = {:?},\nBase (aligned) = {:?}, Options = {:?}",
                        i, &span_a[start..end], &conversion[conv_offset + start..conv_offset + end], &base[base_offset + start..base_offset + end], options
                    );
                    panic!(
                        "leading_span_a mismatch at index {} for options {:?}: ichimoku={}, (conv+base)/2={}",
                        i, options, span_a[i], expected
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Optional output: lagging_span is a verbatim copy of the close input
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_optional_lagging_span() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let options = OPTIONS_LIST[0];

        let (outputs, _) = rust_ichimoku(&inputs, &options, Some(&[true]))
            .expect("Ichimoku with lagging_span failed");

        let lagging_span = &outputs[4];
        assert_eq!(
            lagging_span.len(),
            close.len(),
            "lagging_span length ({}) should equal close length ({})",
            lagging_span.len(),
            close.len()
        );
        for i in 0..close.len() {
            if lagging_span[i] != close[i] {
                let start = if i >= 10 { i - 10 } else { 0 };
                let end = if i < close.len() - 10 {
                    i + 10
                } else {
                    close.len()
                };
                println!(
                    "Test failed at index {}:\nLagging span = {:?},\nClose = {:?}",
                    i,
                    &lagging_span[start..end],
                    &close[start..end]
                );
                panic!(
                    "lagging_span[{}] = {} but close[{}] = {}",
                    i, lagging_span[i], i, close[i]
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // Optional output disabled: lagging_span vec should be empty
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_optional_lagging_span_disabled() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let options = OPTIONS_LIST[0];

        let (outputs, _) =
            rust_ichimoku(&inputs, &options, None).expect("Ichimoku without lagging_span failed");
        assert!(
            outputs[4].is_empty(),
            "lagging_span should be empty when optional output is not requested"
        );
    }

    // -------------------------------------------------------------------------
    // Database: batch_indicator continuation matches the full indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

                // Full run (reference)
                let (full_outputs, _) = rust_ichimoku(&inputs_rust, &options, None)
                    .expect("Ichimoku full indicator failed");

                // Streaming: seed with min_data bars, then chunk-by-chunk
                let mut batch_outputs: Vec<Vec<f64>> = vec![Vec::new(); full_outputs.len()];

                let seed_len = min_data(&options).max(CHUNK_SIZE);
                let seed_inputs = [&high[..seed_len], &low[..seed_len], &close[..seed_len]];

                let (seed_out, mut state) =
                    rust_ichimoku(&seed_inputs, &options, None).expect("Ichimoku seed failed");
                for j in 0..seed_out.len() {
                    batch_outputs[j].extend_from_slice(&seed_out[j]);
                }

                let mut high_chunks = high[seed_len..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[seed_len..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[seed_len..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_out = state
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("Ichimoku batch_indicator failed");
                    for j in 0..chunk_out.len() {
                        batch_outputs[j].extend_from_slice(&chunk_out[j]);
                    }
                }

                let hr = high_chunks.remainder();
                let lr = low_chunks.remainder();
                let cr = close_chunks.remainder();
                if !hr.is_empty() {
                    let chunk_out = state
                        .batch_indicator(&[hr, lr, cr], None)
                        .expect("Ichimoku batch_indicator (remainder) failed");
                    for j in 0..chunk_out.len() {
                        batch_outputs[j].extend_from_slice(&chunk_out[j]);
                    }
                }

                // Compare the four main outputs (skip index 4 — lagging_span is optional)
                let names = ["conversion", "base", "leading_span_a", "leading_span_b"];
                for j in 0..4 {
                    assert_eq!(
                        full_outputs[j].len(),
                        batch_outputs[j].len(),
                        "{} length mismatch for stock {} options {:?}",
                        names[j],
                        stock_symbol,
                        options
                    );
                    let out_len = full_outputs[j].len();
                    for (i, (&full_val, &batch_val)) in full_outputs[j]
                        .iter()
                        .zip(batch_outputs[j].iter())
                        .enumerate()
                    {
                        if full_val != batch_val {
                            let start = if i >= 10 { i - 10 } else { 0 };
                            let end = if i < out_len - 10 { i + 10 } else { out_len };
                            println!(
                                "Test failed at index {}:\nFull {} = {:?},\nBatch {} = {:?}, Stock = {}, Options = {:?}",
                                i, names[j], &full_outputs[j][start..end], names[j], &batch_outputs[j][start..end], stock_symbol, options
                            );
                            panic!(
                                "{} state mismatch at index {} for stock {} options {:?}: full={}, batch={}",
                                names[j], i, stock_symbol, options, full_val, batch_val
                            );
                        }
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by_assets: N=4 assets processed simultaneously
    // Each asset result must match the scalar run within floating-point identity
    // (Ichimoku uses exact rational arithmetic, so diff must be exactly 0).
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_simd_by_assets() {
        use tulip_rs::indicators::ichimoku::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| {
                let (high, low, close) = get_arrays(eod);
                (sym.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let inputs_4: [&[&[f64]; 3]; 4] = [
                &[&stock_data[0].1, &stock_data[0].2, &stock_data[0].3],
                &[&stock_data[1].1, &stock_data[1].2, &stock_data[1].3],
                &[&stock_data[2].1, &stock_data[2].2, &stock_data[2].3],
                &[&stock_data[3].1, &stock_data[3].2, &stock_data[3].3],
            ];

            let (simd_results, _) =
                indicator_by_assets::<4>(&inputs_4, &options, None).expect("SIMD by_assets failed");

            let labels = ["conversion", "base", "span_a", "span_b"];
            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_out, _) =
                    rust_ichimoku(&inputs, &options, None).expect("scalar failed");

                for k in 0..4 {
                    let simd_line = &simd_results[asset_idx][k];
                    let scalar_line = &scalar_out[k];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{} length mismatch: stock={stock_symbol}, options={:?}",
                        labels[k],
                        options
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        assert!(
                            sv.is_finite(),
                            "SIMD {} NaN/Inf at {i}: stock={stock_symbol}",
                            labels[k]
                        );
                        let diff = (sv - rv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                             stock={stock_symbol}, options={:?}",
                            labels[k],
                            options
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by_options: N=4 option sets on one asset
    // Each lane result must match the scalar run for its option set within 1e-10.
    // Uses 4 option combinations covering a range of short/long periods.
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_simd_by_options() {
        use tulip_rs::indicators::ichimoku::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Four option sets: standard Ichimoku, an alternative, a shorter, and a longer.
        let options_4: [&[f64; 2]; 4] = [&[9.0, 26.0], &[7.0, 22.0], &[5.0, 15.0], &[14.0, 45.0]];

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by_options failed");

            let labels = ["conversion", "base", "span_a", "span_b"];
            for (lane, &options) in options_4.iter().enumerate() {
                let (scalar_out, _) = rust_ichimoku(&inputs, options, None).expect("scalar failed");

                for k in 0..4 {
                    let simd_line = &simd_results[lane][k];
                    let scalar_line = &scalar_out[k];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{} length mismatch: stock={stock_symbol}, options={:?}",
                        labels[k],
                        options
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        assert!(
                            sv.is_finite(),
                            "SIMD by_options {} NaN/Inf at {i}: stock={stock_symbol}",
                            labels[k]
                        );
                        let diff = (sv - rv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                             stock={stock_symbol}, options={:?}",
                            labels[k],
                            options
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by_assets state continuity
    // Seed the first FIRST_CHUNK bars with indicator_by_assets, then drive the
    // remaining bars via each asset's returned IndicatorState.batch_indicator.
    // The stitched output must match a full scalar run bar-for-bar.
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_simd_by_assets_state_continuity() {
        use tulip_rs::indicators::ichimoku::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| {
                let (high, low, close) = get_arrays(eod);
                (sym.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let first_len = FIRST_CHUNK.min(stock_data[0].1.len());

            let inputs_4: [&[&[f64]; 3]; 4] = [
                &[
                    &stock_data[0].1[..first_len],
                    &stock_data[0].2[..first_len],
                    &stock_data[0].3[..first_len],
                ],
                &[
                    &stock_data[1].1[..first_len],
                    &stock_data[1].2[..first_len],
                    &stock_data[1].3[..first_len],
                ],
                &[
                    &stock_data[2].1[..first_len],
                    &stock_data[2].2[..first_len],
                    &stock_data[2].3[..first_len],
                ],
                &[
                    &stock_data[3].1[..first_len],
                    &stock_data[3].2[..first_len],
                    &stock_data[3].3[..first_len],
                ],
            ];

            let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by_assets first chunk failed");

            let names = ["conversion", "base", "span_a", "span_b"];
            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                // Full scalar reference
                let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (full_out, _) =
                    rust_ichimoku(&inputs, &options, None).expect("scalar full run failed");

                // Stitch: SIMD first chunk + batch_indicator for the rest
                let mut combined: Vec<Vec<f64>> =
                    (0..4).map(|k| simd_first[asset_idx][k].clone()).collect();

                let rem_high = &high[first_len..];
                let rem_low = &low[first_len..];
                let rem_close = &close[first_len..];

                if !rem_high.is_empty() {
                    let batch_out = states[asset_idx]
                        .batch_indicator(&[rem_high, rem_low, rem_close], None)
                        .expect("batch_indicator failed");
                    for k in 0..4 {
                        combined[k].extend_from_slice(&batch_out[k]);
                    }
                }

                for k in 0..4 {
                    let full_line = &full_out[k];
                    let combined_line = &combined[k];
                    assert_eq!(
                        combined_line.len(),
                        full_line.len(),
                        "{} length mismatch: stock={stock_symbol}, options={:?}",
                        names[k],
                        options
                    );
                    for (i, (&cv, &fv)) in combined_line.iter().zip(full_line.iter()).enumerate() {
                        assert!(
                            cv.is_finite(),
                            "combined {} NaN/Inf at {i}: stock={stock_symbol}",
                            names[k]
                        );
                        let diff = (cv - fv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i}: combined={cv}, full={fv}, diff={diff:.2e}, \
                             stock={stock_symbol}, options={:?}",
                            names[k],
                            options
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by_options state continuity
    // Seed the first FIRST_CHUNK bars with indicator_by_options, then drive the
    // remaining bars via each lane's returned IndicatorState.batch_indicator.
    // The stitched output must match a full scalar run bar-for-bar per lane.
    // -------------------------------------------------------------------------

    #[test]
    fn test_ichimoku_simd_by_options_state_continuity() {
        use tulip_rs::indicators::ichimoku::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4: [&[f64; 2]; 4] = [&[9.0, 26.0], &[7.0, 22.0], &[5.0, 15.0], &[14.0, 45.0]];

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let first_len = FIRST_CHUNK.min(high.len());

            let seed_inputs = [
                &high[..first_len] as &[f64],
                &low[..first_len],
                &close[..first_len],
            ];

            let (simd_first, mut states) =
                indicator_by_options::<4>(&seed_inputs, &options_4, None)
                    .expect("SIMD by_options first chunk failed");

            let names = ["conversion", "base", "span_a", "span_b"];
            let rem_high = &high[first_len..];
            let rem_low = &low[first_len..];
            let rem_close = &close[first_len..];

            for (lane, &options) in options_4.iter().enumerate() {
                // Full scalar reference for this option set
                let full_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (full_out, _) =
                    rust_ichimoku(&full_inputs, options, None).expect("scalar full run failed");

                // Stitch: SIMD first chunk + batch_indicator for the rest
                let mut combined: Vec<Vec<f64>> =
                    (0..4).map(|k| simd_first[lane][k].clone()).collect();

                if !rem_high.is_empty() {
                    let batch_out = states[lane]
                        .batch_indicator(&[rem_high, rem_low, rem_close], None)
                        .expect("batch_indicator failed");
                    for k in 0..4 {
                        combined[k].extend_from_slice(&batch_out[k]);
                    }
                }

                for k in 0..4 {
                    let full_line = &full_out[k];
                    let combined_line = &combined[k];
                    assert_eq!(
                        combined_line.len(),
                        full_line.len(),
                        "{} length mismatch: stock={stock_symbol}, options={:?}",
                        names[k],
                        options
                    );
                    for (i, (&cv, &fv)) in combined_line.iter().zip(full_line.iter()).enumerate() {
                        assert!(
                            cv.is_finite(),
                            "combined {} NaN/Inf at {i}: stock={stock_symbol}, lane={lane}",
                            names[k]
                        );
                        let diff = (cv - fv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i}: combined={cv}, full={fv}, diff={diff:.2e}, \
                             stock={stock_symbol}, options={:?}",
                            names[k],
                            options
                        );
                    }
                }
            }
        }
    }

    }
