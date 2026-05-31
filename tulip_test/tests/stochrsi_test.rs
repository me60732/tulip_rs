#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::stochrsi::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_rsi, ti_rsi_start, ti_stochrsi, ti_stochrsi_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-10;
    const CHUNK_SIZE: usize = 100;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    //const OPTIONS_LIST: [[f64; 1]; 5] = [[5.0], [10.0], [14.0], [20.0], [25.0]];
    const OPTIONS_LIST: [[f64; 1]; 10] = [
        [5.0],
        [7.0],
        [8.0],
        [10.0],
        [14.0],
        [20.0],
        [25.0],
        [35.0],
        [50.0],
        [100.0],
    ];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_close() -> Vec<f64> {
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..15 {
            close_vec.extend_from_slice(&CLOSE);
        }
        close_vec
    }

    #[test]
    fn test_stochrsi_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C STOCHRSI function
            let start_index = unsafe { ti_stochrsi_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_stochrsi_start returned a negative index"
            );
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut stochrsi_output_vec_c = vec![0.0_f64; output_len_c];
            let stochrsi_ptr: *mut f64 = stochrsi_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![stochrsi_ptr];
            let ret = unsafe {
                ti_stochrsi(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_stochrsi returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Rust STOCHRSI indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in stochrsi_output_vec_c
                .iter()
                .rev()
                .take(output_len_rust - 1)
                .zip(outputs[0].iter().rev())
                .enumerate()
            {
                let index = output_len_rust - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust STOCHRSI has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust STOCHRSI has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val * 100.0, rust_val, epsilon = EPSILON) {
                    // Adjust epsilon if needed
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, stochrsi_output_vec_c, outputs[0], options
                    );
                    panic!(
                        "Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index,
                        c_val * 100.0,
                        rust_val,
                        options
                    );
                }
            }
        }
    }

    #[test]
    fn test_stochrsi_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // run c code
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                // Determine the offset required by the C MIN function
                let start_index = unsafe { ti_stochrsi_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_min_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                // Run the C implementation
                let mut stochrsi_output_vec_c = vec![0.0_f64; output_len_c];
                let stochrsi_ptr: *mut f64 = stochrsi_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![stochrsi_ptr];
                let ret = unsafe {
                    ti_stochrsi(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_min returned error code {}", ret);

                let inputs_rust = [close.as_slice()];
                let (outputs, _) = indicator(&inputs_rust, &options, None)
                    .expect("Rust StochRSI indicator failed");
                let rust_output = outputs[0].clone();
                /* let inputs = [&close[200..].to_vec()];
                let rust_result = indicator_from_state(&inputs, &options, &result_rust.state, None)
                    .expect("Rust StochRSI indicator failed");
                rust_output.extend_from_slice(&rust_result.indicators[0]);*/
                let output_len_rust = rust_output.len();

                for (i, (&c_val, &rust_val)) in stochrsi_output_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust - 1)
                    .zip(rust_output.iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() {
                        panic!(
                            "Rust STOCHRSI has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust STOCHRSI has infinity at index {}: Rust = {}, Options = {:?}",
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

                    if !approx_eq!(f64, c_val * 100.0, rust_val, epsilon = EPSILON) {
                        // Adjust epsilon if needed
                        println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, stochrsi_output_vec_c, outputs[0], options, stock_symbol
                        );
                        panic!(
                            "Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index,
                            c_val * 100.0,
                            rust_val,
                            options
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn test_stochrsi_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) = indicator(&inputs_rust, &options, None)
                    .expect("Rust StochRSI indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                    .expect("Rust StochRSI indicator failed");
                for output_idx in 0..first_outputs.len() {
                    batch_full_outputs[output_idx].extend_from_slice(&first_outputs[output_idx]);
                }

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("StochRSI batch indicator failed");
                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("StochRSI batch indicator failed");
                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                // Compare all outputs
                for output_idx in 0..full_outputs.len() {
                    assert_eq!(
                        full_outputs[output_idx].len(),
                        batch_full_outputs[output_idx].len(),
                        "Output {} lengths don't match for stock: {}, options: {:?}",
                        output_idx,
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
                            "State handover mismatch at index {} for output {} for stock {} with options {:?}: full = {}, batch = {}",
                            i, output_idx, stock_symbol, options, full_val, batch_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_stochrsi_rsi_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Get Rust StochRSI with RSI optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) = indicator(&inputs_rust, &options, Some(&[true]))
                .expect("Rust StochRSI indicator failed");

            assert!(!outputs.is_empty(), "StochRSI outputs should not be empty");
            assert!(
                outputs.len() >= 2,
                "StochRSI should have at least 2 outputs when optional outputs enabled"
            );

            let rust_rsi_output = &outputs[1]; // RSI is at index 1

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_rsi_output.is_empty(),
                "RSI optional output vector should not be empty"
            );

            // Get C RSI reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_rsi_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_rsi_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut rsi_output_vec_c = vec![0.0_f64; output_len_c];
            let rsi_ptr: *mut f64 = rsi_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![rsi_ptr];
            let ret = unsafe {
                ti_rsi(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_rsi returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in rsi_output_vec_c
                .iter()
                .rev()
                .zip(rust_rsi_output.iter().rev())
                .enumerate()
            {
                let index = rust_rsi_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust RSI optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust RSI optional output has infinity at index {}: Rust = {}, Options = {:?}",
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
                    panic!(
                        "RSI optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    #[test]
    fn test_stochrsi_database_optional_rsi() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get STOCHRSI with RSI optional output
                let optional_outputs = Some(&[true][..]);
                let (stochrsi_result, _) = tulip_rs::indicators::stochrsi::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_rsi = &stochrsi_result[1];

                // Calculate expected RSI using C Tulip ti_rsi
                let rsi_options = [options[0]];
                let start_index = unsafe { ti_rsi_start(rsi_options.as_ptr()) };
                assert!(start_index >= 0, "ti_rsi_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_rsi_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_rsi_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_rsi(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        rsi_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_rsi failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_rsi.len().min(c_rsi_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_rsi.len() - 1 - i;
                    let c_idx = c_rsi_output.len() - 1 - i;

                    let rust_val = rust_rsi[rust_idx];
                    let c_val = c_rsi_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust RSI output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "STOCHRSI RSI optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_stochrsi_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::stochrsi::indicator_by_assets;

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
                    .expect("SIMD by assets STOCHRSI indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = indicator(&stock_inputs, &options, None)
                        .expect("Regular STOCHRSI indicator failed");

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
                                "SIMD by assets STOCHRSI has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets STOCHRSI has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with epsilon tolerance for STOCHRSI
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

        println!("✓ All SIMD by assets vs Regular STOCHRSI database tests passed!");
    }

    #[test]
    fn test_stochrsi_simd_by_assets_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::stochrsi::indicator_by_assets;

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
            // Test with optional outputs enabled (RSI)
            {
                // Get SIMD by assets result with optional outputs
                let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, Some(&[true]))
                    .expect("SIMD by assets STOCHRSI indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = indicator(&stock_inputs, &options, Some(&[true]))
                        .expect("Regular STOCHRSI indicator failed");

                    let simd_stochrsi_result = &simd_results[stock_idx][0];
                    let simd_rsi_result = &simd_results[stock_idx][1];
                    let regular_stochrsi_result = &regular_results[0];
                    let regular_rsi_result = &regular_results[1];

                    // Compare STOCHRSI output lengths
                    assert_eq!(
                        simd_stochrsi_result.len(),
                        regular_stochrsi_result.len(),
                        "STOCHRSI output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                        stock_symbol,
                        options,
                        simd_stochrsi_result.len(),
                        regular_stochrsi_result.len()
                    );

                    // Compare RSI output lengths
                    assert_eq!(
                        simd_rsi_result.len(),
                        regular_rsi_result.len(),
                        "RSI output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                        stock_symbol,
                        options,
                        simd_rsi_result.len(),
                        regular_rsi_result.len()
                    );

                    // Compare STOCHRSI values
                    for (i, (&simd_val, &regular_val)) in simd_stochrsi_result
                        .iter()
                        .zip(regular_stochrsi_result.iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "STOCHRSI mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    // Compare RSI values
                    for (i, (&simd_val, &regular_val)) in simd_rsi_result
                        .iter()
                        .zip(regular_rsi_result.iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "RSI mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    println!(
                        "✓ SIMD by assets vs Regular optional outputs test passed for stock {} with options {:?}",
                        stock_symbol, options
                    );
                }
            }
        }

        println!(
            "✓ All SIMD by assets vs Regular STOCHRSI optional outputs database tests passed!"
        );
    }

    #[test]
    fn test_stochrsi_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::stochrsi::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4_first = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4_first, _) =
                indicator_by_options::<4>(&inputs, &options_4_first, None)
                    .expect("SIMD STOCHRSI 4-wide first failed");

            // Process second 4 options with 4-wide SIMD
            let options_4_second = [
                &OPTIONS_LIST[4],
                &OPTIONS_LIST[5],
                &OPTIONS_LIST[6],
                &OPTIONS_LIST[7],
            ];
            let (simd_results_4_second, _) =
                indicator_by_options::<4>(&inputs, &options_4_second, None)
                    .expect("SIMD STOCHRSI 4-wide second failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[8], &OPTIONS_LIST[9]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD STOCHRSI 2-wide failed");

            // Combine all SIMD results
            let mut all_simd_results = simd_results_4_first;
            all_simd_results.extend(simd_results_4_second);
            all_simd_results.extend(simd_results_2);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    indicator(&inputs, options, None).expect("Regular STOCHRSI indicator failed");

                let simd_result = &all_simd_results[idx];
                let regular_result = &regular_results;

                // Compare output lengths for STOCHRSI
                assert_eq!(
                    simd_result[0].len(),
                    regular_result[0].len(),
                    "STOCHRSI output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[0].len(),
                    regular_result[0].len()
                );

                // Compare STOCHRSI values
                for (i, (&simd_val, &regular_val)) in simd_result[0]
                    .iter()
                    .zip(regular_result[0].iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD STOCHRSI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD STOCHRSI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "STOCHRSI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular STOCHRSI database tests passed!");
    }

    #[test]
    fn test_stochrsi_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::stochrsi::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4_first = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4_first, _) =
                indicator_by_options::<4>(&inputs, &options_4_first, Some(&[true]))
                    .expect("SIMD STOCHRSI 4-wide first failed");

            // Process second 4 options with 4-wide SIMD
            let options_4_second = [
                &OPTIONS_LIST[4],
                &OPTIONS_LIST[5],
                &OPTIONS_LIST[6],
                &OPTIONS_LIST[7],
            ];
            let (simd_results_4_second, _) =
                indicator_by_options::<4>(&inputs, &options_4_second, Some(&[true]))
                    .expect("SIMD STOCHRSI 4-wide second failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[8], &OPTIONS_LIST[9]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, Some(&[true]))
                .expect("SIMD STOCHRSI 2-wide failed");

            // Combine all SIMD results
            let mut all_simd_results = simd_results_4_first;
            all_simd_results.extend(simd_results_4_second);
            all_simd_results.extend(simd_results_2);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = indicator(&inputs, options, Some(&[true]))
                    .expect("Regular STOCHRSI indicator failed");

                let simd_result = &all_simd_results[idx];
                let regular_result = &regular_results;

                // Compare STOCHRSI output lengths
                assert_eq!(
                    simd_result[0].len(),
                    regular_result[0].len(),
                    "STOCHRSI output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[0].len(),
                    regular_result[0].len()
                );

                // Compare RSI output lengths
                assert_eq!(
                    simd_result[1].len(),
                    regular_result[1].len(),
                    "RSI output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[1].len(),
                    regular_result[1].len()
                );

                // Compare STOCHRSI values
                for (i, (&simd_val, &regular_val)) in simd_result[0]
                    .iter()
                    .zip(regular_result[0].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        /*let start = if i < 10 { 0 } else { i - 10 };
                        println!(
                            "simd stochrsi: {:?} \n\nRegular rsi Results: {:?} \n\n",
                            &simd_result[0][start..(i+10).min(simd_result[0].len())],
                            &regular_result[0][start..(i+10).min(regular_result[0].len())]
                        );*/
                        panic!(
                            "STOCHRSI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare RSI values
                for (i, (&simd_val, &regular_val)) in simd_result[1]
                    .iter()
                    .zip(regular_result[1].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "RSI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular STOCHRSI optional outputs database tests passed!"
        );
    }

    // add test code here
}
