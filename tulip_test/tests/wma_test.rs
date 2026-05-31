#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::wma::{indicator as rust_wma, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_sma, ti_sma_start, ti_wma, ti_wma_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILON: f64 = 1e-8;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [10.0], [14.0], [20.0], [25.0], [30.0]];

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
    fn test_wma_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C WMA function
            let start_index = unsafe { ti_wma_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_wma_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut wma_output_vec_c = vec![0.0_f64; output_len_c];
            let wma_ptr: *mut f64 = wma_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![wma_ptr];
            let ret = unsafe {
                ti_wma(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_wma returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_wma(&inputs_rust, &options, None).expect("Rust WMA indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in wma_output_vec_c
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
                        "Rust WMA has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust WMA has infinity at index {}: Rust = {}, Options = {:?}",
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
                        index, wma_output_vec_c, outputs[0], options
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
    fn test_wma_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // run c code
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                // Determine the offset required by the C WMA function
                let start_index = unsafe { ti_wma_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_wma_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                // Run the C implementation
                let mut wma_output_vec_c = vec![0.0_f64; output_len_c];
                let wma_ptr: *mut f64 = wma_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![wma_ptr];
                let ret = unsafe {
                    ti_wma(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_wma returned error code {}", ret);

                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_wma(&inputs_rust, &options, None).expect("Rust WMA indicator failed");

                let output_len_rust = outputs[0].len();

                for (i, (&c_val, &rust_val)) in wma_output_vec_c
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
                            "Rust WMA has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust WMA has infinity at index {}: Rust = {}, Options = {:?}",
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
                            index, wma_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_wma_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output
                let (full_outputs, _) =
                    rust_wma(&inputs_rust, &options, None).expect("Rust WMA indicator failed");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if close.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) = rust_wma(&inputs_rust, &options, None)
                        .expect("Failed to run WMA indicator");
                    batch_full_output.extend_from_slice(&outputs[0]);
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let close_vec = close[..min_data_val].to_vec();
                    let chunk_inputs = [close_vec.as_slice()];

                    let (first_outputs, mut state) = rust_wma(&chunk_inputs, &options, None)
                        .expect("Failed to run WMA indicator on first chunk");
                    batch_full_output.extend_from_slice(&first_outputs[0]);

                    // Process remaining data in chunks using state
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for close_chunk in close_chunks.by_ref() {
                        let close_vec = close_chunk.to_vec();
                        let chunk_inputs = [close_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("WMA batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }

                    // Process remainder if any
                    let close_rem = close_chunks.remainder();

                    if !close_rem.is_empty() {
                        let close_vec = close_rem.to_vec();
                        let chunk_inputs = [close_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("WMA batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }
                }

                // Compare outputs
                for (i, (&full_val, &batch_val)) in full_outputs[0]
                    .iter()
                    .zip(batch_full_output.iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "WMA mismatch at index {}: full = {}, batch = {}, options = {:?}, stock = {}",
                        i, full_val, batch_val, options, stock_symbol
                    );
                }
            }
        }
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    #[test]
    fn test_wma_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::wma::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
            .collect();

        // Ensure we have exactly 4 stocks for SIMD testing
        if stock_data.len() < 4 {
            panic!(
                "Need at least 4 stocks for SIMD testing, but only found {}",
                stock_data.len()
            );
        }

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
                    .expect("SIMD by assets WMA indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = rust_wma(&stock_inputs, &options, None)
                        .expect("Regular WMA indicator failed");

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
                                "SIMD by assets WMA has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets WMA has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for WMA
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

        println!("✓ All SIMD by assets vs Regular WMA database tests passed!");
    }

    #[test]
    fn test_wma_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::wma::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
            .collect();

        // Ensure we have exactly 4 stocks for SIMD testing
        if stock_data.len() < 4 {
            panic!(
                "Need at least 4 stocks for SIMD testing, but only found {}",
                stock_data.len()
            );
        }

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
                    indicator_by_assets::<4>(&inputs, &options, Some(&[true]))
                        .expect("SIMD by assets WMA indicator with optional outputs failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results_opt, _) = rust_wma(&stock_inputs, &options, Some(&[true]))
                        .expect("Regular WMA indicator with optional outputs failed");

                    // Compare all outputs: WMA, SMA
                    let output_names = ["WMA", "SMA"];
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

        println!("✓ All SIMD by assets vs Regular WMA optional outputs database tests passed!");
    }
    const SMA_EPSILON: f64 = 1e-10; // Use epsilon from sma_test.rs

    #[test]
    fn test_wma_sma_optional_output_vs_c_tulip() {
        let close = expand_close();
        let inputs = [close.as_slice()];
        let period = 14.0;
        let options = [period];
        let optional_outputs = Some([true].as_slice()); // Request sma output

        // Get Rust WMA output with sma optional output
        let result = rust_wma(&inputs, &options, optional_outputs).unwrap();
        let rust_sma = &result.0[1]; // sma is at index 1

        // Fail fast if Rust output is empty
        if rust_sma.is_empty() {
            panic!("Rust WMA sma optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip SMA output for comparison
        let sma_inputs_c: Vec<*const f64> = vec![close.as_ptr()];
        let sma_options = [period];
        let sma_start_index = unsafe { ti_sma_start(sma_options.as_ptr()) };
        let sma_output_len = close.len() - (sma_start_index as usize);
        let mut c_sma = vec![0.0; sma_output_len];
        let mut sma_outputs_c = vec![c_sma.as_mut_ptr()];

        let ret = unsafe {
            ti_sma(
                close.len() as i32,
                sma_inputs_c.as_ptr(),
                sma_options.as_ptr(),
                sma_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_sma returned error code {}", ret);

        // Compare SMA outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing WMA sma optional output vs C Tulip SMA:");
        println!(
            "Rust sma length: {}, C SMA length: {}",
            rust_sma.len(),
            c_sma.len()
        );

        for (i, (rust_val, c_val)) in rust_sma.iter().rev().zip(c_sma.iter().rev()).enumerate() {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust sma output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            // Skip comparison if C output is NaN/infinite (assume C bug)
            if !c_val.is_finite() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > SMA_EPSILON {
                panic!(
                    "WMA sma mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ WMA sma optional output matches C Tulip SMA output");
    }

    #[test]
    fn test_wma_database_optional_sma() {
        const SMA_EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get WMA with SMA optional output
                let optional_outputs = Some(&[true][..]);
                let (wma_result, _) = tulip_rs::indicators::wma::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_sma = &wma_result[1];

                // Calculate expected SMA using C Tulip ti_sma
                let sma_options = [options[0]];
                let start_index = unsafe { ti_sma_start(sma_options.as_ptr()) };
                assert!(start_index >= 0, "ti_sma_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_sma_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_sma_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_sma(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        sma_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_sma failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_sma.len().min(c_sma_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_sma.len() - 1 - i;
                    let c_idx = c_sma_output.len() - 1 - i;

                    let rust_val = rust_sma[rust_idx];
                    let c_val = c_sma_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust SMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = SMA_EPSILON),
                        "WMA SMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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

    // SIMD-by-options test for WMA (compare SIMD to regular for all options)
    #[test]
    fn test_wma_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::wma::indicator_by_options;

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
                .expect("SIMD WMA 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD WMA 2-wide failed");

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
                    rust_wma(&inputs, options, None).expect("Regular WMA indicator failed");

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
                            "SIMD WMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD WMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
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
        }
    }

    // SIMD-by-options optional outputs test for WMA (compare SIMD to regular for all options)
    #[test]
    fn test_wma_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::wma::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Request optional SMA output
            let optional_outputs = Some(&[true][..]);

            // Process first 4 options with 4-wide SIMD (with optional outputs)
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD WMA 4-wide with optional outputs failed");

            // Process remaining 2 options with 2-wide SIMD (with optional outputs)
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) =
                indicator_by_options::<2>(&inputs, &options_2, optional_outputs)
                    .expect("SIMD WMA 2-wide with optional outputs failed");

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
                let (regular_results, _) = rust_wma(&inputs, options, optional_outputs)
                    .expect("Regular WMA indicator with optional outputs failed");

                // For WMA the outputs are: [WMA, SMA]
                let simd_wma_result = &all_simd_results[idx][0];
                let regular_wma_result = &regular_results[0];

                let simd_sma_result = &all_simd_results[idx][1];
                let regular_sma_result = &regular_results[1];

                // Compare WMA output lengths
                assert_eq!(
                    simd_wma_result.len(),
                    regular_wma_result.len(),
                    "WMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_wma_result.len(),
                    regular_wma_result.len()
                );

                // Compare SMA output lengths
                assert_eq!(
                    simd_sma_result.len(),
                    regular_sma_result.len(),
                    "SMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_sma_result.len(),
                    regular_sma_result.len()
                );

                // Compare WMA values
                for (i, (&simd_val, &regular_val)) in simd_wma_result
                    .iter()
                    .zip(regular_wma_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD WMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD WMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "WMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare SMA values
                for (i, (&simd_val, &regular_val)) in simd_sma_result
                    .iter()
                    .zip(regular_sma_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD SMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD SMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "SMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }
    }
}
