#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::psar::{
        indicator as rust_psar, indicator_by_assets, indicator_by_options, min_data,
        TIndicatorState,
    };
    use tulip_test::c_bindings::{ti_psar, ti_psar_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-12;
    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];

    const OPTIONS_LIST: [[f64; 2]; 4] = [[0.2, 2.0], [0.02, 0.2], [0.4, 4.0], [0.04, 0.4]];
    //const OPTIONS_LIST: [[f64; 2]; 2] = [[0.2, 2.0], [0.02, 0.2]];

    const CHUNK_SIZE: usize = 100;

    fn get_hl_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        (high, low)
    }

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
        }
        (high_vec, low_vec)
    }

    #[test]
    fn test_psar_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            // Determine the offset required by the C PSAR function
            let start_index = unsafe { ti_psar_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_psar_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut psar_output_vec_c = vec![0.0_f64; output_len_c];
            let psar_ptr: *mut f64 = psar_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![psar_ptr];
            let ret = unsafe {
                ti_psar(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_psar returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (outputs, _) =
                rust_psar(&inputs_rust, &options, None).expect("Rust PSAR indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in psar_output_vec_c
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
                        "Rust PSAR has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust PSAR has infinity at index {}: Rust = {}, Options = {:?}",
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
                        index, psar_output_vec_c, outputs[0], options
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
    fn test_psar_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

                let start_index = unsafe { ti_psar_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_psar_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_psar(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_psar returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice()];
                let (outputs, _) =
                    rust_psar(&inputs_rust, &options, None).expect("Rust PSAR indicator failed");

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
                            "Rust PSAR has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust PSAR has infinity at index {}: Rust = {}, Options = {:?}",
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
    fn test_psar_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = rust_psar(&inputs_rust, &options, None)
                    .expect("PSAR indicator should work on full data");

                // Process in batches
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // Process first chunk to get initial state
                let first_chunk_size = min_data_val.min(high.len());
                let first_high = high[..first_chunk_size].to_vec();
                let first_low = low[..first_chunk_size].to_vec();
                let first_inputs = [first_high.as_slice(), first_low.as_slice()];

                let (outputs, mut state) = rust_psar(&first_inputs, &options, None)
                    .expect("PSAR indicator should work on first chunk");

                for output_idx in 0..outputs.len() {
                    batch_full_outputs[output_idx].extend_from_slice(&outputs[output_idx]);
                }

                let mut processed = first_chunk_size;

                // Process subsequent chunks using state.batch_indicator
                while processed < high.len() {
                    let end = (processed + CHUNK_SIZE).min(high.len());

                    let chunk_high = high[processed..end].to_vec();
                    let chunk_low = low[processed..end].to_vec();
                    let chunk_inputs = [chunk_high.as_slice(), chunk_low.as_slice()];

                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("PSAR batch indicator failed");

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
    fn test_psar_simd_by_assets_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, (Vec<f64>, Vec<f64>))> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low) = get_hl_arrays(data);
                (symbol.clone(), (high, low))
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 2]; 4] = [
            &[&stock_data[0].1 .0, &stock_data[0].1 .1], // high, low
            &[&stock_data[1].1 .0, &stock_data[1].1 .1], // high, low
            &[&stock_data[2].1 .0, &stock_data[2].1 .1], // high, low
            &[&stock_data[3].1 .0, &stock_data[3].1 .1], // high, low
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets PSAR indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, (stock_high, stock_low))) in
                stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_high.as_slice(), stock_low.as_slice()];
                let (regular_results, _) = rust_psar(&stock_inputs, &options, None)
                    .expect("Regular PSAR indicator failed");

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
                                "SIMD by assets PSAR has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                                "SIMD by assets PSAR has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                    }

                    // Compare values exactly
                    assert_eq!(
                        simd_val, regular_val,
                        "Mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                        i, stock_symbol, options, simd_val, regular_val
                    );
                }

                println!(
                    "✓ SIMD by assets vs Regular test passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All SIMD by assets vs Regular PSAR database tests passed!");
    }

    #[test]
    fn test_psar_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice()];

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD PSAR 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_psar(&inputs, options, None).expect("Regular PSAR indicator failed");

                let simd_result = &all_simd_results[idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "PSAR output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
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
                            "SIMD PSAR has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD PSAR has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values exactly for PSAR
                    assert_eq!(
                        simd_val, regular_val,
                        "PSAR mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                        i, stock_symbol, options, simd_val, regular_val
                    );
                }
            }
        }

        println!("✓ All SIMD by options vs Regular PSAR database tests passed!");
    }

}
