#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::apo::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_apo, ti_apo_start, ti_ema, ti_ema_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const EPSILON: f64 = 1e-12;
    const EMA_EPSILON: f64 = 1e-12; // Use epsilon from ema_test.rs
    const CHUNK_SIZE: usize = 100;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 2]; 4] = [[2.0, 5.0], [11.0, 21.0], [5.0, 11.0], [14.0, 30.0]];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_close() -> Vec<f64> {
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..3 {
            close_vec.extend_from_slice(&CLOSE);
        }
        close_vec
    }

    #[test]
    fn test_apo_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C APO function
            let start_index = unsafe { ti_apo_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_apo_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut output_vec_c = vec![0.0_f64; output_len_c];
            let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
            let ret = unsafe {
                ti_apo(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_apo returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Rust APO indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in output_vec_c
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
                        "Rust APO has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust APO has infinity at index {}: Rust = {}",
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

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = EPSILON),
                    "Mismatch at index {}: C = {}, Rust = {} for options {:?}",
                    index,
                    c_val,
                    rust_val,
                    options
                );
            }
        }
    }

    #[test]
    fn test_apo_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_apo_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_apo_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_apo(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_apo returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    indicator(&inputs_rust, &options, None).expect("Rust APO indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare results
                for (i, (&c_val, &rust_val)) in output_vec_c
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
                            "Rust APO has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust APO has infinity at index {}: Rust = {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILON) {
                        println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, output_vec_c, outputs[0], options, stock_symbol
                        );
                        panic!(
                            "Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_apo_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs_rust = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = indicator(&inputs_rust, &options, None)
                    .expect("Failed to run APO indicator on full data");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if close.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) = indicator(&inputs_rust, &options, None)
                        .expect("Failed to run APO indicator");
                    batch_full_output.extend_from_slice(&outputs[0]);
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let close_vec = close[..min_data_val].to_vec();
                    let chunk_inputs = [close_vec.as_slice()];

                    let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("Failed to run APO indicator on first chunk");
                    batch_full_output.extend_from_slice(&first_outputs[0]);

                    // Process remaining data in chunks using state
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for close_chunk in close_chunks.by_ref() {
                        let close_vec = close_chunk.to_vec();
                        let chunk_inputs = [close_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("APO batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }

                    // Process remainder if any
                    let close_rem = close_chunks.remainder();

                    if !close_rem.is_empty() {
                        let close_vec = close_rem.to_vec();
                        let chunk_inputs = [close_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("APO batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }
                }

                // Compare outputs
                assert_eq!(
                    full_outputs[0].len(),
                    batch_full_output.len(),
                    "Output length mismatch for stock {} with options {:?}: full={}, batch={}",
                    stock_symbol,
                    options,
                    full_outputs[0].len(),
                    batch_full_output.len()
                );

                for (i, (&full_val, &batch_val)) in full_outputs[0]
                    .iter()
                    .zip(batch_full_output.iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "Mismatch at index {} for stock {} with options {:?}: full={}, batch={}",
                        i, stock_symbol, options, full_val, batch_val
                    );
                }
            }
        }
    }

    #[test]
    fn test_apo_simd_vs_regular_database() {
        use tulip_rs::indicators::apo::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1], // close
            &[&stock_data[1].1], // close
            &[&stock_data[2].1], // close
            &[&stock_data[3].1], // close
        ];

        for options in OPTIONS_LIST {
            // Test without optional outputs
            {
                // Get SIMD by assets result
                let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                    .expect("SIMD by assets APO indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = indicator(&stock_inputs, &options, None)
                        .expect("Regular APO indicator failed");

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
                                "SIMD by assets APO has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets APO has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for APO
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
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
        }

        println!("✓ All SIMD by assets vs Regular APO database tests passed!");
    }

    #[test]
    fn test_apo_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::apo::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1], // close
            &[&stock_data[1].1], // close
            &[&stock_data[2].1], // close
            &[&stock_data[3].1], // close
        ];

        for options in OPTIONS_LIST {
            // Test with optional outputs
            {
                // Get SIMD by assets result with optional outputs
                let (simd_results_opt, _) =
                    indicator_by_assets::<4>(&inputs, &options, Some(&[true, true]))
                        .expect("SIMD by assets APO indicator with optional outputs failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results_opt, _) =
                        indicator(&stock_inputs, &options, Some(&[true, true]))
                            .expect("Regular APO indicator with optional outputs failed");

                    // Compare all outputs: APO, short_ema, long_ema
                    let output_names = ["APO", "short_ema", "long_ema"];
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

                            // Compare values with appropriate epsilon
                            if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                                println!(
                                    "SIMD {}: {:?}\n\nRegular {} {:?}",
                                    output_name,
                                    &simd_result[..],
                                    output_name,
                                    &regular_result[..]
                                );
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

        println!("✓ All SIMD by assets vs Regular APO optional outputs database tests passed!");
    }

    #[test]
    fn test_apo_short_ema_optional_output_vs_c_tulip() {
        // Test the short EMA optional output against C Tulip EMA
        let close = expand_close();

        for options in OPTIONS_LIST {
            println!(
                "Testing APO short EMA optional output with options: {:?}",
                options
            );

            // Get Rust APO with short EMA optional output enabled
            let inputs_rust = [close.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[true, false]))
                .expect("Rust APO indicator with short EMA optional output failed");

            let rust_short_ema = &rust_outputs[1]; // short_ema is at index 1

            // Run C Tulip EMA on the close values with short period
            let ema_inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let short_ema_options = [options[0]]; // short period
            let ema_start_index = unsafe { ti_ema_start(short_ema_options.as_ptr()) };
            let ema_output_len = close.len() - (ema_start_index as usize);

            let mut c_short_ema_output = vec![0.0_f64; ema_output_len];
            let mut ema_outputs_c: Vec<*mut f64> = vec![c_short_ema_output.as_mut_ptr()];
            let ret = unsafe {
                ti_ema(
                    close.len() as i32,
                    ema_inputs_c.as_ptr(),
                    short_ema_options.as_ptr(),
                    ema_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(
                ret, 0,
                "ti_ema for short period returned error code {}",
                ret
            );

            // Compare short EMA outputs from the end backwards for better alignment
            let compare_len = rust_short_ema.len().min(c_short_ema_output.len());
            for i in 0..compare_len {
                let rust_idx = rust_short_ema.len() - 1 - i;
                let c_idx = c_short_ema_output.len() - 1 - i;
                let rust_val = rust_short_ema[rust_idx];
                let c_val = c_short_ema_output[c_idx];

                if rust_val.is_nan() {
                    panic!(
                        "Rust short EMA has NaN at index {} (from end): Rust = {}, Options = {:?}",
                        i, rust_val, options
                    );
                }
                if rust_val.is_infinite() {
                    panic!(
                        "Rust short EMA has infinity at index {} (from end): Rust = {}",
                        i, rust_val
                    );
                }
                if c_val.is_nan() && !rust_val.is_nan() {
                    continue;
                }
                if c_val.is_infinite() && !rust_val.is_infinite() {
                    continue;
                }

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = EPSILON),
                    "Short EMA mismatch at index {} (from end): C = {}, Rust = {} for options {:?}",
                    i,
                    c_val,
                    rust_val,
                    options
                );
            }

            println!(
                "✓ Short EMA optional output validated for options {:?}",
                options
            );
        }

        println!("✓ All APO short EMA optional output tests passed!");
    }

    #[test]
    fn test_apo_long_ema_optional_output_vs_c_tulip() {
        // Test the long EMA optional output against C Tulip EMA
        let close = expand_close();

        for options in OPTIONS_LIST {
            println!(
                "Testing APO long EMA optional output with options: {:?}",
                options
            );

            // Get Rust APO with long EMA optional output enabled
            let inputs_rust = [close.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[false, true]))
                .expect("Rust APO indicator with long EMA optional output failed");

            let rust_long_ema = &rust_outputs[2]; // long_ema is at index 2

            // Run C Tulip EMA on the close values with long period
            let ema_inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let long_ema_options = [options[1]]; // long period
            let ema_start_index = unsafe { ti_ema_start(long_ema_options.as_ptr()) };
            let ema_output_len = close.len() - (ema_start_index as usize);

            let mut c_long_ema_output = vec![0.0_f64; ema_output_len];
            let mut ema_outputs_c: Vec<*mut f64> = vec![c_long_ema_output.as_mut_ptr()];
            let ret = unsafe {
                ti_ema(
                    close.len() as i32,
                    ema_inputs_c.as_ptr(),
                    long_ema_options.as_ptr(),
                    ema_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_ema for long period returned error code {}", ret);

            // Compare long EMA outputs from the end backwards for better alignment
            let compare_len = rust_long_ema.len().min(c_long_ema_output.len());
            for i in 0..compare_len {
                let rust_idx = rust_long_ema.len() - 1 - i;
                let c_idx = c_long_ema_output.len() - 1 - i;
                let rust_val = rust_long_ema[rust_idx];
                let c_val = c_long_ema_output[c_idx];

                if rust_val.is_nan() {
                    panic!(
                        "Rust long EMA has NaN at index {} (from end): Rust = {}, Options = {:?}",
                        i, rust_val, options
                    );
                }
                if rust_val.is_infinite() {
                    panic!(
                        "Rust long EMA has infinity at index {} (from end): Rust = {}",
                        i, rust_val
                    );
                }
                if c_val.is_nan() && !rust_val.is_nan() {
                    continue;
                }
                if c_val.is_infinite() && !rust_val.is_infinite() {
                    continue;
                }

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = EPSILON),
                    "Long EMA mismatch at index {} (from end): C = {}, Rust = {} for options {:?}",
                    i,
                    c_val,
                    rust_val,
                    options
                );
            }

            println!(
                "✓ Long EMA optional output validated for options {:?}",
                options
            );
        }

        println!("✓ All APO long EMA optional output tests passed!");
    }

    #[test]
    fn test_apo_short_ema_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                println!(
                    "Testing APO short EMA optional output with database stock {} and options: {:?}",
                    stock_symbol, options
                );

                // Get Rust APO with short EMA optional output enabled
                let inputs_rust = [close.as_slice()];
                let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[true, false]))
                    .expect("Rust APO indicator with short EMA optional output failed");

                let rust_short_ema = &rust_outputs[1]; // short_ema is at index 1

                if rust_short_ema.is_empty() {
                    panic!(
                        "Rust short EMA optional output is empty for stock {}",
                        stock_symbol
                    );
                }

                // Get C Tulip EMA output for comparison
                let ema_inputs_c = [close.as_ptr()];
                let ema_options = [options[0]]; // short_period
                let ema_start_index = unsafe { ti_ema_start(ema_options.as_ptr()) };
                let ema_output_len = close.len() - (ema_start_index as usize);
                let mut c_short_ema = vec![0.0; ema_output_len];
                let mut ema_outputs_c = vec![c_short_ema.as_mut_ptr()];

                let ret = unsafe {
                    ti_ema(
                        close.len() as i32,
                        ema_inputs_c.as_ptr(),
                        ema_options.as_ptr(),
                        ema_outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(
                    ret, 0,
                    "ti_ema returned error code {} for stock {}",
                    ret, stock_symbol
                );

                // Compare from the end backwards
                let compare_len = rust_short_ema.len().min(c_short_ema.len());
                for i in 0..compare_len {
                    let rust_idx = rust_short_ema.len() - 1 - i;
                    let c_idx = c_short_ema.len() - 1 - i;
                    let rust_val = rust_short_ema[rust_idx];
                    let c_val = c_short_ema[c_idx];

                    if !rust_val.is_finite() {
                        panic!(
                            "Rust short EMA output has NaN/infinity at index {} (from end): Rust = {} for stock {} options {:?}",
                            i, rust_val, stock_symbol, options
                        );
                    }

                    if !c_val.is_finite() {
                        continue; // Skip C library bugs
                    }

                    assert!(
                        approx_eq!(f64, c_val, rust_val, epsilon = EMA_EPSILON),
                        "Short EMA mismatch at index {} (from end): C = {}, Rust = {} for stock {} options {:?}",
                        i,
                        c_val,
                        rust_val,
                        stock_symbol,
                        options
                    );
                }

                println!(
                    "✓ Short EMA optional output validated for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All APO short EMA optional output database tests passed!");
    }

    #[test]
    fn test_apo_long_ema_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                println!(
                    "Testing APO long EMA optional output with database stock {} and options: {:?}",
                    stock_symbol, options
                );

                // Get Rust APO with long EMA optional output enabled
                let inputs_rust = [close.as_slice()];
                let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[false, true]))
                    .expect("Rust APO indicator with long EMA optional output failed");

                let rust_long_ema = &rust_outputs[2]; // long_ema is at index 2

                if rust_long_ema.is_empty() {
                    panic!(
                        "Rust long EMA optional output is empty for stock {}",
                        stock_symbol
                    );
                }

                // Get C Tulip EMA output for comparison
                let ema_inputs_c = [close.as_ptr()];
                let ema_options = [options[1]]; // long_period
                let ema_start_index = unsafe { ti_ema_start(ema_options.as_ptr()) };
                let ema_output_len = close.len() - (ema_start_index as usize);
                let mut c_long_ema = vec![0.0; ema_output_len];
                let mut ema_outputs_c = vec![c_long_ema.as_mut_ptr()];

                let ret = unsafe {
                    ti_ema(
                        close.len() as i32,
                        ema_inputs_c.as_ptr(),
                        ema_options.as_ptr(),
                        ema_outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(
                    ret, 0,
                    "ti_ema returned error code {} for stock {}",
                    ret, stock_symbol
                );

                // Compare from the end backwards
                let compare_len = rust_long_ema.len().min(c_long_ema.len());
                for i in 0..compare_len {
                    let rust_idx = rust_long_ema.len() - 1 - i;
                    let c_idx = c_long_ema.len() - 1 - i;
                    let rust_val = rust_long_ema[rust_idx];
                    let c_val = c_long_ema[c_idx];

                    if !rust_val.is_finite() {
                        panic!(
                            "Rust long EMA output has NaN/infinity at index {} (from end): Rust = {} for stock {} options {:?}",
                            i, rust_val, stock_symbol, options
                        );
                    }

                    if !c_val.is_finite() {
                        continue; // Skip C library bugs
                    }

                    assert!(
                        approx_eq!(f64, c_val, rust_val, epsilon = EMA_EPSILON),
                        "Long EMA mismatch at index {} (from end): C = {}, Rust = {} for stock {} options {:?}",
                        i,
                        c_val,
                        rust_val,
                        stock_symbol,
                        options
                    );
                }

                println!(
                    "✓ Long EMA optional output validated for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All APO long EMA optional output database tests passed!");
    }
    #[test]
    fn test_apo_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::apo::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD APO 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    indicator(&inputs, options, None).expect("Regular APO indicator failed");

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
                            "SIMD APO has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD APO has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        println!("SIMD: {:?}", simd_result);
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular APO database tests passed!");
    }

    /*#[test]
    fn test_apo_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::nightly::apo_simd::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Optional outputs: [short_ema, long_ema]
        let optional_outputs = &[true, true];

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let inputs = [close.as_slice()];

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(optional_outputs))
                    .expect("SIMD APO 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) = indicator(&inputs, options, Some(optional_outputs))
                    .expect("Regular APO indicator failed");

                let simd_result = &all_simd_results[idx];
                let regular_result = &regular_results;

                // Should have 3 outputs: [apo, short_ema, long_ema]
                assert_eq!(simd_result.len(), 3, "SIMD result should have 3 outputs");
                assert_eq!(
                    regular_result.len(),
                    3,
                    "Regular result should have 3 outputs"
                );

                // Compare all outputs
                let output_names = ["APO", "Short EMA", "Long EMA"];
                let epsilons = [EPSILON, EMA_EPSILON, EMA_EPSILON];

                for (output_idx, (output_name, epsilon)) in
                    output_names.iter().zip(epsilons.iter()).enumerate()
                {
                    let simd_output = &simd_result[output_idx];
                    let regular_output = &regular_result[output_idx];

                    // Compare output lengths
                    assert_eq!(
                        simd_output.len(),
                        regular_output.len(),
                        "{} output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                        output_name,
                        stock_symbol,
                        options,
                        simd_output.len(),
                        regular_output.len()
                    );

                    // Compare each value
                    for (i, (&simd_val, &regular_val)) in
                        simd_output.iter().zip(regular_output.iter()).enumerate()
                    {
                        // Check for NaN/infinity in SIMD result
                        if simd_val.is_nan() {
                            panic!(
                                "SIMD {} has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                                output_name, i, stock_symbol, simd_val, options
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD {} has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                                output_name, i, stock_symbol, simd_val, options
                            );
                        }

                        // Compare values with tolerance
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = *epsilon) {
                            panic!(
                                "{} mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                                output_name, i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular APO optional outputs database tests passed!");
    }*/
}
