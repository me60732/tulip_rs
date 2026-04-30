#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::bbands::{indicator as rust_bbands, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_bbands, ti_bbands_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const MARGIN: f64 = 1e-4;
    const CHUNK_SIZE: usize = 100;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 2]; 4] = [[5.0, 2.0], [14.0, 2.0], [20.0, 2.0], [50.0, 2.0]];

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
    fn test_bbands_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C BBANDS function
            let start_index = unsafe { ti_bbands_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_bbands_start returned a negative index"
            );
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut lower_output_vec_c = vec![0.0_f64; output_len_c];
            let mut middle_output_vec_c = vec![0.0_f64; output_len_c];
            let mut upper_output_vec_c = vec![0.0_f64; output_len_c];
            let lower_ptr: *mut f64 = lower_output_vec_c.as_mut_ptr();
            let middle_ptr: *mut f64 = middle_output_vec_c.as_mut_ptr();
            let upper_ptr: *mut f64 = upper_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![lower_ptr, middle_ptr, upper_ptr];
            let ret = unsafe {
                ti_bbands(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_bbands returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_bbands(&inputs_rust, &options, None).expect("Rust BBANDS indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the LOWER outputs in reverse
            for (i, (&c_val, &rust_val)) in lower_output_vec_c
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
                        "Rust BBANDS LOWER has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust BBands has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = MARGIN) {
                    /*println!(
                        "Test BBand Lower failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, lower_output_vec_c, outputs[0], options
                    );*/
                    panic!(
                        "bbands Lower Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }

            // Compare the MIDDLE outputs in reverse
            for (i, (&c_val, &rust_val)) in middle_output_vec_c
                .iter()
                .rev()
                .take(output_len_rust)
                .zip(outputs[1].iter().rev())
                .enumerate()
            {
                let index = output_len_rust - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust BBANDS MIDDLE has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Skip if only C has NaN (C bug)
                if c_val.is_nan() && !rust_val.is_nan() {
                    continue;
                }

                if !approx_eq!(f64, c_val, rust_val, epsilon = MARGIN) {
                    /*println!(
                        "Test bbands Middle failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, middle_output_vec_c, outputs[1], options
                    );*/
                    panic!(
                        "bbands Middle Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }

            // Compare the UPPER outputs in reverse
            for (i, (&c_val, &rust_val)) in upper_output_vec_c
                .iter()
                .rev()
                .take(output_len_rust)
                .zip(outputs[2].iter().rev())
                .enumerate()
            {
                let index = output_len_rust - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust BBANDS UPPER has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust BBands has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = MARGIN) {
                    /*println!(
                        "Test bbands Upper failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, upper_output_vec_c, outputs[2], options
                    );*/
                    panic!(
                        "bbands Upper Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_bbands_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_bbands_start(options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_bbands_start returned a negative index"
                );
                let output_len_c = close.len() - (start_index as usize);

                let mut lower_output_vec_c = vec![0.0_f64; output_len_c];
                let mut middle_output_vec_c = vec![0.0_f64; output_len_c];
                let mut upper_output_vec_c = vec![0.0_f64; output_len_c];
                let lower_ptr: *mut f64 = lower_output_vec_c.as_mut_ptr();
                let middle_ptr: *mut f64 = middle_output_vec_c.as_mut_ptr();
                let upper_ptr: *mut f64 = upper_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![lower_ptr, middle_ptr, upper_ptr];
                let ret = unsafe {
                    ti_bbands(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_bbands returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) = rust_bbands(&inputs_rust, &options, None)
                    .expect("Rust BBANDS indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare LOWER results
                for (i, (&c_val, &rust_val)) in lower_output_vec_c
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
                            "Rust BBANDS LOWER has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Skip if only C has NaN (C bug)
                    if c_val.is_nan() && !rust_val.is_nan() {
                        continue;
                    }

                    if !approx_eq!(f64, c_val, rust_val, epsilon = MARGIN) {
                        /*println!(
                            "BBANDS LOWER test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, lower_output_vec_c, outputs[0], options, stock_symbol
                        );*/
                        panic!(
                            "BBANDS LOWER mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }

                // Compare MIDDLE results
                for (i, (&c_val, &rust_val)) in middle_output_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust)
                    .zip(outputs[1].iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() {
                        panic!(
                            "Rust BBANDS MIDDLE has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Skip if only C has NaN (C bug)
                    if c_val.is_nan() && !rust_val.is_nan() {
                        continue;
                    }

                    if !approx_eq!(f64, c_val, rust_val, epsilon = MARGIN) {
                        /*println!(
                            "BBANDS MIDDLE test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, middle_output_vec_c, outputs[1], options, stock_symbol
                        );*/
                        panic!(
                            "BBANDS MIDDLE mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }

                // Compare UPPER results
                for (i, (&c_val, &rust_val)) in upper_output_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust)
                    .zip(outputs[2].iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() {
                        panic!(
                            "Rust BBANDS UPPER has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Skip if only C has NaN (C bug)
                    if c_val.is_nan() && !rust_val.is_nan() {
                        continue;
                    }

                    if !approx_eq!(f64, c_val, rust_val, epsilon = MARGIN) {
                        /*println!(
                            "BBANDS UPPER test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, upper_output_vec_c, outputs[2], options, stock_symbol
                        );*/
                        panic!(
                            "BBANDS UPPER mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_bbands_simd_vs_regular_database() {
        use tulip_rs::indicators::bbands::indicator_by_assets;

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
                    .expect("SIMD by assets BBANDS indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = rust_bbands(&stock_inputs, &options, None)
                        .expect("Regular BBANDS indicator failed");

                    // Compare all three outputs: lower, middle, upper
                    let output_names = ["Lower", "Middle", "Upper"];
                    for (output_idx, output_name) in output_names.iter().enumerate() {
                        let simd_result = &simd_results[stock_idx][output_idx];
                        let regular_result = &regular_results[output_idx];

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
                                    "SIMD by assets BBANDS {} has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                    output_name, i, stock_symbol, options, simd_val
                                );
                            }

                            if simd_val.is_infinite() {
                                panic!(
                                    "SIMD by assets BBANDS {} has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                    output_name, i, stock_symbol, options, simd_val
                                );
                            }

                            // Compare values with appropriate epsilon for BBANDS
                            if !approx_eq!(f64, simd_val, regular_val, epsilon = MARGIN) {
                                panic!(
                                    "Mismatch in {} output at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                    output_name, i, stock_symbol, options, simd_val, regular_val
                                );
                            }
                        }
                    }

                    println!(
                        "✓ SIMD by assets vs Regular test passed for stock {} with options {:?}",
                        stock_symbol, options
                    );
                }
            }
        }

        println!("✓ All SIMD by assets vs Regular BBANDS database tests passed!");
    }

    #[test]
    fn test_bbands_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::bbands::indicator_by_options;

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
                .expect("SIMD BBANDS 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_bbands(&inputs, options, None).expect("Regular BBANDS indicator failed");

                // BBANDS produces 3 outputs: [lower, middle, upper]
                assert_eq!(
                    all_simd_results[idx].len(),
                    3,
                    "SIMD result should have 3 outputs"
                );
                assert_eq!(
                    regular_results.len(),
                    3,
                    "Regular result should have 3 outputs"
                );

                let output_names = ["Lower Band", "Middle Band", "Upper Band"];

                for (output_idx, output_name) in output_names.iter().enumerate() {
                    let simd_result = &all_simd_results[idx][output_idx];
                    let regular_result = &regular_results[output_idx];

                    // Compare output lengths
                    assert_eq!(
                        simd_result.len(),
                        regular_result.len(),
                        "{} output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
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
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = MARGIN) {
                            panic!(
                                "{} mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                                output_name, i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular BBANDS database tests passed!");
    }

    #[test]
    fn test_bbands_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) = rust_bbands(&inputs_rust, &options, None)
                    .expect("Rust BBANDS indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) = rust_bbands(&chunk_inputs, &options, None)
                    .expect("Rust BBANDS indicator failed");
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
                        .expect("Rust BBANDS batch indicator failed");
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
                        .expect("Rust BBANDS batch indicator failed");
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
}
