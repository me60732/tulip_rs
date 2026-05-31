#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::ppo::{indicator as rust_ppo, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_ema, ti_ema_start, ti_ppo, ti_ppo_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 2]; 4] = [[2.0, 5.0], [12.0, 26.0], [9.0, 20.0], [8.0, 18.0]];

    const CHUNK_SIZE: usize = 100;

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
    fn test_ppo_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C PPO function
            let start_index = unsafe { ti_ppo_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_ppo_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut ppo_output_vec_c = vec![0.0_f64; output_len_c];
            let ppo_ptr: *mut f64 = ppo_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![ppo_ptr];
            let ret = unsafe {
                ti_ppo(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_ppo returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_ppo(&inputs_rust, &options, None).expect("Rust PPO indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in ppo_output_vec_c
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
                        "Rust PPO has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust PPO has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-12) {
                    // Adjust epsilon if needed
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, ppo_output_vec_c, outputs[0], options
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
    fn test_ppo_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_ppo_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_ppo_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_ppo(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_ppo returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_ppo(&inputs_rust, &options, None).expect("Rust PPO indicator failed");

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
                            "Rust PPO has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust PPO has infinity at index {}: Rust = {}, Options = {:?}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-12) {
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
    fn test_ppo_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs_rust = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = rust_ppo(&inputs_rust, &options, None)
                    .expect("PPO indicator should work on full data");

                // Process in batches
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // Process first chunk to get initial state
                let first_chunk_size = min_data_val.min(close.len());
                let first_close = close[..first_chunk_size].to_vec();
                let first_inputs = [first_close.as_slice()];

                let (outputs, mut state) = rust_ppo(&first_inputs, &options, None)
                    .expect("PPO indicator should work on first chunk");

                for output_idx in 0..outputs.len() {
                    batch_full_outputs[output_idx].extend_from_slice(&outputs[output_idx]);
                }

                let mut processed = first_chunk_size;

                // Process subsequent chunks using state.batch_indicator
                while processed < close.len() {
                    let end = (processed + CHUNK_SIZE).min(close.len());

                    let chunk_close = close[processed..end].to_vec();
                    let chunk_inputs = [chunk_close.as_slice()];

                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("PPO batch indicator failed");

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
    fn test_ppo_simd_vs_regular_database() {
        use tulip_rs::indicators::ppo::indicator_by_assets;

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
            &[&stock_data[0].1], // close
            &[&stock_data[1].1], // close
            &[&stock_data[2].1], // close
            &[&stock_data[3].1], // close
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets PPO indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) =
                    rust_ppo(&stock_inputs, &options, None).expect("Regular PPO indicator failed");

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
                                "SIMD by assets PPO has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                                "SIMD by assets PPO has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                    }

                    // Compare values with appropriate epsilon
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        println!(
                            "SIMD PPO: {:?}\n\nRegular PPO {:?}",
                            &simd_result[..],
                            &regular_result[..]
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

        println!("✓ All SIMD by assets vs Regular PPO database tests passed!");
    }

    #[test]
    fn test_ppo_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::ppo::indicator_by_assets;

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
                        .expect("SIMD by assets PPO indicator with optional outputs failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results_opt, _) =
                        rust_ppo(&stock_inputs, &options, Some(&[true, true]))
                            .expect("Regular PPO indicator with optional outputs failed");

                    // Compare all outputs: PPO, short_ema, long_ema
                    let output_names = ["PPO", "short_ema", "long_ema"];
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
                            if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
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

        println!("✓ All SIMD by assets vs Regular PPO optional outputs database tests passed!");
    }
    #[test]
    fn test_ppo_short_ema_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let close = expand_close();
        let inputs = [close.as_slice()];
        let options = [12.0, 26.0]; // short=12, long=26
        let optional_outputs = Some([true, false].as_slice()); // Request short_ema output

        // Get Rust PPO output with short_ema optional output
        let result = rust_ppo(&inputs, &options, optional_outputs).unwrap();
        let rust_short_ema = &result.0[1]; // short_ema is at index 1

        // Fail fast if Rust output is empty
        if rust_short_ema.is_empty() {
            panic!("Rust PPO short_ema optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip EMA output for short period for comparison
        let ema_inputs_c: Vec<*const f64> = vec![close.as_ptr()];
        let short_ema_options = [options[0]]; // short period
        let ema_start_index = unsafe { ti_ema_start(short_ema_options.as_ptr()) };
        let ema_output_len = close.len() - (ema_start_index as usize);
        let mut c_short_ema = vec![0.0; ema_output_len];
        let mut ema_outputs_c = vec![c_short_ema.as_mut_ptr()];

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

        // Compare short EMA outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing PPO short_ema optional output vs C Tulip short EMA:");
        println!(
            "Rust short_ema length: {}, C short EMA length: {}",
            rust_short_ema.len(),
            c_short_ema.len()
        );

        for (i, (rust_val, c_val)) in rust_short_ema
            .iter()
            .rev()
            .zip(c_short_ema.iter().rev())
            .enumerate()
        {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust short_ema output contains NaN/infinity at position {}: {}",
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
            if diff > EPSILON {
                panic!(
                    "PPO short_ema mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ PPO short_ema optional output matches C Tulip short EMA output");
    }

    #[test]
    fn test_ppo_long_ema_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let close = expand_close();
        let inputs = [close.as_slice()];
        let options = [12.0, 26.0]; // short=12, long=26
        let optional_outputs = Some([false, true].as_slice()); // Request long_ema output

        // Get Rust PPO output with long_ema optional output
        let result = rust_ppo(&inputs, &options, optional_outputs).unwrap();
        let rust_long_ema = &result.0[2]; // long_ema is at index 2

        // Fail fast if Rust output is empty
        if rust_long_ema.is_empty() {
            panic!("Rust PPO long_ema optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip EMA output for long period for comparison
        let ema_inputs_c: Vec<*const f64> = vec![close.as_ptr()];
        let long_ema_options = [options[1]]; // long period
        let ema_start_index = unsafe { ti_ema_start(long_ema_options.as_ptr()) };
        let ema_output_len = close.len() - (ema_start_index as usize);
        let mut c_long_ema = vec![0.0; ema_output_len];
        let mut ema_outputs_c = vec![c_long_ema.as_mut_ptr()];

        let ret = unsafe {
            ti_ema(
                close.len() as i32,
                ema_inputs_c.as_ptr(),
                long_ema_options.as_ptr(),
                ema_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_ema for long period returned error code {}", ret);

        // Compare long EMA outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing PPO long_ema optional output vs C Tulip long EMA:");
        println!(
            "Rust long_ema length: {}, C long EMA length: {}",
            rust_long_ema.len(),
            c_long_ema.len()
        );

        for (i, (rust_val, c_val)) in rust_long_ema
            .iter()
            .rev()
            .zip(c_long_ema.iter().rev())
            .enumerate()
        {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust long_ema output contains NaN/infinity at position {}: {}",
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
            if diff > EPSILON {
                panic!(
                    "PPO long_ema mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ PPO long_ema optional output matches C Tulip long EMA output");
    }

    #[test]
    fn test_ppo_database_optional_short_ema() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get PPO with short_ema optional output
                let optional_outputs = Some(&[true, false][..]);
                let (ppo_result, _) = tulip_rs::indicators::ppo::indicator(
                    &[&close],
                    &[options[0], options[1]],
                    optional_outputs,
                )
                .unwrap();

                let rust_short_ema = &ppo_result[1];

                // Calculate expected short EMA using C Tulip ti_ema
                let short_ema_options = [options[0]]; // short period
                let start_index = unsafe { ti_ema_start(short_ema_options.as_ptr()) };
                assert!(start_index >= 0, "ti_ema_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_short_ema_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_short_ema_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_ema(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        short_ema_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_ema failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_short_ema.len().min(c_short_ema_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_short_ema.len() - 1 - i;
                    let c_idx = c_short_ema_output.len() - 1 - i;

                    let rust_val = rust_short_ema[rust_idx];
                    let c_val = c_short_ema_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust short EMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "PPO short EMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_ppo_database_optional_long_ema() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get PPO with long_ema optional output
                let optional_outputs = Some(&[false, true][..]);
                let (ppo_result, _) = tulip_rs::indicators::ppo::indicator(
                    &[&close],
                    &[options[0], options[1]],
                    optional_outputs,
                )
                .unwrap();

                let rust_long_ema = &ppo_result[2];

                // Calculate expected long EMA using C Tulip ti_ema
                let long_ema_options = [options[1]]; // long period
                let start_index = unsafe { ti_ema_start(long_ema_options.as_ptr()) };
                assert!(start_index >= 0, "ti_ema_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_long_ema_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_long_ema_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_ema(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        long_ema_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_ema failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_long_ema.len().min(c_long_ema_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_long_ema.len() - 1 - i;
                    let c_idx = c_long_ema_output.len() - 1 - i;

                    let rust_val = rust_long_ema[rust_idx];
                    let c_val = c_long_ema_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust long EMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "PPO long EMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_ppo_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::ppo::indicator_by_options;

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
                .expect("SIMD PPO 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_ppo(&inputs, options, None).expect("Regular PPO indicator failed");

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
                            "SIMD PPO has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD PPO has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-10) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular PPO database tests passed!");
    }

    #[test]
    fn test_ppo_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::ppo::indicator_by_options;
        const EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Test with all optional outputs enabled
            let optional_outputs = Some([true, true].as_slice()); // [short_ema, long_ema]

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD PPO 4-wide with optional outputs failed");

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_ppo(&inputs, options, optional_outputs)
                    .expect("Regular PPO indicator with optional outputs failed");

                let simd_results = &simd_results_4[idx];
                let regular_results: &Vec<Vec<f64>> = &regular_results;

                // Compare all outputs (main + optional)
                assert_eq!(
                    simd_results.len(),
                    regular_results.len(),
                    "Number of outputs mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_results.len(),
                    regular_results.len()
                );

                for (output_idx, (simd_output, regular_output)) in
                    simd_results.iter().zip(regular_results.iter()).enumerate()
                {
                    // Compare output lengths
                    assert_eq!(
                        simd_output.len(),
                        regular_output.len(),
                        "Output {} length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
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
                                "SIMD PPO output {} has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                                output_idx, i, stock_symbol, simd_val, options
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD PPO output {} has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                                output_idx, i, stock_symbol, simd_val, options
                            );
                        }

                        // Compare values with tolerance
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "Output {} mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                                output_idx, i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular PPO database tests with optional outputs passed!"
        );
    }
}
