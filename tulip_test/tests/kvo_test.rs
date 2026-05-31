#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::kvo::{indicator as rust_kvo, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_kvo, ti_kvo_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

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

    const OPTIONS_LIST: [[f64; 2]; 7] = [
        [2.0, 5.0],
        [9.0, 26.0],
        [10.0, 50.0],
        [12.0, 26.0],
        [14.0, 30.0],
        [20.0, 50.0],
        [50.0, 200.0],
    ];

    fn get_hlcv_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
        (high, low, close, volume)
    }

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        let mut volume_vec = VOLUME.to_vec();
        for _ in 0..100 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
            volume_vec.extend_from_slice(&VOLUME);
        }
        (high_vec, low_vec, close_vec, volume_vec)
    }

    #[test]
    fn test_kvo_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close, volume) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> =
                vec![high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr()];

            // Determine the offset required by the C KVO function
            let start_index = unsafe { ti_kvo_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_kvo_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut kvo_output_vec_c = vec![0.0_f64; output_len_c];
            let kvo_ptr: *mut f64 = kvo_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![kvo_ptr];
            let ret = unsafe {
                ti_kvo(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_kvo returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];
            let (outputs, _) =
                rust_kvo(&inputs_rust, &options, None).expect("Rust KVO indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in kvo_output_vec_c
                .iter()
                .rev()
                .take(output_len_rust)
                .zip(outputs[0].iter().rev())
                .enumerate()
            {
                let index = output_len_rust - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust KVO has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust KVO has infinity at index {}: Rust = {}",
                        index, rust_val
                    );
                }

                // Skip if only C has NaN (C bug)
                if c_val.is_nan() && !rust_val.is_nan() {
                    continue;
                }

                // Skip if only C has infinity (C bug)
                if c_val.is_infinite() && !rust_val.is_infinite() {
                    continue;
                }

                if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-6) {
                    // Adjust epsilon if needed
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, kvo_output_vec_c, outputs[0], options
                    );
                    panic!(
                        "Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_kvo_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> =
                    vec![high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr()];

                let start_index = unsafe { ti_kvo_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_kvo_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut kvo_output_vec_c = vec![0.0_f64; output_len_c];
                let kvo_ptr: *mut f64 = kvo_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![kvo_ptr];
                let ret = unsafe {
                    ti_kvo(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_kvo returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [
                    high.as_slice(),
                    low.as_slice(),
                    close.as_slice(),
                    volume.as_slice(),
                ];
                let (outputs, _) =
                    rust_kvo(&inputs_rust, &options, None).expect("Rust KVO indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare results
                for (i, (&c_val, &rust_val)) in kvo_output_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust)
                    .zip(outputs[0].iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() {
                        panic!(
                            "Rust KVO has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust KVO has infinity at index {}: Rust = {}",
                            index, rust_val
                        );
                    }

                    // Skip if only C has NaN (C bug)
                    if c_val.is_nan() && !rust_val.is_nan() {
                        continue;
                    }

                    // Skip if only C has infinity (C bug)
                    if c_val.is_infinite() && !rust_val.is_infinite() {
                        continue;
                    }

                    if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-3) {
                        //} && stock_symbol != "AAPL_NYSE" {
                        let start = i.saturating_sub(10);
                        println!(
                            "Rust kvo results: {:?} \n\nC KVO Results: {:?} \n\n",
                            &outputs[0][start..(i + 10).min(outputs[0].len())], //[..10.min(simd_aroon_down.len())],
                            &kvo_output_vec_c[start..(i + 10).min(kvo_output_vec_c.len())]
                        );
                        /*println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, kvo_output_vec_c, outputs[0], options, stock_symbol
                        );*/
                        panic!(
                            "Stock {}, Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            stock_symbol, index, c_val, rust_val, options
                        );
                    }
                }
            }
        }
    }

    const CHUNK_SIZE: usize = 100;

    #[test]
    fn test_kvo_database_state() {
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
                // Get full output
                let (full_outputs, _) = rust_kvo(&inputs_rust, &options, None)
                    .expect("KVO indicator should work on full data");

                // Process in batches
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // Process first chunk to get initial state
                let first_chunk_size = min_data_val.min(high.len());
                let first_high = high[..first_chunk_size].to_vec();
                let first_low = low[..first_chunk_size].to_vec();
                let first_close = close[..first_chunk_size].to_vec();
                let first_volume = volume[..first_chunk_size].to_vec();
                let first_inputs = [
                    first_high.as_slice(),
                    first_low.as_slice(),
                    first_close.as_slice(),
                    first_volume.as_slice(),
                ];

                let (outputs, mut state) = rust_kvo(&first_inputs, &options, None)
                    .expect("KVO indicator should work on first chunk");

                for output_idx in 0..outputs.len() {
                    batch_full_outputs[output_idx].extend_from_slice(&outputs[output_idx]);
                }

                let mut processed = first_chunk_size;

                // Process subsequent chunks using state.batch_indicator
                while processed < high.len() {
                    let end = (processed + CHUNK_SIZE).min(high.len());

                    let chunk_high = high[processed..end].to_vec();
                    let chunk_low = low[processed..end].to_vec();
                    let chunk_close = close[processed..end].to_vec();
                    let chunk_volume = volume[processed..end].to_vec();
                    let chunk_inputs = [
                        chunk_high.as_slice(),
                        chunk_low.as_slice(),
                        chunk_close.as_slice(),
                        chunk_volume.as_slice(),
                    ];

                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("KVO batch indicator failed");

                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }

                    processed = end;
                }

                // Compare all outputs
                for output_idx in 0..full_outputs.len() {
                    assert_eq!(
                        full_outputs[output_idx].len(),
                        batch_full_outputs[output_idx].len(),
                        "Output length mismatch for stock {}, output {}, options {:?}",
                        stock_symbol,
                        output_idx,
                        options
                    );

                    for (i, (&full_val, &batch_val)) in full_outputs[output_idx]
                        .iter()
                        .zip(batch_full_outputs[output_idx].iter())
                        .enumerate()
                    {
                        assert_eq!(
                            full_val, batch_val,
                            "State handover test failed for stock {}, output {}, index {}, options {:?}: full = {}, batch = {}",
                            stock_symbol, output_idx, i, options, full_val, batch_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_kvo_simd_vs_regular_database() {
        use tulip_rs::indicators::kvo::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, close, volume) = get_hlcv_arrays(data);
                (symbol.clone(), high, low, close, volume)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 4]; 4] = [
            &[
                &stock_data[0].1,
                &stock_data[0].2,
                &stock_data[0].3,
                &stock_data[0].4,
            ], // high, low, close, volume
            &[
                &stock_data[1].1,
                &stock_data[1].2,
                &stock_data[1].3,
                &stock_data[1].4,
            ], // high, low, close, volume
            &[
                &stock_data[2].1,
                &stock_data[2].2,
                &stock_data[2].3,
                &stock_data[2].4,
            ], // high, low, close, volume
            &[
                &stock_data[3].1,
                &stock_data[3].2,
                &stock_data[3].3,
                &stock_data[3].4,
            ], // high, low, close, volume
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets KVO indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, high, low, close, volume)) in
                stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [
                    high.as_slice(),
                    low.as_slice(),
                    close.as_slice(),
                    volume.as_slice(),
                ];
                let (regular_results, _) =
                    rust_kvo(&stock_inputs, &options, None).expect("Regular KVO indicator failed");

                let simd_result = &simd_results[stock_idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "Output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result.len(),
                    regular_result.len()
                );

                // Compare each value
                for (i, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                                "SIMD by assets KVO has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                                "SIMD by assets KVO has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                    }

                    // Compare values with appropriate epsilon for KVO
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-4) {
                        println!(
                            "SIMD: {:?}\n\nRegular: {:?}",
                            &simd_result[..20.min(simd_result.len())],
                            &regular_result[..20.min(regular_result.len())]
                        );
                        panic!(
                                "Mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                    }
                }

                println!(
                    "✓ SIMD by assets vs Regular test passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All SIMD by assets vs Regular KVO database tests passed!");
    }

    #[test]
    fn test_kvo_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::kvo::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, close, volume) = get_hlcv_arrays(data);
                (symbol.clone(), high, low, close, volume)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 4]; 4] = [
            &[
                &stock_data[0].1,
                &stock_data[0].2,
                &stock_data[0].3,
                &stock_data[0].4,
            ], // high, low, close, volume
            &[
                &stock_data[1].1,
                &stock_data[1].2,
                &stock_data[1].3,
                &stock_data[1].4,
            ], // high, low, close, volume
            &[
                &stock_data[2].1,
                &stock_data[2].2,
                &stock_data[2].3,
                &stock_data[2].4,
            ], // high, low, close, volume
            &[
                &stock_data[3].1,
                &stock_data[3].2,
                &stock_data[3].3,
                &stock_data[3].4,
            ], // high, low, close, volume
        ];

        for options in OPTIONS_LIST {
            // Test with optional outputs
            {
                // Get SIMD by assets result with optional outputs
                let (simd_results_opt, _) =
                    indicator_by_assets::<4>(&inputs, &options, Some(&[true, true]))
                        .expect("SIMD by assets KVO indicator with optional outputs failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, high, low, close, volume)) in
                    stock_data.iter().enumerate()
                {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [
                        high.as_slice(),
                        low.as_slice(),
                        close.as_slice(),
                        volume.as_slice(),
                    ];
                    let (regular_results_opt, _) =
                        rust_kvo(&stock_inputs, &options, Some(&[true, true]))
                            .expect("Regular KVO indicator with optional outputs failed");

                    // Compare all outputs: KVO, short_ema, long_ema
                    let output_names = ["KVO", "short_ema", "long_ema"];
                    for (output_idx, output_name) in output_names.iter().enumerate() {
                        let simd_result = &simd_results_opt[stock_idx][output_idx];
                        let regular_result = &regular_results_opt[output_idx];

                        // Skip empty optional outputs
                        if simd_result.is_empty() && regular_result.is_empty() {
                            continue;
                        }

                        // Compare output lengths
                        assert_eq!(
                                simd_result.len(),
                                regular_result.len(),
                                "Output length mismatch for {} output of stock {} with options {:?}: SIMD={}, Regular={}",
                                output_name,
                                stock_symbol,
                                options,
                                simd_result.len(),
                                regular_result.len()
                            );

                        // Compare each value
                        for (i, (&simd_val, &regular_val)) in
                            simd_result.iter().zip(regular_result.iter()).enumerate()
                        {
                            // Check for NaN/infinity in SIMD result
                            if simd_val.is_nan() {
                                panic!(
                                        "SIMD by assets {} has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                        output_name, i, stock_symbol, options, simd_val
                                    );
                            }

                            if simd_val.is_infinite() {
                                panic!(
                                        "SIMD by assets {} has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                        output_name, i, stock_symbol, options, simd_val
                                    );
                            }

                            // Compare values with appropriate epsilon for KVO
                            if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-4) {
                                panic!(
                                        "Mismatch in {} output at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                        output_name, i, stock_symbol, options, simd_val, regular_val
                                    );
                            }
                        }
                    }

                    println!(
                            "✓ SIMD by assets vs Regular optional outputs test passed for stock {} with options {:?}",
                            stock_symbol, options
                        );
                }
            }
        }

        println!("✓ All SIMD by assets vs Regular KVO optional outputs database tests passed!");
    }

    #[test]
    fn test_kvo_optional_outputs_validation() {
        use tulip_rs::indicators::ema;

        let high = HIGH.to_vec();
        let low = LOW.to_vec();
        let close = CLOSE.to_vec();
        let volume = VOLUME.to_vec();
        let inputs = [
            high.as_slice(),
            low.as_slice(),
            close.as_slice(),
            volume.as_slice(),
        ];
        let options = OPTIONS_LIST[0]; // Use first option set from OPTIONS_LIST
        let optional_outputs = Some([true, true].as_slice()); // Request both short_ema and long_ema outputs

        // Get Rust KVO output with optional outputs
        let result = rust_kvo(&inputs, &options, optional_outputs).unwrap();
        let rust_short_ema = &result.0[1]; // short_ema is at index 1
        let rust_long_ema = &result.0[2]; // long_ema is at index 2

        // Validate that optional outputs are not empty
        if rust_short_ema.is_empty() {
            panic!("Rust KVO short_ema optional output is empty - this indicates an indicator bug");
        }
        if rust_long_ema.is_empty() {
            panic!("Rust KVO long_ema optional output is empty - this indicates an indicator bug");
        }

        // Calculate expected lengths using EMA output_length function
        let short_expected_len = ema::output_length(inputs[0].len(), &[options[0]]);
        let long_expected_len = ema::output_length(inputs[0].len(), &[options[1]]);

        println!("KVO Optional Outputs Validation:");
        /*println!(
            "Short EMA - Expected length: {}, Actual length: {}",
            short_expected_len,
            rust_short_ema.len()
        );
        println!(
            "Long EMA - Expected length: {}, Actual length: {}",
            long_expected_len,
            rust_long_ema.len()
        );*/

        // Validate lengths match expected EMA lengths (allowing for KVO's specific data requirements)
        // Note: KVO might have different start requirements than pure EMA due to VF calculation
        assert!(
            !rust_short_ema.is_empty(),
            "Short EMA output should not be empty"
        );
        assert!(
            !rust_long_ema.is_empty(),
            "Long EMA output should not be empty"
        );

        // Check that actual lengths exactly match expected EMA lengths
        assert_eq!(
            rust_short_ema.len(),
            short_expected_len,
            "Short EMA length {} should equal expected EMA length {}",
            rust_short_ema.len(),
            short_expected_len
        );
        assert_eq!(
            rust_long_ema.len(),
            long_expected_len,
            "Long EMA length {} should equal expected EMA length {}",
            rust_long_ema.len(),
            long_expected_len
        );

        // Since EMA is calculated on VF (volume force), we can't compare exact values
        // but we can validate that outputs are finite, non-zero, and reasonable
        for (i, &value) in rust_short_ema.iter().enumerate() {
            if !value.is_finite() {
                panic!(
                    "Short EMA contains NaN/infinity at position {}: {}",
                    i, value
                );
            }
            if value == 0.0 {
                panic!("Short EMA contains zero value at position {}: {}", i, value);
            }
        }

        for (i, &value) in rust_long_ema.iter().enumerate() {
            if !value.is_finite() {
                panic!(
                    "Long EMA contains NaN/infinity at position {}: {}",
                    i, value
                );
            }
            if value == 0.0 {
                panic!("Long EMA contains zero value at position {}: {}", i, value);
            }
        }

        // Validate that short and long EMAs have reasonable relationship
        // (this is a sanity check, not a strict mathematical requirement)
        let short_avg = rust_short_ema.iter().sum::<f64>() / rust_short_ema.len() as f64;
        let long_avg = rust_long_ema.iter().sum::<f64>() / rust_long_ema.len() as f64;

        /*println!("Short EMA average: {:.6}", short_avg);
        println!("Long EMA average: {:.6}", long_avg);*/

        // Both averages should be finite
        assert!(short_avg.is_finite(), "Short EMA average should be finite");
        assert!(long_avg.is_finite(), "Long EMA average should be finite");

        println!("✓ KVO optional outputs validation passed - all values are finite and reasonable");
    }

    #[test]
    fn test_kvo_optional_outputs_database_validation() {
        use tulip_rs::indicators::ema;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        println!("Testing KVO optional outputs with database data...");

        for (stock_symbol, stock_data) in data.iter().take(5) {
            // Skip stocks with insufficient data
            if stock_data.len() < 250 {
                continue;
            }

            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            for &options in OPTIONS_LIST.iter() {
                // Skip options that require more data than available
                let min_required = rust_kvo(&inputs[..4].try_into().unwrap(), &options, None)
                    .map(|_| true)
                    .unwrap_or(false);
                if !min_required {
                    continue;
                }

                let optional_outputs = Some([true, true].as_slice()); // Request both EMAs

                let result =
                    match rust_kvo(&inputs[..4].try_into().unwrap(), &options, optional_outputs) {
                        Ok(result) => result,
                        Err(_) => continue, // Skip if not enough data
                    };

                let rust_short_ema = &result.0[1];
                let rust_long_ema = &result.0[2];

                // Validate outputs are not empty
                assert!(
                    !rust_short_ema.is_empty(),
                    "Stock {} with options {:?}: Short EMA output is empty",
                    stock_symbol,
                    options
                );
                assert!(
                    !rust_long_ema.is_empty(),
                    "Stock {} with options {:?}: Long EMA output is empty",
                    stock_symbol,
                    options
                );

                // Calculate expected lengths
                let short_expected_len = ema::output_length(inputs[0].len(), &[options[0]]);
                let long_expected_len = ema::output_length(inputs[0].len(), &[options[1]]);

                // Validate lengths match expected EMA lengths (allowing for KVO's specific requirements)
                assert!(
                    !rust_short_ema.is_empty(),
                    "Stock {} with options {:?}: Short EMA length should be positive, got {}",
                    stock_symbol,
                    options,
                    rust_short_ema.len()
                );
                assert!(
                    !rust_long_ema.is_empty(),
                    "Stock {} with options {:?}: Long EMA length should be positive, got {}",
                    stock_symbol,
                    options,
                    rust_long_ema.len()
                );

                // Check that actual lengths exactly match expected EMA lengths
                assert_eq!(
                    rust_short_ema.len(),
                    short_expected_len,
                    "Stock {} with options {:?}: Short EMA length {} should equal expected EMA length {}",
                    stock_symbol,
                    options,
                    rust_short_ema.len(),
                    short_expected_len
                );
                assert_eq!(
                    rust_long_ema.len(),
                    long_expected_len,
                    "Stock {} with options {:?}: Long EMA length {} should equal expected EMA length {}",
                    stock_symbol,
                    options,
                    rust_long_ema.len(),
                    long_expected_len
                );

                // Check for NaN, infinity, and zero values in short EMA
                for (i, &value) in rust_short_ema.iter().enumerate() {
                    if !value.is_finite() {
                        panic!(
                            "Stock {} with options {:?}: Short EMA contains NaN/infinity at position {}: {}",
                            stock_symbol, options, i, value
                        );
                    }
                    if value == 0.0 {
                        panic!(
                            "Stock {} with options {:?}: Short EMA contains zero value at position {}: {}",
                            stock_symbol, options, i, value
                        );
                    }
                }

                // Check for NaN, infinity, and zero values in long EMA
                for (i, &value) in rust_long_ema.iter().enumerate() {
                    if !value.is_finite() {
                        panic!(
                            "Stock {} with options {:?}: Long EMA contains NaN/infinity at position {}: {}",
                            stock_symbol, options, i, value
                        );
                    }
                    if value == 0.0 {
                        panic!(
                            "Stock {} with options {:?}: Long EMA contains zero value at position {}: {}",
                            stock_symbol, options, i, value
                        );
                    }
                }

                // Validate averages are finite
                let short_avg = rust_short_ema.iter().sum::<f64>() / rust_short_ema.len() as f64;
                let long_avg = rust_long_ema.iter().sum::<f64>() / rust_long_ema.len() as f64;

                assert!(
                    short_avg.is_finite(),
                    "Stock {} with options {:?}: Short EMA average should be finite, got {}",
                    stock_symbol,
                    options,
                    short_avg
                );
                assert!(
                    long_avg.is_finite(),
                    "Stock {} with options {:?}: Long EMA average should be finite, got {}",
                    stock_symbol,
                    options,
                    long_avg
                );

                println!(
                    "✓ Stock {} with options {:?}: Short EMA len={}, Long EMA len={}",
                    stock_symbol,
                    options,
                    rust_short_ema.len(),
                    rust_long_ema.len()
                );
            }
        }

        println!("✓ KVO optional outputs database validation passed for all stocks and options!");
    }

    #[test]
    fn test_kvo_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::kvo::indicator as rust_kvo;
        use tulip_rs::indicators::kvo::indicator_by_options;

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

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD KVO 4-wide failed");

            // Process next 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD KVO 2-wide failed");

            // Process last option with regular indicator (as single lane)
            let (simd_results_1, _) = rust_kvo(&inputs, &OPTIONS_LIST[6], None)
                .expect("Regular KVO indicator failed for last option");

            // Combine SIMD results
            let mut all_simd_results = Vec::new();

            // Add 4-wide results
            for result in &simd_results_4 {
                all_simd_results.push(result.clone());
            }

            // Add 2-wide results
            for result in &simd_results_2 {
                all_simd_results.push(result.clone());
            }

            // Add single result
            all_simd_results.push(simd_results_1);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_kvo(&inputs, options, None).expect("Regular KVO indicator failed");

                let simd_result = &all_simd_results[idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "Output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result.len(),
                    regular_result.len()
                );

                // Compare each value
                for (i, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by options KVO has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options KVO has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with appropriate epsilon for KVO
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-4) {
                        /*println!(
                            "SIMD: {:?}\n\nRegular: {:?}",
                            &simd_result[..20.min(simd_result.len())],
                            &regular_result[..20.min(regular_result.len())]
                        );*/
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }

            println!(
                "✓ SIMD by options vs Regular test passed for stock {}",
                stock_symbol
            );
        }

        println!("✓ All SIMD by options vs Regular KVO database tests passed!");
    }
}
