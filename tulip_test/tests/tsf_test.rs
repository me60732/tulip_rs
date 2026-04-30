#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::tsf::{indicator as rust_tsf, min_data, TIndicatorState};
    use tulip_test::c_bindings::{
        ti_linreg, ti_linreg_start, ti_linregintercept, ti_linregintercept_start, ti_linregslope,
        ti_linregslope_start, ti_tsf, ti_tsf_start,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-10;
    const CHUNK_SIZE: usize = 100;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    // Match LINREG's options list (4 options) so SIMD-by-options tests can use 4-wide lanes.
    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [25.0]];

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
    fn test_tsf_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C TSF function
            let start_index = unsafe { ti_tsf_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_tsf_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut tsf_output_vec_c = vec![0.0_f64; output_len_c];
            let tsf_ptr: *mut f64 = tsf_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![tsf_ptr];
            let ret = unsafe {
                ti_tsf(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_tsf returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_tsf(&inputs_rust, &options, None).expect("Rust TSF indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in tsf_output_vec_c
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
                        "Rust TSF has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust TSF has infinity at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
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
                    // Adjust epsilon if needed
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, tsf_output_vec_c, outputs[0], options
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
    fn test_tsf_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_tsf_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_tsf_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_tsf(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_tsf returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_tsf(&inputs_rust, &options, None).expect("Rust TSF indicator failed");

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
                            "Rust TSF has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust TSF has infinity at index {}: Rust = {}, Options = {:?}",
                            index, rust_val, options
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
    fn test_tsf_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_tsf(&inputs_rust, &options, None).expect("Rust TSF indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) =
                    rust_tsf(&chunk_inputs, &options, None).expect("Rust TSF indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("TSF batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("TSF batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Compare outputs
                assert_eq!(
                    full_outputs[0].len(),
                    batch_full_output.len(),
                    "Output lengths don't match for stock: {}, options: {:?}",
                    stock_symbol,
                    options
                );

                for (i, (&full_val, &batch_val)) in full_outputs[0]
                    .iter()
                    .zip(batch_full_output.iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "State handover mismatch at index {} for stock {} with options {:?}: full = {}, batch = {}",
                        i, stock_symbol, options, full_val, batch_val
                    );
                }
            }
        }
    }

    #[test]
    fn test_tsf_simd_vs_regular_database() {
        use tulip_rs::indicators::tsf::indicator_by_assets;

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
                    .expect("SIMD by assets TSF indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = rust_tsf(&stock_inputs, &options, None)
                        .expect("Regular TSF indicator failed");

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
                                "SIMD by assets TSF has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets TSF has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for TSF
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
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
        }

        println!("✓ All SIMD by assets vs Regular TSF database tests passed!");
    }

    #[test]
    fn test_tsf_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::tsf::indicator_by_assets;

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
                    indicator_by_assets::<4>(&inputs, &options, Some(&[true, true, true]))
                        .expect("SIMD by assets TSF indicator with optional outputs failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results_opt, _) =
                        rust_tsf(&stock_inputs, &options, Some(&[true, true, true]))
                            .expect("Regular TSF indicator with optional outputs failed");

                    // Compare all outputs: TSF, linreg, slope, intercept
                    let output_names = ["TSF", "linreg", "slope", "intercept"];
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

                            // Compare values with appropriate epsilon for TSF
                            if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
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

        println!("✓ All SIMD by assets vs Regular TSF optional outputs database tests passed!");
    }

    // --- SIMD by options tests (new) ---

    #[test]
    fn test_tsf_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::tsf::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

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
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD TSF 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_tsf(&inputs, options, None).expect("Regular TSF indicator failed");

                let simd_result = &all_simd_results[idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "TSF output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result.len(),
                    regular_result.len()
                );

                // Compare values
                for (i, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by options TSF has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options TSF has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with appropriate epsilon for TSF
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        println!(
                            "SIMD: {:?}\n\nRegular: {:?}",
                            &simd_result[..20.min(simd_result.len())],
                            &regular_result[..20.min(regular_result.len())]
                        );
                        panic!(
                            "TSF mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
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

        println!("✓ All SIMD by options vs Regular TSF database tests passed!");
    }

    #[test]
    fn test_tsf_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::tsf::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let inputs = [close.as_slice()];

            // Test with all optional outputs: linreg, slope, intercept
            let optional_outputs = Some(&[true, true, true][..]);

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD TSF 4-wide with optional outputs failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_tsf(&inputs, options, optional_outputs)
                    .expect("Regular TSF indicator with optional outputs failed");

                let simd_tsf_result = &all_simd_results[idx][0];
                let regular_tsf_result = &regular_results[0];

                let simd_linreg_result = &all_simd_results[idx][1];
                let regular_linreg_result = &regular_results[1];

                let simd_slope_result = &all_simd_results[idx][2];
                let regular_slope_result = &regular_results[2];

                let simd_intercept_result = &all_simd_results[idx][3];
                let regular_intercept_result = &regular_results[3];

                // Compare TSF output lengths
                assert_eq!(
                    simd_tsf_result.len(),
                    regular_tsf_result.len(),
                    "TSF output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_tsf_result.len(),
                    regular_tsf_result.len()
                );

                // Compare linreg output lengths
                assert_eq!(
                    simd_linreg_result.len(),
                    regular_linreg_result.len(),
                    "linreg output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_linreg_result.len(),
                    regular_linreg_result.len()
                );

                // Compare slope output lengths
                assert_eq!(
                    simd_slope_result.len(),
                    regular_slope_result.len(),
                    "slope output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_slope_result.len(),
                    regular_slope_result.len()
                );

                // Compare intercept output lengths
                assert_eq!(
                    simd_intercept_result.len(),
                    regular_intercept_result.len(),
                    "intercept output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_intercept_result.len(),
                    regular_intercept_result.len()
                );

                // Compare TSF values
                for (i, (&simd_val, &regular_val)) in simd_tsf_result
                    .iter()
                    .zip(regular_tsf_result.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by options TSF has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options TSF has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "TSF mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare linreg values
                for (i, (&simd_val, &regular_val)) in simd_linreg_result
                    .iter()
                    .zip(regular_linreg_result.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by options linreg has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options linreg has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "linreg mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare slope values
                for (i, (&simd_val, &regular_val)) in simd_slope_result
                    .iter()
                    .zip(regular_slope_result.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by options slope has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options slope has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "slope mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare intercept values
                for (i, (&simd_val, &regular_val)) in simd_intercept_result
                    .iter()
                    .zip(regular_intercept_result.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by options intercept has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options intercept has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "intercept mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }

            println!(
                "✓ SIMD by options vs Regular optional outputs test passed for stock {}",
                stock_symbol
            );
        }

        println!("✓ All SIMD by options vs Regular TSF optional outputs database tests passed!");
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    #[test]
    fn test_tsf_linreg_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let close = CLOSE.to_vec();
        let inputs = [close.as_slice()];
        let options = [5.0]; // period = 5
        let optional_outputs = Some([true, false, false].as_slice()); // Request linreg output

        // Get Rust TSF output with linreg optional output
        let result = rust_tsf(&inputs, &options, optional_outputs).unwrap();
        let rust_linreg = &result.0[1]; // linreg is at index 1

        // Fail fast if Rust output is empty
        if rust_linreg.is_empty() {
            panic!("Rust TSF linreg optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip linreg output for comparison
        let c_inputs: Vec<*const f64> = vec![close.as_ptr()];
        let c_options = [5.0];
        let c_start_index = unsafe { ti_linreg_start(c_options.as_ptr()) } as usize;
        let c_output_len = close.len() - c_start_index;
        let mut c_linreg = vec![0.0; c_output_len];
        let mut c_outputs = vec![c_linreg.as_mut_ptr()];

        let ret = unsafe {
            ti_linreg(
                close.len() as i32,
                c_inputs.as_ptr(),
                c_options.as_ptr(),
                c_outputs.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_linreg returned error code {}", ret);

        // Compare outputs from the end backwards (reverse order comparison)
        println!("Comparing TSF linreg optional output vs C Tulip linreg:");
        println!(
            "Rust linreg length: {}, C linreg length: {}",
            rust_linreg.len(),
            c_linreg.len()
        );

        for (i, (rust_val, c_val)) in rust_linreg
            .iter()
            .rev()
            .zip(c_linreg.iter().rev())
            .enumerate()
        {
            if !rust_val.is_finite() {
                panic!(
                    "Rust linreg output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            if !c_val.is_finite() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > EPSILON {
                println!("\nRUST: {:?}, \n\nC: {:?}", rust_linreg, c_linreg);
                panic!(
                    "TSF linreg mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ TSF linreg optional output matches C Tulip linreg output");
    }

    #[test]
    fn test_tsf_linregslope_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let close = CLOSE.to_vec();
        let inputs = [close.as_slice()];
        let options = [5.0]; // period = 5
        let optional_outputs = Some([false, true, false].as_slice()); // Request slope output

        // Get Rust TSF output with slope optional output
        let result = rust_tsf(&inputs, &options, optional_outputs).unwrap();
        let rust_slope = &result.0[2]; // slope is at index 2

        // Fail fast if Rust output is empty
        if rust_slope.is_empty() {
            panic!("Rust TSF slope optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip linregslope output for comparison
        let c_inputs: Vec<*const f64> = vec![close.as_ptr()];
        let c_options = [5.0];
        let c_start_index = unsafe { ti_linregslope_start(c_options.as_ptr()) } as usize;
        let c_output_len = close.len() - c_start_index;
        let mut c_slope = vec![0.0; c_output_len];
        let mut c_outputs = vec![c_slope.as_mut_ptr()];

        let ret = unsafe {
            ti_linregslope(
                close.len() as i32,
                c_inputs.as_ptr(),
                c_options.as_ptr(),
                c_outputs.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_linregslope returned error code {}", ret);

        // Compare outputs from the end backwards (reverse order comparison)
        println!("Comparing TSF slope optional output vs C Tulip linregslope:");
        println!(
            "Rust slope length: {}, C slope length: {}",
            rust_slope.len(),
            c_slope.len()
        );

        for (i, (rust_val, c_val)) in rust_slope
            .iter()
            .rev()
            .zip(c_slope.iter().rev())
            .enumerate()
        {
            if !rust_val.is_finite() {
                panic!(
                    "Rust slope output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            if !c_val.is_finite() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > EPSILON {
                println!("\nRUST: {:?}, \n\nC: {:?}", rust_slope, c_slope);
                panic!(
                    "TSF slope mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ TSF slope optional output matches C Tulip linregslope output");
    }

    #[test]
    fn test_tsf_linregintercept_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let close = CLOSE.to_vec();
        let inputs = [close.as_slice()];
        let options = [5.0]; // period = 5
        let optional_outputs = Some([false, true, true].as_slice()); // Request both slope and intercept outputs

        // Get Rust TSF output with slope and intercept optional outputs
        let result = rust_tsf(&inputs, &options, optional_outputs).unwrap();
        let rust_slope = &result.0[2]; // slope is at index 2
        let rust_intercept = &result.0[3]; // intercept is at index 3

        // Calculate intercept + slope * 1.0 to match C library's ti_linregintercept behavior
        let rust_linregintercept: Vec<f64> = rust_intercept
            .iter()
            .zip(rust_slope.iter())
            .map(|(intercept, slope)| intercept + slope * 1.0)
            .collect();

        // Fail fast if Rust output is empty
        if rust_linregintercept.is_empty() {
            panic!("Rust TSF calculated linregintercept (intercept + slope * 1.0) is empty - this indicates an indicator bug");
        }

        // Get C Tulip linregintercept output for comparison
        let c_inputs: Vec<*const f64> = vec![close.as_ptr()];
        let c_options = [5.0];
        let c_start_index = unsafe { ti_linregintercept_start(c_options.as_ptr()) } as usize;
        let c_output_len = close.len() - c_start_index;
        let mut c_linregintercept = vec![0.0; c_output_len];
        let mut c_outputs = vec![c_linregintercept.as_mut_ptr()];

        let ret = unsafe {
            ti_linregintercept(
                close.len() as i32,
                c_inputs.as_ptr(),
                c_options.as_ptr(),
                c_outputs.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_linregintercept returned error code {}", ret);

        // Compare outputs from the end backwards (reverse order comparison)
        println!("Comparing TSF calculated linregintercept (intercept + slope * 1.0) vs C Tulip linregintercept:");
        println!(
            "Rust linregintercept length: {}, C linregintercept length: {}",
            rust_linregintercept.len(),
            c_linregintercept.len()
        );

        for (i, (rust_val, c_val)) in rust_linregintercept
            .iter()
            .rev()
            .zip(c_linregintercept.iter().rev())
            .enumerate()
        {
            if !rust_val.is_finite() {
                panic!(
                    "Rust calculated linregintercept (intercept + slope * 1.0) contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            if !c_val.is_finite() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > EPSILON {
                println!(
                    "\nRUST: {:?}, \n\nC: {:?}",
                    rust_linregintercept, c_linregintercept
                );
                panic!(
                    "TSF calculated linregintercept (intercept + slope * 1.0) mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ TSF calculated linregintercept (intercept + slope * 1.0) matches C Tulip linregintercept output");
    }

    #[test]
    fn test_tsf_database_optional_linreg() {
        const EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 20 {
                continue;
            }

            let close = get_close_array(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get TSF with linreg optional output
                let optional_outputs = Some(&[true, false, false][..]);
                let (tsf_result, _) = tulip_rs::indicators::tsf::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_linreg = &tsf_result[1];

                // Calculate expected linreg using C Tulip ti_linreg
                let linreg_options = vec![options[0]]; // period
                let start_index = unsafe { ti_linreg_start(linreg_options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_linreg_start returned a negative index"
                );
                let output_len_c = close.len() - (start_index as usize);

                let mut c_linreg_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_linreg_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_linreg(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        linreg_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_linreg failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_linreg.len().min(c_linreg_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_linreg.len() - 1 - i;
                    let c_idx = c_linreg_output.len() - 1 - i;

                    let rust_val = rust_linreg[rust_idx];
                    let c_val = c_linreg_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust linreg output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "TSF linreg optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
                        rust_idx,
                        options,
                        rust_val,
                        c_val,
                        (rust_val - c_val).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn test_tsf_database_optional_slope() {
        const EPSILON: f64 = 1e-9;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 20 {
                continue;
            }

            let close = get_close_array(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get TSF with slope optional output
                let optional_outputs = Some(&[false, true, false][..]);
                let (tsf_result, _) = tulip_rs::indicators::tsf::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_slope = &tsf_result[2];

                // Calculate expected slope using C Tulip ti_linregslope
                let slope_options = vec![options[0]]; // period
                let start_index = unsafe { ti_linregslope_start(slope_options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_linregslope_start returned a negative index"
                );
                let output_len_c = close.len() - (start_index as usize);

                let mut c_slope_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_slope_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_linregslope(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        slope_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_linregslope failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_slope.len().min(c_slope_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_slope.len() - 1 - i;
                    let c_idx = c_slope_output.len() - 1 - i;

                    let rust_val = rust_slope[rust_idx];
                    let c_val = c_slope_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust slope output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "TSF slope optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
                        rust_idx,
                        options,
                        rust_val,
                        c_val,
                        (rust_val - c_val).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn test_tsf_database_optional_intercept() {
        const EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 20 {
                continue;
            }

            let close = get_close_array(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get TSF with both slope and intercept optional outputs
                let optional_outputs = Some(&[false, true, true][..]);
                let (tsf_result, _) = tulip_rs::indicators::tsf::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_slope = &tsf_result[2];
                let rust_intercept = &tsf_result[3];

                // Calculate intercept + slope * 1.0 to match C library's ti_linregintercept behavior
                let rust_linregintercept: Vec<f64> = rust_intercept
                    .iter()
                    .zip(rust_slope.iter())
                    .map(|(intercept, slope)| intercept + slope * 1.0)
                    .collect();

                // Calculate expected intercept using C Tulip ti_linregintercept
                let intercept_options = vec![options[0]]; // period
                let start_index = unsafe { ti_linregintercept_start(intercept_options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_linregintercept_start returned a negative index"
                );
                let output_len_c = close.len() - (start_index as usize);

                let mut c_intercept_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_intercept_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_linregintercept(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        intercept_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_linregintercept failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_linregintercept.len().min(c_intercept_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_linregintercept.len() - 1 - i;
                    let c_idx = c_intercept_output.len() - 1 - i;

                    let rust_val = rust_linregintercept[rust_idx];
                    let c_val = c_intercept_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust calculated linregintercept (intercept + slope * 1.0) is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "TSF calculated linregintercept (intercept + slope * 1.0) optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
                        rust_idx,
                        options,
                        rust_val,
                        c_val,
                        (rust_val - c_val).abs()
                    );
                }
            }
        }
    }
}
