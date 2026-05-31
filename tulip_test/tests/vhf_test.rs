#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::vhf::{indicator as rust_vhf, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_vhf, ti_vhf_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 7] = [[5.0], [10.0], [14.0], [18.0], [25.0], [28.0], [30.0]];
    //const OPTIONS_LIST: [[f64; 1]; 1] = [[5.0]];
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
    fn test_vhf_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C VHF function
            let start_index = unsafe { ti_vhf_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_vhf_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut vhf_output_vec_c = vec![0.0_f64; output_len_c];
            let vhf_ptr: *mut f64 = vhf_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![vhf_ptr];
            let ret = unsafe {
                ti_vhf(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_vhf returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_vhf(&inputs_rust, &options, None).expect("Rust VHF indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in vhf_output_vec_c
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
                        "Rust VHF has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust VHF has infinity at index {}: Rust = {}, Options = {:?}",
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
                        index, vhf_output_vec_c, outputs[0], options
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
    fn test_vhf_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // run c code
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                // Determine the offset required by the C VHF function
                let start_index = unsafe { ti_vhf_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_vhf_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                // Run the C implementation
                let mut vhf_output_vec_c = vec![0.0_f64; output_len_c];
                let vhf_ptr: *mut f64 = vhf_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![vhf_ptr];
                let ret = unsafe {
                    ti_vhf(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_vhf returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_vhf(&inputs_rust, &options, None).expect("Rust VHF indicator failed");

                let output_len_rust = outputs[0].len();

                for (i, (&c_val, &rust_val)) in vhf_output_vec_c
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
                            "Rust VHF has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust VHF has infinity at index {}: Rust = {}, Options = {:?}",
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
                            index, vhf_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_vhf_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_vhf(&inputs_rust, &options, None).expect("Rust VHF indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) =
                    rust_vhf(&chunk_inputs, &options, None).expect("Rust VHF indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VHF batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VHF batch indicator failed");
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
    fn test_vhf_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::vhf::indicator_by_assets;

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

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets VHF indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) =
                    rust_vhf(&stock_inputs, &options, None).expect("Regular VHF indicator failed");

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
                for (value_idx, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD VHF has NaN at index {}: SIMD = {}, Stock = {}, Options = {:?}",
                            value_idx, simd_val, stock_symbol, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD VHF has infinity at index {}: SIMD = {}, Stock = {}, Options = {:?}",
                            value_idx, simd_val, stock_symbol, options
                        );
                    }

                    // Compare values
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        let start = value_idx.saturating_sub(10);
                        println!(
                            "simd VHF results: {:?} \n\nRegular VHF Results: {:?} \n\n",
                            &simd_result[start..(value_idx + 10).min(simd_result.len())], //[..10.min(simd_aroon_down.len())],
                            &regular_result[start..(value_idx + 10).min(regular_result.len())]
                        );
                        panic!(
                            "SIMD vs Regular mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            value_idx, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_vhf_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::vhf::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Process first 4 options with SIMD
            let options_4_1 = [
                &OPTIONS_LIST[0], // [5.0]
                &OPTIONS_LIST[1], // [10.0]
                &OPTIONS_LIST[2], // [14.0]
                &OPTIONS_LIST[3], // [18.0]
            ];

            let (simd_results_1, _) = indicator_by_options::<4>(&inputs, &options_4_1, None)
                .expect("SIMD by options VHF indicator failed (batch 1)");

            // Process remaining 3 options with SIMD (pad with first option to make 4)
            let options_4_2 = [
                &OPTIONS_LIST[4], // [25.0]
                &OPTIONS_LIST[5], // [28.0]
                &OPTIONS_LIST[6], // [30.0]
                &OPTIONS_LIST[0], // [5.0] - padding
            ];

            let (simd_results_2, _) = indicator_by_options::<4>(&inputs, &options_4_2, None)
                .expect("SIMD by options VHF indicator failed (batch 2)");

            // Test all 7 options
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result for this option set
                let (regular_results, _) =
                    rust_vhf(&inputs, options, None).expect("Regular VHF indicator failed");

                // Get SIMD result from appropriate batch
                let simd_result = if idx < 4 {
                    &simd_results_1[idx][0]
                } else {
                    &simd_results_2[idx - 4][0]
                };
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
                for (value_idx, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD VHF has NaN at index {}: SIMD = {}, Stock = {}, Options = {:?}",
                            value_idx, simd_val, stock_symbol, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD VHF has infinity at index {}: SIMD = {}, Stock = {}, Options = {:?}",
                            value_idx, simd_val, stock_symbol, options
                        );
                    }

                    // Compare values
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        let start = value_idx.saturating_sub(10);
                        println!(
                            "SIMD VHF results: {:?} \n\nRegular VHF Results: {:?} \n\n",
                            &simd_result[start..(value_idx + 10).min(simd_result.len())],
                            &regular_result[start..(value_idx + 10).min(regular_result.len())]
                        );
                        panic!(
                            "SIMD vs Regular mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            value_idx, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All 7 SIMD by options vs Regular VHF database tests passed!");
    }
}
