#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::smaenvelope::{
        SmaEnvelope, Indicator, IndicatorState, TIndicatorState,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    /// Epsilon for SIMD vs scalar comparisons.
    const MARGIN: f64 = 1e-10;
    /// Epsilon for JSON round-trip comparisons (JSON f64 serialisation drops
    /// the last ULP or two, so we allow a small tolerance).
    const JSON_EPS: f64 = 1e-9;
    const CHUNK_SIZE: usize = 100;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    // (period, percentage)
    const OPTIONS_LIST: [[f64; 2]; 4] = [[5.0, 2.5], [14.0, 2.5], [20.0, 5.0], [50.0, 5.0]];

    /// Returns a synthetic dataset long enough for all option sets (period up to 50).
    fn expand_close() -> Vec<f64> {
        let mut v = CLOSE.to_vec();
        for _ in 0..100 {
            v.extend_from_slice(&CLOSE);
        }
        v // 1_515 bars
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // -------------------------------------------------------------------------
    // Sanity checks on the full indicator
    // -------------------------------------------------------------------------

    /// Verify output length, absence of NaN/Inf, and that bands are ordered
    /// lower ≤ middle ≤ upper for all positive price inputs.
    #[test]
    fn test_smaenvelope_indicator_sanity() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            let inputs = [close.as_slice()];
            let (outputs, _) =
                SmaEnvelope::indicator(&inputs, &options, None).expect("SMA Envelope indicator failed");

            assert_eq!(outputs.len(), 3, "expected 3 output bands");

            let expected_len = SmaEnvelope::output_length(close.len(), &options);
            assert_eq!(
                outputs[0].len(),
                expected_len,
                "lower band length mismatch for options {:?}",
                options
            );
            assert_eq!(
                outputs[1].len(),
                expected_len,
                "middle band length mismatch for options {:?}",
                options
            );
            assert_eq!(
                outputs[2].len(),
                expected_len,
                "upper band length mismatch for options {:?}",
                options
            );

            let (lower, middle, upper) = (&outputs[0], &outputs[1], &outputs[2]);

            for i in 0..expected_len {
                assert!(
                    !lower[i].is_nan(),
                    "lower[{i}] is NaN for options {:?}",
                    options
                );
                assert!(
                    !middle[i].is_nan(),
                    "middle[{i}] is NaN for options {:?}",
                    options
                );
                assert!(
                    !upper[i].is_nan(),
                    "upper[{i}] is NaN for options {:?}",
                    options
                );
                assert!(
                    !lower[i].is_infinite(),
                    "lower[{i}] is infinite for options {:?}",
                    options
                );
                assert!(
                    !middle[i].is_infinite(),
                    "middle[{i}] is infinite for options {:?}",
                    options
                );
                assert!(
                    !upper[i].is_infinite(),
                    "upper[{i}] is infinite for options {:?}",
                    options
                );
                assert!(
                    lower[i] <= middle[i],
                    "lower > middle at index {i} for options {:?}: lower={}, middle={}",
                    options,
                    lower[i],
                    middle[i]
                );
                assert!(
                    middle[i] <= upper[i],
                    "middle > upper at index {i} for options {:?}: middle={}, upper={}",
                    options,
                    middle[i],
                    upper[i]
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // Invalid option rejection
    // -------------------------------------------------------------------------

    #[test]
    fn test_smaenvelope_invalid_options_period_zero() {
        let close = expand_close();
        let inputs = [close.as_slice()];
        let result = SmaEnvelope::indicator(&inputs, &[0.0, 2.5], None);
        assert!(result.is_err(), "expected Err for period=0, got Ok");
    }

    #[test]
    fn test_smaenvelope_invalid_options_percentage_zero() {
        let close = expand_close();
        let inputs = [close.as_slice()];
        let result = SmaEnvelope::indicator(&inputs, &[14.0, 0.0], None);
        assert!(result.is_err(), "expected Err for percentage=0, got Ok");
    }

    #[test]
    fn test_smaenvelope_invalid_options_percentage_negative() {
        let close = expand_close();
        let inputs = [close.as_slice()];
        let result = SmaEnvelope::indicator(&inputs, &[14.0, -1.0], None);
        assert!(result.is_err(), "expected Err for percentage=-1, got Ok");
    }

    #[test]
    fn test_smaenvelope_too_few_inputs() {
        // Only 1 bar — always too short for any valid period
        let inputs = [&[100.0_f64][..]];
        let result = SmaEnvelope::indicator(&inputs, &[5.0, 2.5], None);
        assert!(result.is_err(), "expected Err for 1-bar input, got Ok");
    }

    // -------------------------------------------------------------------------
    // batch_indicator vs full indicator — chunked streaming
    // -------------------------------------------------------------------------

    /// Process data in chunks of CHUNK_SIZE and verify every output value
    /// produced by `batch_indicator` exactly matches the corresponding value
    /// from the full `indicator` call.
    #[test]
    fn test_smaenvelope_batch_vs_full_chunked() {
        let close = expand_close();
        let inputs = [close.as_slice()];

        for options in OPTIONS_LIST {
            // Reference: run the full indicator on all data
            let (full_outputs, _) = SmaEnvelope::indicator(&inputs, &options, None)
                .expect("SMA Envelope full indicator failed");

            // Streaming: seed state with the minimum required bars, then
            // process the remainder in CHUNK_SIZE chunks
            let min = SmaEnvelope::min_data(&options);
            // Ensure the seed slice is at least CHUNK_SIZE so we have a
            // meaningful first batch
            let seed_len = min.max(CHUNK_SIZE);

            let mut batch_outputs: Vec<Vec<f64>> = vec![Vec::new(); 3];

            let chunk_inputs = [&close[..seed_len]];
            let (first_out, mut state) = SmaEnvelope::indicator(&chunk_inputs, &options, None)
                .expect("SMA Envelope seed indicator failed");
            for band in 0..3 {
                batch_outputs[band].extend_from_slice(&first_out[band]);
            }

            let mut chunks = close[seed_len..].chunks_exact(CHUNK_SIZE);
            for chunk in chunks.by_ref() {
                let chunk_inputs = [chunk];
                let out = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("SMA Envelope batch_indicator failed");
                for band in 0..3 {
                    batch_outputs[band].extend_from_slice(&out[band]);
                }
            }

            let remainder = chunks.remainder();
            if !remainder.is_empty() {
                let chunk_inputs = [remainder];
                let out = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("SMA Envelope batch_indicator (remainder) failed");
                for band in 0..3 {
                    batch_outputs[band].extend_from_slice(&out[band]);
                }
            }

            // Compare
            for band in 0..3 {
                assert_eq!(
                    full_outputs[band].len(),
                    batch_outputs[band].len(),
                    "band {} length mismatch for options {:?}: full={}, batch={}",
                    band,
                    options,
                    full_outputs[band].len(),
                    batch_outputs[band].len()
                );
                for (i, (&full_val, &batch_val)) in full_outputs[band]
                    .iter()
                    .zip(batch_outputs[band].iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "band {} mismatch at index {} for options {:?}: full={}, batch={}",
                        band, i, options, full_val, batch_val
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // batch_indicator vs full indicator — 1-bar streaming
    // -------------------------------------------------------------------------

    /// Feed data one bar at a time via `batch_indicator` and verify every
    /// output matches the full indicator.
    #[test]
    fn test_smaenvelope_batch_vs_full_one_bar_at_a_time() {
        let close = expand_close();
        let inputs = [close.as_slice()];

        for options in OPTIONS_LIST {
            // Reference
            let (full_outputs, _) = SmaEnvelope::indicator(&inputs, &options, None)
                .expect("SMA Envelope full indicator failed");

            // Seed with exactly SmaEnvelope::min_data bars to obtain state
            let min = SmaEnvelope::min_data(&options);
            let chunk_inputs = [&close[..min]];
            let (first_out, mut state) =
                SmaEnvelope::indicator(&chunk_inputs, &options, None).expect("SMA Envelope seed failed");

            let mut batch_outputs: Vec<Vec<f64>> = vec![Vec::new(); 3];
            for band in 0..3 {
                batch_outputs[band].extend_from_slice(&first_out[band]);
            }

            // Feed one bar at a time
            for bar in &close[min..] {
                let single = std::slice::from_ref(bar);
                let chunk_inputs = [single];
                let out = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("SMA Envelope 1-bar batch_indicator failed");
                for band in 0..3 {
                    batch_outputs[band].extend_from_slice(&out[band]);
                }
            }

            // Compare
            for band in 0..3 {
                assert_eq!(
                    full_outputs[band].len(),
                    batch_outputs[band].len(),
                    "band {} length mismatch (1-bar) for options {:?}",
                    band,
                    options
                );
                for (i, (&full_val, &batch_val)) in full_outputs[band]
                    .iter()
                    .zip(batch_outputs[band].iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "band {} mismatch at index {} (1-bar) for options {:?}: full={}, batch={}",
                        band, i, options, full_val, batch_val
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // batch_indicator vs full indicator — JSON state round-trip
    // -------------------------------------------------------------------------

    /// Serialize the `IndicatorState` to JSON, deserialize it, then continue
    /// streaming and verify the output still matches the full indicator.
    #[test]
    fn test_smaenvelope_batch_vs_full_json_round_trip() {
        let close = expand_close();
        let inputs = [close.as_slice()];

        for options in OPTIONS_LIST {
            // Reference
            let (full_outputs, _) = SmaEnvelope::indicator(&inputs, &options, None)
                .expect("SMA Envelope full indicator failed");

            let min = SmaEnvelope::min_data(&options);
            let seed_len = min.max(CHUNK_SIZE);

            // Seed up to seed_len bars to obtain initial state
            let chunk_inputs = [&close[..seed_len]];
            let (first_out, state) =
                SmaEnvelope::indicator(&chunk_inputs, &options, None).expect("SMA Envelope seed failed");

            let mut batch_outputs: Vec<Vec<f64>> = vec![Vec::new(); 3];
            for band in 0..3 {
                batch_outputs[band].extend_from_slice(&first_out[band]);
            }

            // Serialize → deserialize the state
            let json =
                serde_json::to_string(&state).expect("IndicatorState JSON serialisation failed");
            let mut state: IndicatorState =
                serde_json::from_str(&json).expect("IndicatorState JSON deserialisation failed");

            // Continue streaming the remainder
            let mut chunks = close[seed_len..].chunks_exact(CHUNK_SIZE);
            for chunk in chunks.by_ref() {
                let chunk_inputs = [chunk];
                let out = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("SMA Envelope batch_indicator (post-JSON) failed");
                for band in 0..3 {
                    batch_outputs[band].extend_from_slice(&out[band]);
                }
            }

            let remainder = chunks.remainder();
            if !remainder.is_empty() {
                let chunk_inputs = [remainder];
                let out = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("SMA Envelope batch_indicator (remainder, post-JSON) failed");
                for band in 0..3 {
                    batch_outputs[band].extend_from_slice(&out[band]);
                }
            }

            // Compare
            for band in 0..3 {
                assert_eq!(
                    full_outputs[band].len(),
                    batch_outputs[band].len(),
                    "band {} length mismatch (JSON round-trip) for options {:?}",
                    band,
                    options
                );
                for (i, (&full_val, &batch_val)) in full_outputs[band]
                    .iter()
                    .zip(batch_outputs[band].iter())
                    .enumerate()
                {
                    assert!(
                        approx_eq!(f64, full_val, batch_val, epsilon = JSON_EPS),
                        "band {} mismatch at index {} (JSON round-trip) for options {:?}: full={}, batch={}",
                        band, i, options, full_val, batch_val
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Edge-case: minimum-length input
    // -------------------------------------------------------------------------

    /// When the input is exactly `SmaEnvelope::min_data` bars the indicator should produce
    /// exactly one output value per band.
    #[test]
    fn test_smaenvelope_min_data_produces_one_output() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            let min = SmaEnvelope::min_data(&options);
            let inputs = [&close[..min]];
            let (outputs, _) = SmaEnvelope::indicator(&inputs, &options, None)
                .expect("SMA Envelope failed at SmaEnvelope::min_data length");
            for band in 0..3 {
                assert_eq!(
                    outputs[band].len(),
                    1,
                    "expected 1 output for band {} at SmaEnvelope::min_data length for options {:?}",
                    band,
                    options
                );
            }
        }
    }

    /// When the input has exactly `SmaEnvelope::min_data - 1` bars the indicator must return
    /// an error (too few inputs).
    #[test]
    fn test_smaenvelope_below_min_data_returns_error() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            let min = SmaEnvelope::min_data(&options);
            if min == 0 {
                continue; // nothing to test
            }
            let inputs = [&close[..min - 1]];
            let result = SmaEnvelope::indicator(&inputs, &options, None);
            assert!(
                result.is_err(),
                "expected Err for {} bars (SmaEnvelope::min_data-1) with options {:?}, got Ok",
                min - 1,
                options
            );
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets vs scalar indicator — database
    // -------------------------------------------------------------------------

    /// Compare `indicator_by_assets` (4 lanes, real market data) against the
    /// scalar `indicator` for each stock and every option set.
    #[test]
    fn test_smaenvelope_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::smaenvelope::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Take the first 4 stocks as 4 parallel asset lanes
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
            .collect();

        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        let output_names = ["Lower", "Middle", "Upper"];

        for options in OPTIONS_LIST {
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by-assets SMA Envelope failed");

            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) = SmaEnvelope::indicator(&stock_inputs, &options, None)
                    .expect("Scalar SMA Envelope failed");

                for (output_idx, output_name) in output_names.iter().enumerate() {
                    let simd_band = &simd_results[stock_idx][output_idx];
                    let regular_band = &regular_results[output_idx];

                    assert_eq!(
                        simd_band.len(),
                        regular_band.len(),
                        "{} length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                        output_name,
                        stock_symbol,
                        options,
                        simd_band.len(),
                        regular_band.len()
                    );

                    for (i, (&simd_val, &regular_val)) in
                        simd_band.iter().zip(regular_band.iter()).enumerate()
                    {
                        if simd_val.is_nan() {
                            panic!(
                                "SIMD by-assets {} has NaN at index {} for stock {} with options {:?}",
                                output_name, i, stock_symbol, options
                            );
                        }
                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by-assets {} is infinite at index {} for stock {} with options {:?}",
                                output_name, i, stock_symbol, options
                            );
                        }
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = MARGIN) {
                            panic!(
                                "{} mismatch at index {} for stock {} with options {:?}: SIMD={}, Regular={}",
                                output_name, i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }
                }

                println!(
                    "✓ SIMD by-assets vs Regular passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All SIMD by-assets vs Regular SMA Envelope database tests passed!");
    }

    // -------------------------------------------------------------------------
    // SIMD by-options vs scalar indicator — database
    // -------------------------------------------------------------------------

    /// Compare `indicator_by_options` (4 option lanes, real market data) against
    /// the scalar `indicator` for every stock and every option set.
    #[test]
    fn test_smaenvelope_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::smaenvelope::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        let output_names = ["Lower Band", "Middle Band", "Upper Band"];

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by-options SMA Envelope failed");

            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    SmaEnvelope::indicator(&inputs, options, None).expect("Scalar SMA Envelope failed");

                assert_eq!(
                    simd_results[idx].len(),
                    3,
                    "SIMD result should have 3 outputs"
                );
                assert_eq!(
                    regular_results.len(),
                    3,
                    "Regular result should have 3 outputs"
                );

                for (output_idx, output_name) in output_names.iter().enumerate() {
                    let simd_band = &simd_results[idx][output_idx];
                    let regular_band = &regular_results[output_idx];

                    assert_eq!(
                        simd_band.len(),
                        regular_band.len(),
                        "{} length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                        output_name,
                        stock_symbol,
                        options,
                        simd_band.len(),
                        regular_band.len()
                    );

                    for (i, (&simd_val, &regular_val)) in
                        simd_band.iter().zip(regular_band.iter()).enumerate()
                    {
                        if simd_val.is_nan() {
                            panic!(
                                "SIMD by-options {} has NaN at index {} for stock {} options {:?}",
                                output_name, i, stock_symbol, options
                            );
                        }
                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by-options {} is infinite at index {} for stock {} options {:?}",
                                output_name, i, stock_symbol, options
                            );
                        }
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = MARGIN) {
                            panic!(
                                "{} mismatch at index {} for stock {} options {:?}: SIMD={}, Regular={}",
                                output_name, i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }
                }
            }
        }

        println!("\u{2713} All SIMD by-options vs Regular SMA Envelope database tests passed!");
    }

    }
