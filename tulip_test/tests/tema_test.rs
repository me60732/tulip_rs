#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::tema::indicator_by_assets;
    use tulip_rs::indicators::tema::indicator_by_options;
    use tulip_rs::indicators::tema::{indicator as rust_tema, min_data, TIndicatorState};
    use tulip_test::c_bindings::{
        ti_dema, ti_dema_start, ti_ema, ti_ema_start, ti_tema, ti_tema_start,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILION: f64 = 1e-10;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [50.0]];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_close() -> Vec<f64> {
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..500 {
            close_vec.extend_from_slice(&CLOSE);
        }
        close_vec
    }

    #[test]
    fn test_tema_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C TEMA function
            let start_index = unsafe { ti_tema_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_tema_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut tema_output_vec_c = vec![0.0_f64; output_len_c];
            let tema_ptr: *mut f64 = tema_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![tema_ptr];
            let ret = unsafe {
                ti_tema(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_tema returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_tema(&inputs_rust, &options, None).expect("Rust TEMA indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in tema_output_vec_c
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
                        "Rust TEMA has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust TEMA has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILION) {
                    // Adjust epsilon if needed
                    /*println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, tema_output_vec_c, outputs[0], options
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
    fn test_tema_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // run c code
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                // Determine the offset required by the C TEMA function
                let start_index = unsafe { ti_tema_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_tema_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                // Run the C implementation
                let mut tema_output_vec_c = vec![0.0_f64; output_len_c];
                let tema_ptr: *mut f64 = tema_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![tema_ptr];
                let ret = unsafe {
                    ti_tema(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_tema returned error code {}", ret);

                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_tema(&inputs_rust, &options, None).expect("Rust TEMA indicator failed");

                let output_len_rust = outputs[0].len();

                for (i, (&c_val, &rust_val)) in tema_output_vec_c
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
                            "Rust TEMA has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust TEMA has infinity at index {}: Rust = {}, Options = {:?}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILION) {
                        /*println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, tema_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_tema_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_tema(&inputs_rust, &options, None).expect("Rust TEMA indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) =
                    rust_tema(&chunk_inputs, &options, None).expect("Rust TEMA indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("TEMA batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("TEMA batch indicator failed");
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
    fn test_tema_simd_by_assets() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for SIMD (4 assets with same data)
            let inputs: [&[&[f64]; 1]; 4] = [
                &[close.as_slice()],
                &[close.as_slice()],
                &[close.as_slice()],
                &[close.as_slice()],
            ];

            // Run SIMD implementation
            let (simd_outputs, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD TEMA indicator failed");

            // Run regular implementation for comparison
            let inputs_rust = [close.as_slice()];
            let (regular_outputs, _) =
                rust_tema(&inputs_rust, &options, None).expect("Regular TEMA indicator failed");

            // Compare each SIMD asset output with regular output
            for (asset_idx, simd_output_data) in simd_outputs.iter().enumerate() {
                let simd_output = &simd_output_data[0];
                let regular_output = &regular_outputs[0];

                assert_eq!(
                    simd_output.len(),
                    regular_output.len(),
                    "Output length mismatch for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                    asset_idx,
                    simd_output.len(),
                    regular_output.len(),
                    options
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_output.iter().zip(regular_output.iter()).enumerate()
                {
                    // Check for NaN or infinity in SIMD output
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD TEMA has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TEMA has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "SIMD vs Regular mismatch at index {} for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                            i, asset_idx, simd_val, regular_val, options
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_tema_simd_by_assets_database() {
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
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets TEMA indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) = rust_tema(&stock_inputs, &options, None)
                    .expect("Regular TEMA indicator failed");

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
                            "SIMD by assets TEMA has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets TEMA has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    // Compare values with high precision
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
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

        println!("✓ All SIMD by assets vs Regular TEMA database tests passed!");
    }

    #[test]
    fn test_tema_simd_by_assets_optional_outputs() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for SIMD (4 assets with same data)
            let inputs: [&[&[f64]; 1]; 4] = [
                &[close.as_slice()],
                &[close.as_slice()],
                &[close.as_slice()],
                &[close.as_slice()],
            ];

            // Test with optional outputs (both DEMA and EMA)
            let (simd_outputs_opt, _) =
                indicator_by_assets::<4>(&inputs, &options, Some(&[true, true]))
                    .expect("SIMD TEMA indicator with optional outputs failed");

            // Run regular implementation for comparison with optional outputs
            let inputs_rust = [close.as_slice()];
            let (regular_outputs_opt, _) = rust_tema(&inputs_rust, &options, Some(&[true, true]))
                .expect("Regular TEMA indicator with optional outputs failed");

            // Compare each SIMD asset output with regular output
            for (asset_idx, simd_output_opt_data) in simd_outputs_opt.iter().enumerate() {
                // Compare TEMA output (index 0)
                let simd_tema = &simd_output_opt_data[0];
                let regular_tema = &regular_outputs_opt[0];

                assert_eq!(
                    simd_tema.len(),
                    regular_tema.len(),
                    "TEMA output length mismatch for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                    asset_idx,
                    simd_tema.len(),
                    regular_tema.len(),
                    options
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_tema.iter().zip(regular_tema.iter()).enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD TEMA output has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TEMA output has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "TEMA output mismatch at index {} for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                            i, asset_idx, simd_val, regular_val, options
                        );
                    }
                }

                // Compare DEMA output (index 1) if it exists
                if simd_outputs_opt[asset_idx].len() > 1 && regular_outputs_opt.len() > 1 {
                    let simd_dema = &simd_outputs_opt[asset_idx][1];
                    let regular_dema = &regular_outputs_opt[1];

                    // Skip empty optional outputs
                    if !simd_dema.is_empty() && !regular_dema.is_empty() {
                        assert_eq!(
                            simd_dema.len(),
                            regular_dema.len(),
                            "DEMA output length mismatch for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                            asset_idx,
                            simd_dema.len(),
                            regular_dema.len(),
                            options
                        );

                        for (i, (&simd_val, &regular_val)) in
                            simd_dema.iter().zip(regular_dema.iter()).enumerate()
                        {
                            if simd_val.is_nan() {
                                panic!(
                                    "SIMD DEMA output has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                                    i, asset_idx, simd_val, options
                                );
                            }

                            if simd_val.is_infinite() {
                                panic!(
                                    "SIMD DEMA output has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                                    i, asset_idx, simd_val, options
                                );
                            }

                            if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                                panic!(
                                    "DEMA output mismatch at index {} for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                                    i, asset_idx, simd_val, regular_val, options
                                );
                            }
                        }
                    }
                }

                // Compare EMA output (index 2) if it exists
                if simd_outputs_opt[asset_idx].len() > 2 && regular_outputs_opt.len() > 2 {
                    let simd_ema = &simd_outputs_opt[asset_idx][2];
                    let regular_ema = &regular_outputs_opt[2];

                    // Skip empty optional outputs
                    if !simd_ema.is_empty() && !regular_ema.is_empty() {
                        assert_eq!(
                            simd_ema.len(),
                            regular_ema.len(),
                            "EMA output length mismatch for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                            asset_idx,
                            simd_ema.len(),
                            regular_ema.len(),
                            options
                        );

                        for (i, (&simd_val, &regular_val)) in
                            simd_ema.iter().zip(regular_ema.iter()).enumerate()
                        {
                            if simd_val.is_nan() {
                                panic!(
                                    "SIMD EMA output has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                                    i, asset_idx, simd_val, options
                                );
                            }

                            if simd_val.is_infinite() {
                                panic!(
                                    "SIMD EMA output has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                                    i, asset_idx, simd_val, options
                                );
                            }

                            if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                                panic!(
                                    "EMA output mismatch at index {} for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                                    i, asset_idx, simd_val, regular_val, options
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_tema_simd_by_assets_database_optional_outputs() {
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
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result with optional outputs
            let (simd_results_opt, _) =
                indicator_by_assets::<4>(&inputs, &options, Some(&[true, true]))
                    .expect("SIMD by assets TEMA indicator with optional outputs failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock with optional outputs
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results_opt, _) =
                    rust_tema(&stock_inputs, &options, Some(&[true, true]))
                        .expect("Regular TEMA indicator with optional outputs failed");

                // Compare all outputs: TEMA, DEMA, EMA
                let output_names = ["TEMA", "DEMA", "EMA"];
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

                        // Compare values with high precision
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
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

        println!("✓ All SIMD by assets vs Regular TEMA optional outputs database tests passed!");
    }

    //ADD TEST FUNCTIONS HERE

    #[test]
    fn test_tema_simd_by_options_vs_regular_database() {
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
                .expect("SIMD TEMA 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_tema(&inputs, options, None).expect("Regular TEMA indicator failed");

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
                            "SIMD TEMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TEMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular TEMA database tests passed!");
    }

    #[test]
    fn test_tema_simd_by_options_vs_regular_database_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Test with EMA and DEMA optional outputs
            let optional_outputs = Some(&[true, true][..]);

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD TEMA 4-wide with optional outputs failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_tema(&inputs, options, optional_outputs)
                    .expect("Regular TEMA indicator with optional outputs failed");

                let simd_tema_result = &all_simd_results[idx][0];
                let regular_tema_result = &regular_results[0];

                let simd_ema_result = &all_simd_results[idx][1];
                let regular_ema_result = &regular_results[1];

                let simd_dema_result = &all_simd_results[idx][2];
                let regular_dema_result = &regular_results[2];

                // Compare TEMA output lengths
                assert_eq!(
                    simd_tema_result.len(),
                    regular_tema_result.len(),
                    "TEMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_tema_result.len(),
                    regular_tema_result.len()
                );

                // Compare EMA output lengths
                assert_eq!(
                    simd_ema_result.len(),
                    regular_ema_result.len(),
                    "EMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_ema_result.len(),
                    regular_ema_result.len()
                );

                // Compare DEMA output lengths
                assert_eq!(
                    simd_dema_result.len(),
                    regular_dema_result.len(),
                    "DEMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_dema_result.len(),
                    regular_dema_result.len()
                );

                // Compare TEMA values
                for (i, (&simd_val, &regular_val)) in simd_tema_result
                    .iter()
                    .zip(regular_tema_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD TEMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TEMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "TEMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare EMA values
                for (i, (&simd_val, &regular_val)) in simd_ema_result
                    .iter()
                    .zip(regular_ema_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD EMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD EMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "EMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare DEMA values
                for (i, (&simd_val, &regular_val)) in simd_dema_result
                    .iter()
                    .zip(regular_dema_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD DEMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD DEMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "DEMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular TEMA database tests with optional outputs passed!"
        );
    }

    #[test]
    fn test_tema_dema_optional_output_vs_c_tulip() {
        const EPSILON: f64 = EPSILION;
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Get Rust TEMA with DEMA optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) = rust_tema(&inputs_rust, &options, Some(&[true, false]))
                .expect("Rust TEMA indicator failed");

            assert!(!outputs.is_empty(), "TEMA outputs should not be empty");
            assert!(
                outputs.len() >= 2,
                "TEMA should have at least 2 outputs when optional outputs enabled"
            );

            let rust_dema_output = &outputs[1]; // DEMA is at index 1

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_dema_output.is_empty(),
                "DEMA optional output vector should not be empty"
            );

            // Get C DEMA reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_dema_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_dema_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut dema_output_vec_c = vec![0.0_f64; output_len_c];
            let dema_ptr: *mut f64 = dema_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![dema_ptr];
            let ret = unsafe {
                ti_dema(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_dema returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in dema_output_vec_c
                .iter()
                .rev()
                .zip(rust_dema_output.iter().rev())
                .enumerate()
            {
                let index = rust_dema_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust DEMA optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust DEMA optional output has infinity at index {}: Rust = {}, Options = {:?}",
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
                        "DEMA optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_tema_ema_optional_output_vs_c_tulip() {
        const EPSILON: f64 = EPSILION;
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Get Rust TEMA with EMA optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) = rust_tema(&inputs_rust, &options, Some(&[false, true]))
                .expect("Rust TEMA indicator failed");

            assert!(!outputs.is_empty(), "TEMA outputs should not be empty");
            assert!(
                outputs.len() >= 3,
                "TEMA should have at least 3 outputs when optional outputs enabled"
            );

            let rust_ema_output = &outputs[2]; // EMA is at index 2

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_ema_output.is_empty(),
                "EMA optional output vector should not be empty"
            );

            // Get C EMA reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_ema_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_ema_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut ema_output_vec_c = vec![0.0_f64; output_len_c];
            let ema_ptr: *mut f64 = ema_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![ema_ptr];
            let ret = unsafe {
                ti_ema(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_ema returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in ema_output_vec_c
                .iter()
                .rev()
                .zip(rust_ema_output.iter().rev())
                .enumerate()
            {
                let index = rust_ema_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust EMA optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust EMA optional output has infinity at index {}: Rust = {}, Options = {:?}",
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
                        "EMA optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
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
    fn test_tema_database_optional_dema() {
        const EPSILON: f64 = EPSILION;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get TEMA with DEMA optional output
                let optional_outputs = Some(&[true, false][..]);
                let (tema_result, _) = tulip_rs::indicators::tema::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_dema = &tema_result[1];

                // Calculate expected DEMA using C Tulip ti_dema
                let dema_options = [options[0]];
                let start_index = unsafe { ti_dema_start(dema_options.as_ptr()) };
                assert!(start_index >= 0, "ti_dema_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_dema_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_dema_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_dema(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        dema_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_dema failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_dema.len().min(c_dema_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_dema.len() - 1 - i;
                    let c_idx = c_dema_output.len() - 1 - i;

                    let rust_val = rust_dema[rust_idx];
                    let c_val = c_dema_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust DEMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "TEMA DEMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_tema_database_optional_ema() {
        const EPSILON: f64 = EPSILION;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get TEMA with EMA optional output
                let optional_outputs = Some(&[false, true][..]);
                let (tema_result, _) = tulip_rs::indicators::tema::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_ema = &tema_result[2];

                // Calculate expected EMA using C Tulip ti_ema
                let ema_options = [options[0]];
                let start_index = unsafe { ti_ema_start(ema_options.as_ptr()) };
                assert!(start_index >= 0, "ti_ema_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_ema_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_ema_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_ema(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        ema_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_ema failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_ema.len().min(c_ema_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_ema.len() - 1 - i;
                    let c_idx = c_ema_output.len() - 1 - i;

                    let rust_val = rust_ema[rust_idx];
                    let c_val = c_ema_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust EMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "TEMA EMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
