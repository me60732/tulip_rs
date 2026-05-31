#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::md::{indicator as rust_md, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_md, ti_md_start, ti_sma, ti_sma_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILON: f64 = 1e-10;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [10.0], [14.0], [20.0]];

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
    fn test_md_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C MD function
            let start_index = unsafe { ti_md_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_md_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut md_output_vec_c = vec![0.0_f64; output_len_c];
            let md_ptr: *mut f64 = md_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![md_ptr];
            let ret = unsafe {
                ti_md(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_md returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_md(&inputs_rust, &options, None).expect("Rust MD indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in md_output_vec_c
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
                        "Rust MD has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust MD has infinity at index {}: Rust = {}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-12) {
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, md_output_vec_c, outputs[0], options
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
    fn test_md_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_md_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_md_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut md_output_vec_c = vec![0.0_f64; output_len_c];
                let md_ptr: *mut f64 = md_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![md_ptr];
                let ret = unsafe {
                    ti_md(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_md returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_md(&inputs_rust, &options, None).expect("Rust MD indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare results
                for (i, (&c_val, &rust_val)) in md_output_vec_c
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
                            "Rust MD has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust MD has infinity at index {}: Rust = {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-12) {
                        println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, md_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_md_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs_rust = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = rust_md(&inputs_rust, &options, None)
                    .expect("MD indicator should work on full data");

                // Process in batches
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // Process first chunk to get initial state
                let first_chunk_size = min_data_val.min(close.len());
                let first_close = close[..first_chunk_size].to_vec();
                let first_inputs = [first_close.as_slice()];

                let (outputs, mut state) = rust_md(&first_inputs, &options, None)
                    .expect("MD indicator should work on first chunk");

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
                        .expect("MD batch indicator failed");

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
    fn test_md_simd_vs_regular_database() {
        use tulip_rs::indicators::md::indicator_by_assets;

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
                    .expect("SIMD by assets MD indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = rust_md(&stock_inputs, &options, None)
                        .expect("Regular MD indicator failed");

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
                                "SIMD by assets MD has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets MD has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for MD
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

        println!("✓ All SIMD by assets vs Regular MD database tests passed!");
    }

    #[test]
    fn test_md_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::md::indicator_by_assets;

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
            // Test with optional outputs enabled
            {
                // Get SIMD by assets result with optional SMA output
                let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, Some(&[true]))
                    .expect("SIMD by assets MD indicator with optional output failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock with optional output
                    let stock_inputs = [stock_close.as_slice()];
                    let (regular_results, _) = rust_md(&stock_inputs, &options, Some(&[true]))
                        .expect("Regular MD indicator with optional output failed");

                    // Compare MD output (index 0)
                    let simd_md_result = &simd_results[stock_idx][0];
                    let regular_md_result = &regular_results[0];

                    assert_eq!(
                        simd_md_result.len(),
                        regular_md_result.len(),
                        "MD output length mismatch for stock {} with options {:?}",
                        stock_symbol,
                        options
                    );

                    for (i, (&simd_val, &regular_val)) in simd_md_result
                        .iter()
                        .zip(regular_md_result.iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "MD mismatch at index {} for stock {} with options {:?}: SIMD = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    // Compare SMA output (index 1)
                    let simd_sma_result = &simd_results[stock_idx][1];
                    let regular_sma_result = &regular_results[1];

                    assert_eq!(
                        simd_sma_result.len(),
                        regular_sma_result.len(),
                        "SMA output length mismatch for stock {} with options {:?}",
                        stock_symbol,
                        options
                    );

                    for (i, (&simd_val, &regular_val)) in simd_sma_result
                        .iter()
                        .zip(regular_sma_result.iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "SMA mismatch at index {} for stock {} with options {:?}: SIMD = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    println!(
                        "✓ SIMD by assets vs Regular test (with optional outputs) passed for stock {} with options {:?}",
                        stock_symbol, options
                    );
                }
            }
        }

        println!("✓ All SIMD by assets vs Regular MD database tests with optional outputs passed!");
    }

    #[test]
    fn test_md_sma_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-10;

        let close = expand_close();
        let inputs = [close.as_slice()];
        let options = [5.0]; // period = 5
        let optional_outputs = Some([true].as_slice()); // Request sma output

        // Get Rust MD output with sma optional output
        let result = rust_md(&inputs, &options, optional_outputs).unwrap();
        let rust_sma = &result.0[1]; // sma is at index 1

        // Fail fast if Rust output is empty
        if rust_sma.is_empty() {
            panic!("Rust MD sma optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip SMA output for comparison
        let sma_inputs_c: Vec<*const f64> = vec![close.as_ptr()];
        let sma_start_index = unsafe { ti_sma_start(options.as_ptr()) };
        let sma_output_len = close.len() - (sma_start_index as usize);
        let mut c_sma = vec![0.0; sma_output_len];
        let mut sma_outputs_c = vec![c_sma.as_mut_ptr()];

        let ret = unsafe {
            ti_sma(
                close.len() as i32,
                sma_inputs_c.as_ptr(),
                options.as_ptr(),
                sma_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_sma returned error code {}", ret);

        // Compare SMA outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing MD sma optional output vs C Tulip SMA:");
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
            if c_val.is_infinite() || c_val.is_nan() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > EPSILON {
                panic!(
                    "MD sma mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ MD sma optional output matches C Tulip SMA output");
    }

    #[test]
    fn test_md_database_optional_sma() {
        const EPSILON: f64 = 1e-10;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get MD with SMA optional output
                let optional_outputs = Some(&[true][..]);
                let (md_result, _) =
                    tulip_rs::indicators::md::indicator(&[&close], &[options[0]], optional_outputs)
                        .unwrap();

                let rust_sma = &md_result[1];

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
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "MD SMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_md_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::md::indicator_by_options;

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
            let (all_simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD MD 4-wide failed");

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_md(&inputs, options, None).expect("Regular MD indicator failed");

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
                            "SIMD MD has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD MD has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
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

        println!("✓ All SIMD by options vs Regular MD database tests passed!");
    }

    #[test]
    fn test_md_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::md::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];
            let optional_outputs = Some(&[true][..]);

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (all_simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD MD 4-wide failed");

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) = rust_md(&inputs, options, optional_outputs)
                    .expect("Regular MD indicator failed");

                let simd_result = &all_simd_results[idx];
                let regular_result = &regular_results;

                // Should have 2 outputs: MD and SMA
                assert_eq!(
                    simd_result.len(),
                    2,
                    "SIMD result should have 2 outputs (MD and SMA)"
                );
                assert_eq!(
                    regular_result.len(),
                    2,
                    "Regular result should have 2 outputs (MD and SMA)"
                );

                // Compare MD output (index 0)
                let simd_md = &simd_result[0];
                let regular_md = &regular_result[0];

                assert_eq!(
                    simd_md.len(),
                    regular_md.len(),
                    "MD output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_md.len(),
                    regular_md.len()
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_md.iter().zip(regular_md.iter()).enumerate()
                {
                    if simd_val.is_nan() || simd_val.is_infinite() {
                        panic!(
                            "SIMD MD has NaN/infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "MD mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare SMA output (index 1)
                let simd_sma = &simd_result[1];
                let regular_sma = &regular_result[1];

                assert_eq!(
                    simd_sma.len(),
                    regular_sma.len(),
                    "SMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_sma.len(),
                    regular_sma.len()
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_sma.iter().zip(regular_sma.iter()).enumerate()
                {
                    if simd_val.is_nan() || simd_val.is_infinite() {
                        panic!(
                            "SIMD SMA has NaN/infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "SMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular MD database tests with optional outputs passed!"
        );
    }
}
