#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::volatility::indicator_by_options;
    use tulip_rs::indicators::volatility::{
        indicator as rust_volatility, min_data, TIndicatorState,
    };
    use tulip_test::c_bindings::{ti_volatility, ti_volatility_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-8;
    const CHUNK_SIZE: usize = 100;
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
    fn test_volatility_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C Volatility function
            let start_index = unsafe { ti_volatility_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_volatility_start returned a negative index"
            );
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut volatility_output_vec_c = vec![0.0_f64; output_len_c];
            let volatility_ptr: *mut f64 = volatility_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![volatility_ptr];
            let ret = unsafe {
                ti_volatility(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_volatility returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) = rust_volatility(&inputs_rust, &options, None)
                .expect("Rust Volatility indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in volatility_output_vec_c
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
                        "Rust VOLATILITY has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust VOLATILITY has infinity at index {}: Rust = {}, Options = {:?}",
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
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}",
                        index, volatility_output_vec_c, outputs[0], options
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
    fn test_volatility_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_volatility_start(options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_volatility_start returned a negative index"
                );
                let output_len_c = close.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_volatility(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_volatility returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) = rust_volatility(&inputs_rust, &options, None)
                    .expect("Rust VOLATILITY indicator failed");

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
                            "Rust VOLATILITY has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust VOLATILITY has infinity at index {}: Rust = {}, Options = {:?}",
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
    fn test_volatility_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) = rust_volatility(&inputs_rust, &options, None)
                    .expect("Rust VOLATILITY indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) = rust_volatility(&chunk_inputs, &options, None)
                    .expect("Rust VOLATILITY indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VOLATILITY batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VOLATILITY batch indicator failed");
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
    fn test_volatility_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::volatility::indicator_by_assets;

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
                .expect("SIMD by assets VOLATILITY indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) = rust_volatility(&stock_inputs, &options, None)
                    .expect("Regular VOLATILITY indicator failed");

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
                            "SIMD by assets VOLATILITY has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets VOLATILITY has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for VOLATILITY
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

        println!("✓ All SIMD by assets vs Regular VOLATILITY database tests passed!");
    }

    #[test]
    fn test_volatility_simd_by_options_vs_regular_database() {
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
                .expect("SIMD VOLATILITY 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD VOLATILITY 2-wide failed");

            // Combine SIMD results
            let mut all_simd_results = Vec::new();

            // Add 4-wide results
            for result in &simd_results_4 {
                all_simd_results.push(result.clone());
            }

            // Add 2-wide results
            for result in &simd_results_2 {
                all_simd_results.push(result.clone());
            }

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) = rust_volatility(&inputs, options, None)
                    .expect("Regular VOLATILITY indicator failed");

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
                            "SIMD by options VOLATILITY has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options VOLATILITY has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for VOLATILITY
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Mismatch at index {} for stock {} with options {:?}: SIMD by options = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                println!(
                    "✓ SIMD by options vs Regular test passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All SIMD by options vs Regular VOLATILITY database tests passed!");
    }

    #[test]
    fn test_volatility_simd_state_handover_by_options() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // number of bars to process with SIMD first
        let first_bars = 2000usize;

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let total_len = close.len();
            if total_len == 0 {
                continue;
            }

            let split = first_bars.min(total_len);

            // prepare slices for first part and remaining
            let first_inputs = [&close[..split]];
            let remaining_inputs = if split < total_len {
                Some([&close[split..]])
            } else {
                None
            };

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, states_4) =
                indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD VOLATILITY 4-wide failed on first chunk");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, states_2) =
                indicator_by_options::<2>(&first_inputs, &options_2, None)
                    .expect("SIMD VOLATILITY 2-wide failed on first chunk");

            // Combine SIMD results for first part and prepare to extend with batch_indicator outputs
            let mut all_simd_results: Vec<Vec<f64>> = Vec::new();
            for result in &simd_results_4 {
                all_simd_results.push(result[0].clone());
            }
            for result in &simd_results_2 {
                all_simd_results.push(result[0].clone());
            }

            // If there is remaining data, use the returned states to process it
            if let Some(rem_inputs) = remaining_inputs {
                // states_4 are Vec<IndicatorState>
                for (i, mut st) in states_4.into_iter().enumerate() {
                    let chunk_out = st.batch_indicator(&rem_inputs, None).expect("batch failed");
                    all_simd_results[i].extend_from_slice(&chunk_out[0]);
                }

                for (i, mut st) in states_2.into_iter().enumerate() {
                    let chunk_out = st.batch_indicator(&rem_inputs, None).expect("batch failed");
                    all_simd_results[i + 4].extend_from_slice(&chunk_out[0]);
                }
            }

            // Compare each SIMD result with regular indicator over the full data
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) = rust_volatility(&[close.as_slice()], options, None)
                    .expect("Regular VOLATILITY indicator failed");
                let regular = &regular_results[0];
                let simd_res = &all_simd_results[idx];

                assert_eq!(
                    regular.len(),
                    simd_res.len(),
                    "Length mismatch for stock {} option {:?}",
                    stock_symbol,
                    options
                );

                for (k, (&r, &s)) in regular.iter().zip(simd_res.iter()).enumerate() {
                    if r.is_nan() && s.is_nan() {
                        continue;
                    }
                    if !approx_eq!(f64, r, s, epsilon = EPSILON) {
                        panic!(
                            "Mismatch stock {} option {:?} index {}: regular = {}, simd = {}",
                            stock_symbol, options, k, r, s
                        );
                    }
                }
            }
        }

        println!("✓ All VOLATILITY SIMD state handover by options tests passed!");
    }
}
