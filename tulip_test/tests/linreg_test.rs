#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::linreg::{indicator as rust_linreg, min_data, TIndicatorState};
    use tulip_test::c_bindings::{
        ti_linreg, ti_linreg_start, ti_linregintercept, ti_linregintercept_start, ti_linregslope,
        ti_linregslope_start,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [25.0]];

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
    fn test_linreg_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C LINREG function
            let start_index = unsafe { ti_linreg_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_linreg_start returned a negative index"
            );
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut linreg_output_vec_c = vec![0.0_f64; output_len_c];
            let linreg_ptr: *mut f64 = linreg_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![linreg_ptr];
            let ret = unsafe {
                ti_linreg(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_linreg returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_linreg(&inputs_rust, &options, None).expect("Rust LINREG indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in linreg_output_vec_c
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
                        "Rust LINREG has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust LINREG has infinity at index {}: Rust = {}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-9) {
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, linreg_output_vec_c, outputs[0], options
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
    fn test_linreg_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_linreg_start(options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_linreg_start returned a negative index"
                );
                let output_len_c = close.len() - (start_index as usize);

                let mut linreg_output_vec_c = vec![0.0_f64; output_len_c];
                let linreg_ptr: *mut f64 = linreg_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![linreg_ptr];
                let ret = unsafe {
                    ti_linreg(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_linreg returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) = rust_linreg(&inputs_rust, &options, None)
                    .expect("Rust LINREG indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare results
                for (i, (&c_val, &rust_val)) in linreg_output_vec_c
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
                            "Rust LINREG has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust LINREG has infinity at index {}: Rust = {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-9) {
                        /*println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, linreg_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_linreg_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs_rust = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = rust_linreg(&inputs_rust, &options, None)
                    .expect("Failed to run LINREG indicator on full data");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) = rust_linreg(&chunk_inputs, &options, None)
                    .expect("Failed to run LINREG indicator on first chunk");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks using state
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("LINREG batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Process remainder if any
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("LINREG batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
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
                        "Mismatch in LINREG output at index {}: full = {}, batch = {}, Stock: {}, Options: {:?}",
                        i, full_val, batch_val, stock_symbol, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_linreg_simd_vs_regular_database() {
        use tulip_rs::indicators::linreg::indicator_by_assets;

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
                    .expect("SIMD by assets LINREG indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = rust_linreg(&stock_inputs, &options, None)
                        .expect("Regular LINREG indicator failed");

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
                                "SIMD by assets LINREG has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets LINREG has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for LINREG
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-9) {
                            println!(
                                "SIMD: {:?}\n\nRegular: {:?}",
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
        }

        println!("✓ All SIMD by assets vs Regular LINREG database tests passed!");
    }

    #[test]
    fn test_linreg_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::linreg::indicator_by_assets;

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
                        .expect("SIMD by assets LINREG indicator with optional outputs failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results_opt, _) =
                        rust_linreg(&stock_inputs, &options, Some(&[true, true]))
                            .expect("Regular LINREG indicator with optional outputs failed");

                    // Compare all outputs: LINREG, slope, intercept
                    let output_names = ["LINREG", "slope", "intercept"];
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
                            if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-9) {
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

        println!("✓ All SIMD by assets vs Regular LINREG optional outputs database tests passed!");
    }

    #[test]
    fn test_linreg_linregslope_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let close = CLOSE.to_vec();
        let inputs = [close.as_slice()];
        let options = [5.0]; // period = 5
        let optional_outputs = Some([true, false].as_slice()); // Request linregslope output

        // Get Rust LINREG output with linregslope optional output
        let result = rust_linreg(&inputs, &options, optional_outputs).unwrap();
        let rust_linregslope = &result.0[1]; // linregslope is at index 1

        // Fail fast if Rust output is empty
        if rust_linregslope.is_empty() {
            panic!("Rust LINREG linregslope optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip linregslope output for comparison
        let c_inputs: Vec<*const f64> = vec![close.as_ptr()];
        let c_options = [5.0];
        let c_start_index = unsafe { ti_linregslope_start(c_options.as_ptr()) } as usize;
        let c_output_len = close.len() - c_start_index;
        let mut c_linregslope = vec![0.0; c_output_len];
        let mut c_outputs = vec![c_linregslope.as_mut_ptr()];

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
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing LINREG linregslope optional output vs C Tulip linregslope:");
        println!(
            "Rust linregslope length: {}, C linregslope length: {}",
            rust_linregslope.len(),
            c_linregslope.len()
        );

        for (i, (rust_val, c_val)) in rust_linregslope
            .iter()
            .rev()
            .zip(c_linregslope.iter().rev())
            .enumerate()
        {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust linregslope output contains NaN/infinity at position {}: {}",
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
                    "LINREG linregslope mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ LINREG linregslope optional output matches C Tulip linregslope output");
    }

    #[test]
    fn test_linreg_linregintercept_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let close = CLOSE.to_vec();
        let inputs = [close.as_slice()];
        let options = [5.0]; // period = 5
        let optional_outputs = Some([true, true].as_slice()); // Request both slope and intercept outputs

        // Get Rust LINREG output with slope and intercept optional outputs
        let result = rust_linreg(&inputs, &options, optional_outputs).unwrap();
        let rust_slope = &result.0[1]; // slope is at index 1
        let rust_intercept = &result.0[2]; // intercept is at index 2

        // Calculate intercept + slope * 1.0 to match C library's ti_linregintercept behavior
        let rust_linregintercept: Vec<f64> = rust_intercept
            .iter()
            .zip(rust_slope.iter())
            .map(|(intercept, slope)| intercept + slope * 1.0)
            .collect();

        // Fail fast if Rust output is empty
        if rust_linregintercept.is_empty() {
            panic!("Rust LINREG calculated linregintercept (intercept + slope * 1.0) is empty - this indicates an indicator bug");
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
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing LINREG calculated linregintercept (intercept + slope * 1.0) vs C Tulip linregintercept:");
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
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust calculated linregintercept (intercept + slope * 1.0) contains NaN/infinity at position {}: {}",
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
                println!(
                    "\nRUST: {:?}, \n\nC: {:?}",
                    rust_linregintercept, c_linregintercept
                );
                panic!(
                    "LINREG calculated linregintercept (intercept + slope * 1.0) mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ LINREG calculated linregintercept (intercept + slope * 1.0) matches C Tulip linregintercept output");
    }

    #[test]
    fn test_linreg_database_optional_slope() {
        const EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 20 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get LINREG with slope optional output
                let optional_outputs = Some(&[true, false][..]);
                let (linreg_result, _) = tulip_rs::indicators::linreg::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_slope = &linreg_result[1];

                // Calculate expected slope using C Tulip ti_linregslope
                let slope_options = [options[0]]; // period
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
                        "LINREG slope optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_linreg_database_optional_intercept() {
        const EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 20 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get LINREG with both slope and intercept optional outputs
                let optional_outputs = Some(&[true, true][..]);
                let (linreg_result, _) = tulip_rs::indicators::linreg::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_slope = &linreg_result[1];
                let rust_intercept = &linreg_result[2];

                // Calculate intercept + slope * 1.0 to match C library's ti_linregintercept behavior
                let rust_linregintercept: Vec<f64> = rust_intercept
                    .iter()
                    .zip(rust_slope.iter())
                    .map(|(intercept, slope)| intercept + slope * 1.0)
                    .collect();

                // Calculate expected intercept using C Tulip ti_linregintercept
                let intercept_options = [options[0]]; // period
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
                        "LINREG calculated linregintercept (intercept + slope * 1.0) optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_linreg_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::linreg::indicator as rust_linreg;
        use tulip_rs::indicators::linreg::indicator_by_options;

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
                .expect("SIMD LINREG 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_linreg(&inputs, options, None).expect("Regular LINREG indicator failed");

                let simd_result = &all_simd_results[idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "LINREG output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
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
                            "SIMD by options LINREG has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options LINREG has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with appropriate epsilon for LINREG
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-9) {
                        println!(
                            "SIMD: {:?}\n\nRegular: {:?}",
                            &simd_result[..20.min(simd_result.len())],
                            &regular_result[..20.min(regular_result.len())]
                        );
                        panic!(
                            "LINREG mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
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

        println!("✓ All SIMD by options vs Regular LINREG database tests passed!");
    }

    #[test]
    fn test_linreg_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::linreg::indicator as rust_linreg;
        use tulip_rs::indicators::linreg::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Test with all optional outputs: slope, intercept
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
                    .expect("SIMD LINREG 4-wide with optional outputs failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_linreg(&inputs, options, optional_outputs)
                    .expect("Regular LINREG indicator with optional outputs failed");

                let simd_linreg_result = &all_simd_results[idx][0];
                let regular_linreg_result = &regular_results[0];

                let simd_slope_result = &all_simd_results[idx][1];
                let regular_slope_result = &regular_results[1];

                let simd_intercept_result = &all_simd_results[idx][2];
                let regular_intercept_result = &regular_results[2];

                // Compare LINREG output lengths
                assert_eq!(
                    simd_linreg_result.len(),
                    regular_linreg_result.len(),
                    "LINREG output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
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

                // Compare LINREG values
                for (i, (&simd_val, &regular_val)) in simd_linreg_result
                    .iter()
                    .zip(regular_linreg_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by options LINREG has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options LINREG has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with appropriate epsilon for LINREG
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-9) {
                        panic!(
                            "LINREG mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
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
                    // Check for NaN/infinity in SIMD result
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

                    // Compare values with appropriate epsilon for slope
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-9) {
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
                    // Check for NaN/infinity in SIMD result
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

                    // Compare values with appropriate epsilon for intercept
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-9) {
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

        println!("✓ All SIMD by options vs Regular LINREG optional outputs database tests passed!");
    }
}
