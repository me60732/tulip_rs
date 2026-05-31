#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::vidya::{indicator as rust_vidya, min_data, TIndicatorState};
    use tulip_test::c_bindings::{
        ti_sma, ti_sma_start, ti_stddev, ti_stddev_start, ti_vidya, ti_vidya_start,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-5;
    const SMA_EPSILON: f64 = 1e-10;
    const STDDEV_EPSILON: f64 = 1e-4;
    const CHUNK_SIZE: usize = 100;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 3]; 6] = [
        [2.0, 5.0, 0.2],
        [5.0, 9.0, 0.2],
        [9.0, 12.0, 0.2],
        [12.0, 20.0, 0.2],
        [20.0, 50.0, 0.2],
        [14.0, 30.0, 0.2],
    ];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_close() -> Vec<f64> {
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..10 {
            close_vec.extend_from_slice(&CLOSE);
        }
        close_vec
    }

    #[test]
    fn test_vidya_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C VIDYA function
            let start_index = unsafe { ti_vidya_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_vidya_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut vidya_output_vec_c = vec![0.0_f64; output_len_c];
            let vidya_ptr: *mut f64 = vidya_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![vidya_ptr];
            let ret = unsafe {
                ti_vidya(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_vidya returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_vidya(&inputs_rust, &options, None).expect("Rust VIDYA indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in vidya_output_vec_c
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
                        "Rust VIDYA has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust VIDYA has infinity at index {}: Rust = {}, Options = {:?}",
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
                    /*println!(
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}",
                        index, vidya_output_vec_c, outputs[0], options
                    );*/
                    panic!(
                        "Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_vidya_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_vidya_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_vidya_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_vidya(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_vidya returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_vidya(&inputs_rust, &options, None).expect("Rust VIDYA indicator failed");

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
                            "Rust VIDYA has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust VIDYA has infinity at index {}: Rust = {}, Options = {:?}",
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
                        /*println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, output_vec_c, outputs[0], options, stock_symbol
                        );*/
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
    fn test_vidya_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_vidya(&inputs_rust, &options, None).expect("Rust VIDYA indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) =
                    rust_vidya(&chunk_inputs, &options, None).expect("Rust VIDYA indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VIDYA batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VIDYA batch indicator failed");
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

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    #[test]
    fn test_vidya_simd_vs_regular_database() {
        use tulip_rs::indicators::vidya::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let close = get_close_array(data);
                (symbol.clone(), close)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[stock_data[0].1.as_slice()],
            &[stock_data[1].1.as_slice()],
            &[stock_data[2].1.as_slice()],
            &[stock_data[3].1.as_slice()],
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets VIDYA indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) = rust_vidya(&stock_inputs, &options, None)
                    .expect("Regular VIDYA indicator failed");

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
                            "SIMD by assets VIDYA has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets VIDYA has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for VIDYA
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        print!(
                            "SIMD: {:?}\nRegular: {:?}",
                            &simd_result[..20],
                            &regular_result[..20]
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

        println!("✓ All SIMD by assets vs Regular VIDYA database tests passed!");
    }

    #[test]
    fn test_vidya_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::vidya::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let close = get_close_array(data);
                (symbol.clone(), close)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[stock_data[0].1.as_slice()],
            &[stock_data[1].1.as_slice()],
            &[stock_data[2].1.as_slice()],
            &[stock_data[3].1.as_slice()],
        ];

        for options in OPTIONS_LIST {
            let optional_flags = [true, true, true, true]; // All 4 optional outputs

            // Get SIMD by assets result with optional outputs
            let (simd_results, _) =
                indicator_by_assets::<4>(&inputs, &options, Some(&optional_flags))
                    .expect("SIMD by assets VIDYA indicator with optional outputs failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock with optional outputs
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) =
                    rust_vidya(&stock_inputs, &options, Some(&optional_flags))
                        .expect("Regular VIDYA indicator with optional outputs failed");

                // Compare all outputs (main + optional)
                assert_eq!(
                    simd_results[stock_idx].len(),
                    regular_results.len(),
                    "Number of outputs mismatch for stock {} with options {:?}",
                    stock_symbol,
                    options
                );

                for (output_idx, (simd_output, regular_output)) in simd_results[stock_idx]
                    .iter()
                    .zip(regular_results.iter())
                    .enumerate()
                {
                    // Compare output lengths
                    assert_eq!(
                        simd_output.len(),
                        regular_output.len(),
                        "Output {} length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                        output_idx,
                        stock_symbol,
                        options,
                        simd_output.len(),
                        regular_output.len()
                    );

                    // Compare each value in this output
                    for (i, (&simd_val, &regular_val)) in
                        simd_output.iter().zip(regular_output.iter()).enumerate()
                    {
                        // Check for NaN/infinity in SIMD result
                        if simd_val.is_nan() {
                            panic!(
                                "SIMD by assets VIDYA has NaN in output {} at index {} for stock {} with options {:?}: SIMD = {}",
                                output_idx, i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets VIDYA has infinity in output {} at index {} for stock {} with options {:?}: SIMD = {}",
                                output_idx, i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for VIDYA
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            print!(
                                "SIMD: {:?}\nRegular: {:?}",
                                &simd_output[..20],
                                &regular_output[..20]
                            );
                            panic!(
                                "Mismatch in output {} at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                output_idx, i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }
                }

                println!(
                    "✓ SIMD by assets vs Regular test with optional outputs passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!(
            "✓ All SIMD by assets vs Regular VIDYA database tests with optional outputs passed!"
        );
    }
    #[test]
    fn test_vidya_short_sma_optional_output_vs_c_tulip() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            let short_period_f64 = [options[0]];

            // Get Rust VIDYA with short_sma optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_vidya(&inputs_rust, &options, Some(&[true, false, false, false]))
                    .expect("Rust VIDYA indicator failed");

            assert!(!outputs.is_empty(), "VIDYA outputs should not be empty");
            assert!(
                outputs.len() >= 2,
                "VIDYA should have at least 2 outputs when optional outputs enabled"
            );

            let rust_short_sma_output = &outputs[1]; // short_sma is at index 1

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_short_sma_output.is_empty(),
                "short_sma optional output vector should not be empty"
            );

            // Get C SMA reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_sma_start(short_period_f64.as_ptr()) };
            assert!(start_index >= 0, "ti_sma_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut sma_output_vec_c = vec![0.0_f64; output_len_c];
            let sma_ptr: *mut f64 = sma_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![sma_ptr];
            let ret = unsafe {
                ti_sma(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    short_period_f64.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_sma returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in sma_output_vec_c
                .iter()
                .rev()
                .zip(rust_short_sma_output.iter().rev())
                .enumerate()
            {
                let index = rust_short_sma_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust short_sma optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust short_sma optional output has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = SMA_EPSILON) {
                    panic!(
                        "short_sma optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_vidya_long_sma_optional_output_vs_c_tulip() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            let long_period_f64 = [options[1]];

            // Get Rust VIDYA with long_sma optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_vidya(&inputs_rust, &options, Some(&[false, true, false, false]))
                    .expect("Rust VIDYA indicator failed");

            assert!(!outputs.is_empty(), "VIDYA outputs should not be empty");
            assert!(
                outputs.len() >= 3,
                "VIDYA should have at least 3 outputs when optional outputs enabled"
            );

            let rust_long_sma_output = &outputs[2]; // long_sma is at index 2

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_long_sma_output.is_empty(),
                "long_sma optional output vector should not be empty"
            );

            // Get C SMA reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_sma_start(long_period_f64.as_ptr()) };
            assert!(start_index >= 0, "ti_sma_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut sma_output_vec_c = vec![0.0_f64; output_len_c];
            let sma_ptr: *mut f64 = sma_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![sma_ptr];
            let ret = unsafe {
                ti_sma(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    long_period_f64.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_sma returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in sma_output_vec_c
                .iter()
                .rev()
                .zip(rust_long_sma_output.iter().rev())
                .enumerate()
            {
                let index = rust_long_sma_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust long_sma optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust long_sma optional output has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = SMA_EPSILON) {
                    panic!(
                        "long_sma optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_vidya_short_stddev_optional_output_vs_c_tulip() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            let short_period_f64 = [options[0]];

            // Get Rust VIDYA with short_stddev optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_vidya(&inputs_rust, &options, Some(&[false, false, true, false]))
                    .expect("Rust VIDYA indicator failed");

            assert!(!outputs.is_empty(), "VIDYA outputs should not be empty");
            assert!(
                outputs.len() >= 4,
                "VIDYA should have at least 4 outputs when optional outputs enabled"
            );

            let rust_short_stddev_output = &outputs[3]; // short_stddev is at index 3

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_short_stddev_output.is_empty(),
                "short_stddev optional output vector should not be empty"
            );

            // Get C STDDEV reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_stddev_start(short_period_f64.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_stddev_start returned a negative index"
            );
            let output_len_c = close.len() - (start_index as usize);

            let mut stddev_output_vec_c = vec![0.0_f64; output_len_c];
            let stddev_ptr: *mut f64 = stddev_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![stddev_ptr];
            let ret = unsafe {
                ti_stddev(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    short_period_f64.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_stddev returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in stddev_output_vec_c
                .iter()
                .rev()
                .zip(rust_short_stddev_output.iter().rev())
                .enumerate()
            {
                let index = rust_short_stddev_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust short_stddev optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust short_stddev optional output has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = STDDEV_EPSILON) {
                    panic!(
                        "short_stddev optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_vidya_long_stddev_optional_output_vs_c_tulip() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            let long_period_f64 = [options[1]];

            // Get Rust VIDYA with long_stddev optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_vidya(&inputs_rust, &options, Some(&[false, false, false, true]))
                    .expect("Rust VIDYA indicator failed");

            assert!(!outputs.is_empty(), "VIDYA outputs should not be empty");
            assert!(
                outputs.len() >= 5,
                "VIDYA should have at least 5 outputs when optional outputs enabled"
            );

            let rust_long_stddev_output = &outputs[4]; // long_stddev is at index 4

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_long_stddev_output.is_empty(),
                "long_stddev optional output vector should not be empty"
            );

            // Get C STDDEV reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_stddev_start(long_period_f64.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_stddev_start returned a negative index"
            );
            let output_len_c = close.len() - (start_index as usize);

            let mut stddev_output_vec_c = vec![0.0_f64; output_len_c];
            let stddev_ptr: *mut f64 = stddev_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![stddev_ptr];
            let ret = unsafe {
                ti_stddev(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    long_period_f64.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_stddev returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in stddev_output_vec_c
                .iter()
                .rev()
                .zip(rust_long_stddev_output.iter().rev())
                .enumerate()
            {
                let index = rust_long_stddev_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust long_stddev optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust long_stddev optional output has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = STDDEV_EPSILON) {
                    panic!(
                        "long_stddev optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_vidya_database_optional_short_sma() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get VIDYA with short_sma optional output
                let optional_outputs = Some(&[true, false, false, false][..]);
                let (vidya_result, _) = tulip_rs::indicators::vidya::indicator(
                    &[&close],
                    &[options[0], options[1], options[2]],
                    optional_outputs,
                )
                .unwrap();

                let rust_short_sma = &vidya_result[1];

                // Calculate expected short SMA using C Tulip ti_sma
                let short_sma_options = [options[0]]; // short period
                let start_index = unsafe { ti_sma_start(short_sma_options.as_ptr()) };
                assert!(start_index >= 0, "ti_sma_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_short_sma_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_short_sma_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_sma(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        short_sma_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_sma failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_short_sma.len().min(c_short_sma_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_short_sma.len() - 1 - i;
                    let c_idx = c_short_sma_output.len() - 1 - i;

                    let rust_val = rust_short_sma[rust_idx];
                    let c_val = c_short_sma_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust short SMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = SMA_EPSILON),
                        "VIDYA short SMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_vidya_database_optional_long_sma() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get VIDYA with long_sma optional output
                let optional_outputs = Some(&[false, true, false, false][..]);
                let (vidya_result, _) = tulip_rs::indicators::vidya::indicator(
                    &[&close],
                    &[options[0], options[1], options[2]],
                    optional_outputs,
                )
                .unwrap();

                let rust_long_sma = &vidya_result[2];

                // Calculate expected long SMA using C Tulip ti_sma
                let long_sma_options = [options[1]]; // long period
                let start_index = unsafe { ti_sma_start(long_sma_options.as_ptr()) };
                assert!(start_index >= 0, "ti_sma_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_long_sma_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_long_sma_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_sma(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        long_sma_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_sma failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_long_sma.len().min(c_long_sma_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_long_sma.len() - 1 - i;
                    let c_idx = c_long_sma_output.len() - 1 - i;

                    let rust_val = rust_long_sma[rust_idx];
                    let c_val = c_long_sma_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust long SMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = SMA_EPSILON),
                        "VIDYA long SMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_vidya_database_optional_short_stddev() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get VIDYA with short_stddev optional output
                let optional_outputs = Some(&[false, false, true, false][..]);
                let (vidya_result, _) = tulip_rs::indicators::vidya::indicator(
                    &[&close],
                    &[options[0], options[1], options[2]],
                    optional_outputs,
                )
                .unwrap();

                let rust_short_stddev = &vidya_result[3];

                // Calculate expected short stddev using C Tulip ti_stddev
                let short_stddev_options = [options[0]]; // short period
                let start_index = unsafe { ti_stddev_start(short_stddev_options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_stddev_start returned a negative index"
                );
                let output_len_c = close.len() - (start_index as usize);

                let mut c_short_stddev_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_short_stddev_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_stddev(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        short_stddev_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_stddev failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_short_stddev.len().min(c_short_stddev_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_short_stddev.len() - 1 - i;
                    let c_idx = c_short_stddev_output.len() - 1 - i;

                    let rust_val = rust_short_stddev[rust_idx];
                    let c_val = c_short_stddev_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust short stddev output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = STDDEV_EPSILON),
                        "VIDYA short stddev optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_vidya_database_optional_long_stddev() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get VIDYA with long_stddev optional output
                let optional_outputs = Some(&[false, false, false, true][..]);
                let (vidya_result, _) = tulip_rs::indicators::vidya::indicator(
                    &[&close],
                    &[options[0], options[1], options[2]],
                    optional_outputs,
                )
                .unwrap();

                let rust_long_stddev = &vidya_result[4];

                // Calculate expected long stddev using C Tulip ti_stddev
                let long_stddev_options = [options[1]]; // long period
                let start_index = unsafe { ti_stddev_start(long_stddev_options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_stddev_start returned a negative index"
                );
                let output_len_c = close.len() - (start_index as usize);

                let mut c_long_stddev_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_long_stddev_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_stddev(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        long_stddev_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_stddev failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_long_stddev.len().min(c_long_stddev_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_long_stddev.len() - 1 - i;
                    let c_idx = c_long_stddev_output.len() - 1 - i;

                    let rust_val = rust_long_stddev[rust_idx];
                    let c_val = c_long_stddev_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust long stddev output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = STDDEV_EPSILON),
                        "VIDYA long stddev optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_vidya_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::vidya::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD VIDYA 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD VIDYA 2-wide failed");

            // Combine SIMD results in the same order as OPTIONS_LIST
            let mut all_simd_results = Vec::with_capacity(OPTIONS_LIST.len());
            for res in simd_results_4.into_iter() {
                all_simd_results.push(res);
            }
            for res in simd_results_2.into_iter() {
                all_simd_results.push(res);
            }

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_vidya(&inputs, options, None).expect("Regular VIDYA indicator failed");

                // main VIDYA output
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
                            "SIMD VIDYA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD VIDYA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
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

        println!("✓ All SIMD by options vs Regular VIDYA database tests passed!");
    }

    #[test]
    fn test_vidya_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::vidya::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Request all optional outputs: short_sma, long_sma, short_stddev, long_stddev
            let optional_outputs = Some(&[true, true, true, true][..]);

            // Process first 4 options with 4-wide SIMD (with optional outputs)
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD VIDYA 4-wide with optional outputs failed");

            // Process remaining 2 options with 2-wide SIMD (with optional outputs)
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) =
                indicator_by_options::<2>(&inputs, &options_2, optional_outputs)
                    .expect("SIMD VIDYA 2-wide with optional outputs failed");

            // Combine SIMD results in the same order as OPTIONS_LIST
            let mut all_simd_results = Vec::with_capacity(OPTIONS_LIST.len());
            for res in simd_results_4.into_iter() {
                all_simd_results.push(res);
            }
            for res in simd_results_2.into_iter() {
                all_simd_results.push(res);
            }

            // Compare each SIMD result with regular indicator (with optional outputs)
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_vidya(&inputs, options, optional_outputs)
                    .expect("Regular VIDYA indicator with optional outputs failed");

                // For VIDYA the outputs are: [VIDYA, short_sma, long_sma, short_stddev, long_stddev]
                let simd_vidya_result = &all_simd_results[idx][0];
                let regular_vidya_result = &regular_results[0];

                let simd_short_sma = &all_simd_results[idx][1];
                let regular_short_sma = &regular_results[1];

                let simd_long_sma = &all_simd_results[idx][2];
                let regular_long_sma = &regular_results[2];

                let simd_short_stddev = &all_simd_results[idx][3];
                let regular_short_stddev = &regular_results[3];

                let simd_long_stddev = &all_simd_results[idx][4];
                let regular_long_stddev = &regular_results[4];

                // Compare VIDYA output lengths
                assert_eq!(
                    simd_vidya_result.len(),
                    regular_vidya_result.len(),
                    "VIDYA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_vidya_result.len(),
                    regular_vidya_result.len()
                );

                // Compare VIDYA values
                for (i, (&simd_val, &regular_val)) in simd_vidya_result
                    .iter()
                    .zip(regular_vidya_result.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD VIDYA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD VIDYA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "VIDYA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare SMA optional outputs with SMA_EPSILON
                for (i, (&simd_val, &regular_val)) in simd_short_sma
                    .iter()
                    .zip(regular_short_sma.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD short_sma has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD short_sma has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = SMA_EPSILON) {
                        panic!(
                            "short_sma mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                for (i, (&simd_val, &regular_val)) in simd_long_sma
                    .iter()
                    .zip(regular_long_sma.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD long_sma has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD long_sma has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = SMA_EPSILON) {
                        panic!(
                            "long_sma mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare STDDEV optional outputs with STDDEV_EPSILON
                for (i, (&simd_val, &regular_val)) in simd_short_stddev
                    .iter()
                    .zip(regular_short_stddev.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD short_stddev has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD short_stddev has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = STDDEV_EPSILON) {
                        panic!(
                            "short_stddev mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                for (i, (&simd_val, &regular_val)) in simd_long_stddev
                    .iter()
                    .zip(regular_long_stddev.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD long_stddev has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD long_stddev has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = STDDEV_EPSILON) {
                        panic!(
                            "long_stddev mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
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

        println!("✓ All SIMD by options vs Regular VIDYA optional outputs database tests passed!");
    }
}
