#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::roc::{indicator as rust_roc, min_data, TIndicatorState};
    use tulip_rs::indicators::roc::{indicator_by_assets, indicator_by_options};
    use tulip_test::c_bindings::{ti_mom, ti_mom_start, ti_roc, ti_roc_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 8] = [
        [5.0],
        [10.0],
        [14.0],
        [20.0],
        [25.0],
        [30.0],
        [50.0],
        [100.0],
    ];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_close() -> Vec<f64> {
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..300 {
            close_vec.extend_from_slice(&CLOSE);
        }
        close_vec
    }

    #[test]
    fn test_roc_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C ROC function
            let start_index = unsafe { ti_roc_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_roc_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut roc_output_vec_c = vec![0.0_f64; output_len_c];
            let roc_ptr: *mut f64 = roc_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![roc_ptr];
            let ret = unsafe {
                ti_roc(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_roc returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_roc(&inputs_rust, &options, None).expect("Rust ROC indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in roc_output_vec_c
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
                        "Rust ROC has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust ROC has infinity at index {}: Rust = {}, Options = {:?}",
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
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}",
                        index, roc_output_vec_c, outputs[0], options
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
    fn test_roc_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);

            for options in OPTIONS_LIST {
                // run c code
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                // Determine the offset required by the C ROC function
                let start_index = unsafe { ti_roc_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_roc_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                // Run the C implementation
                let mut roc_output_vec_c = vec![0.0_f64; output_len_c];
                let roc_ptr: *mut f64 = roc_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![roc_ptr];
                let ret = unsafe {
                    ti_roc(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_roc returned error code {}", ret);

                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_roc(&inputs_rust, &options, None).expect("Rust ROC indicator failed");

                let output_len_rust = outputs[0].len();

                for (i, (&c_val, &rust_val)) in roc_output_vec_c
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
                            "Rust ROC has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust ROC has infinity at index {}: Rust = {}, Options = {:?}",
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
                            index, roc_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_roc_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_roc(&inputs_rust, &options, None).expect("Rust ROC indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) =
                    rust_roc(&chunk_inputs, &options, None).expect("Rust ROC indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("ROC batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("ROC batch indicator failed");
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
    fn test_roc_simd_by_assets() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks for SIMD testing
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), data.iter().map(|d| d.close).collect()))
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            // Run SIMD by assets implementation
            let (simd_outputs, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets ROC indicator failed");

            // Compare with individual Rust implementations
            for i in 0..4 {
                let individual_inputs = [stock_data[i].1.as_slice()];
                let (individual_outputs, _) = rust_roc(&individual_inputs, &options, None)
                    .expect("Individual Rust ROC indicator failed");

                // Compare outputs
                assert_eq!(
                    simd_outputs[i][0].len(),
                    individual_outputs[0].len(),
                    "Output lengths don't match for stock {} with options {:?}",
                    stock_data[i].0,
                    options
                );

                for (j, (&simd_val, &individual_val)) in simd_outputs[i][0]
                    .iter()
                    .zip(individual_outputs[0].iter())
                    .enumerate()
                {
                    // Check for NaN or infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD ROC has NaN at index {}: SIMD = {}, Options = {:?}, Stock: {}",
                            j, simd_val, options, stock_data[i].0
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD ROC has infinity at index {}: SIMD = {}, Options = {:?}, Stock: {}",
                            j, simd_val, options, stock_data[i].0
                        );
                    }

                    if !approx_eq!(f64, simd_val, individual_val, epsilon = 1e-12) {
                        panic!(
                            "SIMD vs Individual mismatch at index {} for stock {} with options {:?}: SIMD = {}, Individual = {}",
                            j, stock_data[i].0, options, simd_val, individual_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_roc_simd_by_assets_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks for SIMD testing
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), data.iter().map(|d| d.close).collect()))
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            // Test with optional outputs (ROC has ROC and MOM outputs)
            let (simd_results_opt, _) =
                indicator_by_assets::<4>(&inputs, &options, Some(&[true, true]))
                    .expect("SIMD by assets ROC indicator with optional outputs failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock with optional outputs
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results_opt, _) =
                    rust_roc(&stock_inputs, &options, Some(&[true, true]))
                        .expect("Regular ROC indicator with optional outputs failed");

                // Compare all outputs: ROC, MOM
                let output_names = ["ROC", "MOM"];
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
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-10) {
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

        println!("✓ All SIMD by assets vs Regular ROC optional outputs database tests passed!");
    }
    #[test]
    fn test_roc_mom_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let close = expand_close();
        let inputs = [close.as_slice()];
        let options = [10.0]; // period = 10
        let optional_outputs = Some([true].as_slice()); // Request mom output

        // Get Rust ROC output with mom optional output
        let result = rust_roc(&inputs, &options, optional_outputs).unwrap();
        let rust_mom = &result.0[1]; // mom is at index 1

        // Fail fast if Rust output is empty
        if rust_mom.is_empty() {
            panic!("Rust ROC mom optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip MOM output for comparison
        let mom_inputs_c: Vec<*const f64> = vec![close.as_ptr()];
        let mom_start_index = unsafe { ti_mom_start(options.as_ptr()) };
        let mom_output_len = close.len() - (mom_start_index as usize);
        let mut c_mom = vec![0.0; mom_output_len];
        let mut mom_outputs_c = vec![c_mom.as_mut_ptr()];

        let ret = unsafe {
            ti_mom(
                close.len() as i32,
                mom_inputs_c.as_ptr(),
                options.as_ptr(),
                mom_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_mom returned error code {}", ret);

        // Compare MOM outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing ROC mom optional output vs C Tulip MOM:");
        println!(
            "Rust mom length: {}, C MOM length: {}",
            rust_mom.len(),
            c_mom.len()
        );

        for (i, (rust_val, c_val)) in rust_mom.iter().rev().zip(c_mom.iter().rev()).enumerate() {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust mom output contains NaN/infinity at position {}: {}",
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
                    "ROC mom mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ ROC mom optional output matches C Tulip MOM output");
    }

    #[test]
    fn test_roc_database_optional_mom() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get ROC with mom optional output
                let optional_outputs = Some(&[true][..]);
                let (roc_result, _) = tulip_rs::indicators::roc::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_mom = &roc_result[1];

                // Calculate expected mom using C Tulip ti_mom
                let mom_options = vec![options[0]];
                let start_index = unsafe { ti_mom_start(mom_options.as_ptr()) };
                assert!(start_index >= 0, "ti_mom_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_mom_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_mom_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_mom(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        mom_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_mom failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_mom.len().min(c_mom_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_mom.len() - 1 - i;
                    let c_idx = c_mom_output.len() - 1 - i;

                    let rust_val = rust_mom[rust_idx];
                    let c_val = c_mom_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust mom output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "ROC mom optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_roc_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let inputs = [close.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD ROC 4-wide failed");

            // Process remaining 4 options with 4-wide SIMD
            let options_4_second = [
                &OPTIONS_LIST[4],
                &OPTIONS_LIST[5],
                &OPTIONS_LIST[6],
                &OPTIONS_LIST[7],
            ];
            let (simd_results_4_second, _) =
                indicator_by_options::<4>(&inputs, &options_4_second, None)
                    .expect("SIMD ROC 4-wide second failed");

            // Combine SIMD results
            let mut all_simd_results = Vec::new();
            for i in 0..4 {
                all_simd_results.push(simd_results_4[i].clone());
            }
            for i in 0..4 {
                all_simd_results.push(simd_results_4_second[i].clone());
            }

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_roc(&inputs, options, None).expect("Regular ROC indicator failed");

                let simd_result = &all_simd_results[idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "ROC output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
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
                            "SIMD ROC has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD ROC has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        panic!(
                            "ROC mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }
    }
}
