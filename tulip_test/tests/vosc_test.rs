#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::vosc::{indicator as rust_vosc, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_sma, ti_sma_start, ti_vosc, ti_vosc_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const VOLUME: [f64; 15] = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];

    const OPTIONS_LIST: [[f64; 2]; 4] = [[2.0, 5.0], [5.0, 20.0], [10.0, 25.0], [14.0, 28.0]];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_volume() -> Vec<f64> {
        let mut volume_vec = VOLUME.to_vec();
        for _ in 0..3 {
            volume_vec.extend_from_slice(&VOLUME);
        }
        volume_vec
    }

    #[test]
    fn test_vosc_indicator() {
        // Use the same input data as in the benchmarks
        let volume = expand_volume();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![volume.as_ptr()];

            // Determine the offset required by the C VOSC function
            let start_index = unsafe { ti_vosc_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_vosc_start returned a negative index");
            let output_len_c = volume.len() - (start_index as usize);

            // Run the C implementation
            let mut vosc_output_vec_c = vec![0.0_f64; output_len_c];
            let vosc_ptr: *mut f64 = vosc_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![vosc_ptr];
            let ret = unsafe {
                ti_vosc(
                    volume.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_vosc returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [volume.as_slice()];
            let (outputs, _) =
                rust_vosc(&inputs_rust, &options, None).expect("Rust VOSC indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in vosc_output_vec_c
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
                        "Rust VOSC has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust VOSC has infinity at index {}: Rust = {}, Options = {:?}",
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
                        index, vosc_output_vec_c, outputs[0], options
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
    fn test_vosc_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let volume = get_volume_array(&stock_data);
            /*for (i, &v) in volume.iter().enumerate() {
                if v == 0.0 {
                    panic!(
                        "Rust VOSC 0.0 in volume at index {}: volume = {}, Stock: {}",
                        i, v, stock_symbol
                    );
                }
            }*/
            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![volume.as_ptr()];

                let start_index = unsafe { ti_vosc_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_vosc_start returned a negative index");
                let output_len_c = volume.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_vosc(
                        volume.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_vosc returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [volume.as_slice()];
                let (outputs, _) =
                    rust_vosc(&inputs_rust, &options, None).expect("Rust VOSC indicator failed");

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
                            "Rust VOSC has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust VOSC has infinity at index {}: Rust = {}, Options = {:?}",
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
    fn test_vosc_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let volume = get_volume_array(&stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [volume.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_vosc(&inputs_rust, &options, None).expect("Rust VOSC indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let volume_vec = volume[..min_data_val].to_vec();
                let chunk_inputs = [volume_vec.as_slice()];

                let (first_outputs, mut state) =
                    rust_vosc(&chunk_inputs, &options, None).expect("Rust VOSC indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut volume_chunks = volume[min_data_val..].chunks_exact(CHUNK_SIZE);

                for volume_chunk in volume_chunks.by_ref() {
                    let volume_vec = volume_chunk.to_vec();
                    let chunk_inputs = [volume_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VOSC batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let volume_rem = volume_chunks.remainder();
                if !volume_rem.is_empty() {
                    let volume_vec = volume_rem.to_vec();
                    let chunk_inputs = [volume_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VOSC batch indicator failed");
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

    fn get_volume_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.volume).collect()
    }

    #[test]
    fn test_vosc_simd_vs_regular_database() {
        use tulip_rs::indicators::vosc::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let volume = get_volume_array(data);
                (symbol.clone(), volume)
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
                .expect("SIMD by assets VOSC indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_volume)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_volume.as_slice()];
                let (regular_results, _) = rust_vosc(&stock_inputs, &options, None)
                    .expect("Regular VOSC indicator failed");

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
                            "SIMD by assets VOSC has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets VOSC has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for VOSC
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
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

        println!("✓ All SIMD by assets vs Regular VOSC database tests passed!");
    }

    #[test]
    fn test_vosc_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::vosc::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let volume = get_volume_array(data);
                (symbol.clone(), volume)
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
            let optional_flags = [true, true]; // All 2 optional outputs

            // Get SIMD by assets result with optional outputs
            let (simd_results, _) =
                indicator_by_assets::<4>(&inputs, &options, Some(&optional_flags))
                    .expect("SIMD by assets VOSC indicator with optional outputs failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_volume)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock with optional outputs
                let stock_inputs = [stock_volume.as_slice()];
                let (regular_results, _) =
                    rust_vosc(&stock_inputs, &options, Some(&optional_flags))
                        .expect("Regular VOSC indicator with optional outputs failed");

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
                                "SIMD by assets VOSC has NaN in output {} at index {} for stock {} with options {:?}: SIMD = {}",
                                output_idx, i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets VOSC has infinity in output {} at index {} for stock {} with options {:?}: SIMD = {}",
                                output_idx, i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for VOSC
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
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
            "✓ All SIMD by assets vs Regular VOSC database tests with optional outputs passed!"
        );
    }
    const SMA_EPSILON: f64 = 1e-10; // Use epsilon from sma_test.rs

    #[test]
    fn test_vosc_short_sma_optional_output_vs_c_tulip() {
        let volume = expand_volume();
        let inputs = [volume.as_slice()];
        let short_period = 5.0;
        let long_period = 20.0;
        let options = [short_period, long_period];
        let optional_outputs = Some([true, false].as_slice()); // Request only short_sma output

        // Get Rust VOSC output with short_sma optional output
        let result = rust_vosc(&inputs, &options, optional_outputs).unwrap();
        let rust_short_sma = &result.0[1]; // short_sma is at index 1

        // Fail fast if Rust output is empty
        if rust_short_sma.is_empty() {
            panic!(
                "Rust VOSC short_sma optional output is empty - this indicates an indicator bug"
            );
        }

        // Get C Tulip SMA output for comparison
        let sma_inputs_c: Vec<*const f64> = vec![volume.as_ptr()];
        let short_options = [short_period];
        let sma_start_index = unsafe { ti_sma_start(short_options.as_ptr()) };
        let sma_output_len = volume.len() - (sma_start_index as usize);
        let mut c_short_sma = vec![0.0; sma_output_len];
        let mut sma_outputs_c = vec![c_short_sma.as_mut_ptr()];

        let ret = unsafe {
            ti_sma(
                volume.len() as i32,
                sma_inputs_c.as_ptr(),
                short_options.as_ptr(),
                sma_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_sma returned error code {}", ret);

        // Compare short SMA outputs from the end backwards (reverse order comparison)
        println!("Comparing VOSC short_sma optional output vs C Tulip SMA:");
        println!(
            "Rust short_sma length: {}, C SMA length: {}",
            rust_short_sma.len(),
            c_short_sma.len()
        );

        for (i, (rust_val, c_val)) in rust_short_sma
            .iter()
            .rev()
            .zip(c_short_sma.iter().rev())
            .enumerate()
        {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust short_sma output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            // Skip comparison if C output is NaN/infinite (assume C bug)
            if !c_val.is_finite() {
                println!(
                    "Skipping short_sma comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > SMA_EPSILON {
                panic!(
                    "VOSC short_sma mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ VOSC short_sma optional output matches C Tulip SMA output");
    }

    #[test]
    fn test_vosc_long_sma_optional_output_vs_c_tulip() {
        let volume = expand_volume();
        let inputs = [volume.as_slice()];
        let short_period = 5.0;
        let long_period = 20.0;
        let options = [short_period, long_period];
        let optional_outputs = Some([false, true].as_slice()); // Request only long_sma output

        // Get Rust VOSC output with long_sma optional output
        let result = rust_vosc(&inputs, &options, optional_outputs).unwrap();
        let rust_long_sma = &result.0[2]; // long_sma is at index 2

        // Fail fast if Rust output is empty
        if rust_long_sma.is_empty() {
            panic!("Rust VOSC long_sma optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip SMA output for comparison
        let sma_inputs_c: Vec<*const f64> = vec![volume.as_ptr()];
        let long_options = [long_period];
        let sma_start_index = unsafe { ti_sma_start(long_options.as_ptr()) };
        let sma_output_len = volume.len() - (sma_start_index as usize);
        let mut c_long_sma = vec![0.0; sma_output_len];
        let mut sma_outputs_c = vec![c_long_sma.as_mut_ptr()];

        let ret = unsafe {
            ti_sma(
                volume.len() as i32,
                sma_inputs_c.as_ptr(),
                long_options.as_ptr(),
                sma_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_sma returned error code {}", ret);

        // Compare long SMA outputs from the end backwards (reverse order comparison)
        println!("Comparing VOSC long_sma optional output vs C Tulip SMA:");
        println!(
            "Rust long_sma length: {}, C SMA length: {}",
            rust_long_sma.len(),
            c_long_sma.len()
        );

        for (i, (rust_val, c_val)) in rust_long_sma
            .iter()
            .rev()
            .zip(c_long_sma.iter().rev())
            .enumerate()
        {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust long_sma output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            // Skip comparison if C output is NaN/infinite (assume C bug)
            if !c_val.is_finite() {
                println!(
                    "Skipping long_sma comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > SMA_EPSILON {
                panic!(
                    "VOSC long_sma mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ VOSC long_sma optional output matches C Tulip SMA output");
    }

    #[test]
    fn test_vosc_database_optional_short_sma() {
        const SMA_EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let volume = get_volume_array(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get VOSC with short_sma optional output
                let optional_outputs = Some(&[true, false][..]);
                let (vosc_result, _) = tulip_rs::indicators::vosc::indicator(
                    &[&volume],
                    &[options[0], options[1]],
                    optional_outputs,
                )
                .unwrap();

                let rust_short_sma = &vosc_result[1];

                // Calculate expected short SMA using C Tulip ti_sma
                let short_sma_options = vec![options[0]]; // short period
                let start_index = unsafe { ti_sma_start(short_sma_options.as_ptr()) };
                assert!(start_index >= 0, "ti_sma_start returned a negative index");
                let output_len_c = volume.len() - (start_index as usize);

                let mut c_short_sma_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![volume.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_short_sma_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_sma(
                        volume.len() as i32,
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
                        "VOSC short SMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_vosc_database_optional_long_sma() {
        const SMA_EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let volume = get_volume_array(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get VOSC with long_sma optional output
                let optional_outputs = Some(&[false, true][..]);
                let (vosc_result, _) = tulip_rs::indicators::vosc::indicator(
                    &[&volume],
                    &[options[0], options[1]],
                    optional_outputs,
                )
                .unwrap();

                let rust_long_sma = &vosc_result[2];

                // Calculate expected long SMA using C Tulip ti_sma
                let long_sma_options = vec![options[1]]; // long period
                let start_index = unsafe { ti_sma_start(long_sma_options.as_ptr()) };
                assert!(start_index >= 0, "ti_sma_start returned a negative index");
                let output_len_c = volume.len() - (start_index as usize);

                let mut c_long_sma_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![volume.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_long_sma_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_sma(
                        volume.len() as i32,
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
                        "VOSC long SMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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

    // SIMD-by-options test for VOSC (4-wide SIMD, compare to regular)
    #[test]
    fn test_vosc_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::vosc::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let volume = get_volume_array(&stock_data);
            let inputs = [volume.as_slice()];

            // All 4 options at once with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD VOSC 4-wide failed");

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_vosc(&inputs, options, None).expect("Regular VOSC indicator failed");

                // main VOSC output
                let simd_result = &simd_results_4[idx][0];
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
                            "SIMD VOSC has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD VOSC has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = SMA_EPSILON) {
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

        println!("✓ All SIMD by options vs Regular VOSC database tests passed!");
    }
    // SIMD-by-options optional outputs test for VOSC (4-wide SIMD, compare to regular)
    #[test]
    fn test_vosc_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::vosc::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let volume = get_volume_array(&stock_data);
            let inputs = [volume.as_slice()];

            // Request all optional outputs: short_sma, long_sma
            let optional_outputs = Some(&[true, true][..]);

            // All 4 options at once with 4-wide SIMD (with optional outputs)
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD VOSC 4-wide with optional outputs failed");

            // Compare each SIMD result with regular indicator (with optional outputs)
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_vosc(&inputs, options, optional_outputs)
                    .expect("Regular VOSC indicator with optional outputs failed");

                // For VOSC the outputs are: [VOSC, short_sma, long_sma]
                let simd_vosc_result = &simd_results_4[idx][0];
                let regular_vosc_result = &regular_results[0];

                let simd_short_sma = &simd_results_4[idx][1];
                let regular_short_sma = &regular_results[1];

                let simd_long_sma = &simd_results_4[idx][2];
                let regular_long_sma = &regular_results[2];

                // Compare VOSC output lengths
                assert_eq!(
                    simd_vosc_result.len(),
                    regular_vosc_result.len(),
                    "VOSC output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_vosc_result.len(),
                    regular_vosc_result.len()
                );

                // Compare VOSC values
                for (i, (&simd_val, &regular_val)) in simd_vosc_result
                    .iter()
                    .zip(regular_vosc_result.iter())
                    .enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD VOSC has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD VOSC has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = SMA_EPSILON) {
                        panic!(
                            "VOSC mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare short_sma optional outputs with SMA_EPSILON
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

                // Compare long_sma optional outputs with SMA_EPSILON
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
            }

            println!(
                "✓ SIMD by options vs Regular optional outputs test passed for stock {}",
                stock_symbol
            );
        }

        println!("✓ All SIMD by options vs Regular VOSC optional outputs database tests passed!");
    }
}
