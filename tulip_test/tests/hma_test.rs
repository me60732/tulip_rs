#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::hma::{indicator as rust_hma, min_data, TIndicatorState};
    use tulip_rs::indicators::hma::{indicator_by_assets, indicator_by_options};
    use tulip_test::c_bindings::{ti_hma, ti_hma_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [50.0]];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_close() -> Vec<f64> {
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..10 {
            close_vec.extend_from_slice(&CLOSE);
        }
        close_vec
    }

    #[test]
    fn test_hma_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C HMA function
            let start_index = unsafe { ti_hma_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_hma_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut hma_output_vec_c = vec![0.0_f64; output_len_c];
            let hma_ptr: *mut f64 = hma_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![hma_ptr];
            let ret = unsafe {
                ti_hma(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_hma returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_hma(&inputs_rust, &options, None).expect("Rust HMA indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in hma_output_vec_c
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
                        "Rust HMA has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust HMA has infinity at index {}: Rust = {}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-8) {
                    // Adjust epsilon if needed
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, hma_output_vec_c, outputs[0], options
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
    fn test_hma_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // run c code
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                // Determine the offset required by the C HMA function
                let start_index = unsafe { ti_hma_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_hma_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                // Run the C implementation
                let mut hma_output_vec_c = vec![0.0_f64; output_len_c];
                let hma_ptr: *mut f64 = hma_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![hma_ptr];
                let ret = unsafe {
                    ti_hma(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_hma returned error code {}", ret);

                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_hma(&inputs_rust, &options, None).expect("Rust HMA indicator failed");

                let output_len_rust = outputs[0].len();

                for (i, (&c_val, &rust_val)) in hma_output_vec_c
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
                            "Rust HMA has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust HMA has infinity at index {}: Rust = {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-8) {
                        println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, hma_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_hma_simd_by_assets_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' close data
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

        let mut all_failures = Vec::new();

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = match indicator_by_assets::<4>(&inputs, &options, None) {
                Ok(r) => r,
                Err(e) => {
                    all_failures.push(format!(
                        "SIMD by assets failed for options {:?}: {}",
                        options, e
                    ));
                    continue;
                }
            };

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) = match rust_hma(&stock_inputs, &options, None) {
                    Ok(r) => r,
                    Err(e) => {
                        all_failures.push(format!(
                            "Regular HMA failed for {} options {:?}: {}",
                            stock_symbol, options, e
                        ));
                        continue;
                    }
                };

                let simd_result = &simd_results[stock_idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                if simd_result.len() != regular_result.len() {
                    all_failures.push(format!(
                        "Output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                        stock_symbol,
                        options,
                        simd_result.len(),
                        regular_result.len()
                    ));
                    continue;
                }

                // Compare each value and collect failures
                let mut mismatches = Vec::new();
                for (i, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        mismatches.push(format!("NaN at index {}: SIMD = {}", i, simd_val));
                        continue;
                    }

                    if simd_val.is_infinite() {
                        mismatches.push(format!("Infinity at index {}: SIMD = {}", i, simd_val));
                        continue;
                    }

                    // Compare values with high precision
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        mismatches.push(format!(
                            "Index {}: SIMD = {}, Regular = {}, Diff = {}",
                            i,
                            simd_val,
                            regular_val,
                            (simd_val - regular_val).abs()
                        ));
                    }
                }

                if !mismatches.is_empty() {
                    all_failures.push(format!(
                        "Stock {} options {:?}: {} mismatches - First few: {}",
                        stock_symbol,
                        options,
                        mismatches.len(),
                        mismatches
                            .iter()
                            .take(3)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("; ")
                    ));
                }
            }
        }

        // Report all failures at once
        if !all_failures.is_empty() {
            println!("\n=== SIMD vs Regular HMA Database Test Failures ===");
            for (i, failure) in all_failures.iter().enumerate() {
                println!("\n{}. {}", i + 1, failure);
            }
            panic!(
                "\n\nFound {} total failures in SIMD vs Regular comparison",
                all_failures.len()
            );
        }
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    #[test]
    fn test_hma_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs_rust = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = rust_hma(&inputs_rust, &options, None)
                    .expect("Failed to run HMA indicator on full data");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) = rust_hma(&chunk_inputs, &options, None)
                    .expect("Failed to run HMA indicator on first chunk");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks using state
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("HMA batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Process remainder if any
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("HMA batch indicator failed");
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
                        "Mismatch in HMA output at index {}: full = {}, batch = {}, Stock: {}, Options: {:?}",
                        i, full_val, batch_val, stock_symbol, options
                    );
                }
            }
        }
    }
    #[test]
    fn test_hma_simd_by_assets_vs_regular_close() {
        // Create variable length data using CLOSE const
        // Stock 1: base length
        let stock1_close = expand_close();

        // Stock 2: base length * 2
        let mut stock2_close = expand_close();
        stock2_close.extend(&CLOSE);

        // Stock 3: base length * 3
        let mut stock3_close = expand_close();
        stock3_close.extend(&CLOSE);
        stock3_close.extend(&CLOSE);

        // Stock 4: base length * 4
        let mut stock4_close = expand_close();
        stock4_close.extend(&CLOSE);
        stock4_close.extend(&CLOSE);
        stock4_close.extend(&CLOSE);

        let stocks = [("STOCK1", stock1_close.as_slice()),
            ("STOCK2", stock2_close.as_slice()),
            ("STOCK3", stock3_close.as_slice()),
            ("STOCK4", stock4_close.as_slice())];

        // Prepare inputs for SIMD processing - we need 4 assets for SIMD width of 4
        let simd_inputs = [
            &[stock1_close.as_slice()], // Stock 1 inputs
            &[stock2_close.as_slice()], // Stock 2 inputs
            &[stock3_close.as_slice()], // Stock 3 inputs
            &[stock4_close.as_slice()], // Stock 4 inputs
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&simd_inputs, &options, None)
                .expect("SIMD by assets HMA indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stocks.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [*stock_close];
                let (regular_results, _) =
                    rust_hma(&stock_inputs, &options, None).expect("Regular HMA indicator failed");

                let simd_result = &simd_results[stock_idx][0];
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
                            "SIMD by assets HMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets HMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with high precision
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        //println!("SIMD: {:?} \n\nRegular: {:?}", simd_result, regular_result);
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_hma_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            if close.is_empty() {
                continue;
            }

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];

            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD HMA by options failed");

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    rust_hma(&inputs, options, None).expect("Regular HMA indicator failed");
                let regular = &regular_results[0];
                let simd = &simd_results_4[idx][0];

                assert_eq!(
                    regular.len(),
                    simd.len(),
                    "Length mismatch for stock {} options {:?}",
                    stock_symbol,
                    options
                );

                for (i, (&regular_val, &simd_val)) in regular.iter().zip(simd.iter()).enumerate() {
                    if regular_val.is_nan() && simd_val.is_nan() {
                        continue;
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular HMA database tests passed!");
    }

    #[test]
    fn test_hma_simd_state_handover_by_options() {
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

            // process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, states_4) =
                indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD HMA 4-wide failed on first chunk");

            // Combine SIMD results for first part and prepare to extend with batch_indicator outputs
            let mut all_simd_results: Vec<Vec<f64>> = Vec::new();
            for result in &simd_results_4 {
                all_simd_results.push(result[0].clone());
            }

            // If there is remaining data, use the returned states to process it
            if let Some(rem_inputs) = remaining_inputs {
                // states_4 are Vec<IndicatorState>
                for (i, mut st) in states_4.into_iter().enumerate() {
                    let chunk_out = st.batch_indicator(&rem_inputs, None).expect("batch failed");
                    all_simd_results[i].extend_from_slice(&chunk_out[0]);
                }
            }

            // Compare each SIMD result with regular indicator over the full data
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) = rust_hma(&[close.as_slice()], options, None)
                    .expect("Regular HMA indicator failed");
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
                    if !approx_eq!(f64, r, s, epsilon = 1e-12) {
                        panic!(
                            "Mismatch stock {} option {:?} index {}: regular = {}, simd = {}",
                            stock_symbol, options, k, r, s
                        );
                    }
                }
            }
        }

        println!("✓ All HMA SIMD state handover by options tests passed!");
    }
}
