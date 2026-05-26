#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::rsi::indicator_by_assets;
    use tulip_rs::indicators::rsi::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_rsi, ti_rsi_start};
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
    fn test_rsi_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C RSI function
            let start_index = unsafe { ti_rsi_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_rsi_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut rsi_output_vec_c = vec![0.0_f64; output_len_c];
            let rsi_ptr: *mut f64 = rsi_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![rsi_ptr];
            let ret = unsafe {
                ti_rsi(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_rsi returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Rust RSI indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in rsi_output_vec_c
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
                        "Rust RSI has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust RSI has infinity at index {}: Rust = {}, Options = {:?}",
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
                        index, rsi_output_vec_c, outputs[0], options
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
    fn test_rsi_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);

            for options in OPTIONS_LIST {
                // run c code
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                // Determine the offset required by the C RSI function
                let start_index = unsafe { ti_rsi_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_rsi_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                // Run the C implementation
                let mut rsi_output_vec_c = vec![0.0_f64; output_len_c];
                let rsi_ptr: *mut f64 = rsi_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![rsi_ptr];
                let ret = unsafe {
                    ti_rsi(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_rsi returned error code {}", ret);

                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    indicator(&inputs_rust, &options, None).expect("Rust RSI indicator failed");

                let output_len_rust = outputs[0].len();

                for (i, (&c_val, &rust_val)) in rsi_output_vec_c
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
                            "Rust RSI has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust RSI has infinity at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
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
                            index, rsi_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_rsi_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let inputs_rust = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = indicator(&inputs_rust, &options, None)
                    .expect("Failed to run RSI indicator on full data");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if close.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) = indicator(&inputs_rust, &options, None)
                        .expect("Failed to run RSI indicator");
                    batch_full_output.extend_from_slice(&outputs[0]);
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let close_vec = close[..min_data_val].to_vec();
                    let chunk_inputs = [close_vec.as_slice()];

                    let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("Failed to run RSI indicator on first chunk");
                    batch_full_output.extend_from_slice(&first_outputs[0]);

                    // Process remaining data in chunks using state
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for close_chunk in close_chunks.by_ref() {
                        let close_vec = close_chunk.to_vec();
                        let chunk_inputs = [close_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("RSI batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }

                    // Process remainder if any
                    let close_rem = close_chunks.remainder();

                    if !close_rem.is_empty() {
                        let close_vec = close_rem.to_vec();
                        let chunk_inputs = [close_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("RSI batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }
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
                        "Mismatch at index {} for stock {} with options {:?}: full={}, batch={}",
                        i, stock_symbol, options, full_val, batch_val
                    );
                }
            }
        }
    }

    #[test]
    fn test_rsi_simd_by_assets() {
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
                .expect("SIMD RSI indicator failed");

            // Run regular implementation for comparison
            let inputs_rust = [close.as_slice()];
            let (regular_outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Regular RSI indicator failed");

            // Compare each SIMD asset output with regular output
            for asset_idx in 0..4 {
                let simd_output = &simd_outputs[asset_idx][0];
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
                            "SIMD RSI has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD RSI has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
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
    fn test_rsi_simd_by_assets_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Group stocks in sets of 4
        let stock_data: Vec<_> = data.into_iter().collect();
        let chunks: Vec<_> = stock_data.chunks(4).collect();

        for chunk in chunks {
            let close_arrays: Vec<_> = chunk
                .iter()
                .map(|(_, data)| get_close_array(data))
                .collect();

            // Pad to 4 assets if needed
            let mut padded_close = close_arrays.clone();
            while padded_close.len() < 4 {
                padded_close.push(padded_close[0].clone());
            }

            for options in OPTIONS_LIST {
                let min_len = padded_close.iter().map(|c| c.len()).min().unwrap_or(0);
                if min_len < min_data(&options) {
                    continue;
                }

                // Prepare inputs for SIMD
                let inputs: [&[&[f64]; 1]; 4] = [
                    &[padded_close[0].as_slice()],
                    &[padded_close[1].as_slice()],
                    &[padded_close[2].as_slice()],
                    &[padded_close[3].as_slice()],
                ];

                // Run SIMD implementation
                let (simd_outputs, _) = indicator_by_assets::<4>(&inputs, &options, None)
                    .expect("SIMD RSI indicator failed");

                // Compare each asset's SIMD output with its regular output
                for (asset_idx, close_data) in padded_close.iter().enumerate().take(chunk.len()) {
                    let inputs_rust = [close_data.as_slice()];
                    let (regular_outputs, _) = indicator(&inputs_rust, &options, None)
                        .expect("Regular RSI indicator failed");

                    let simd_output = &simd_outputs[asset_idx][0];
                    let regular_output = &regular_outputs[0];

                    assert_eq!(
                        simd_output.len(),
                        regular_output.len(),
                        "Output length mismatch for asset {}: SIMD = {}, Regular = {}",
                        asset_idx,
                        simd_output.len(),
                        regular_output.len()
                    );

                    for (i, (&simd_val, &regular_val)) in
                        simd_output.iter().zip(regular_output.iter()).enumerate()
                    {
                        if simd_val.is_nan() {
                            panic!(
                                "SIMD RSI has NaN at index {} for asset {}: SIMD = {}",
                                i, asset_idx, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD RSI has infinity at index {} for asset {}: SIMD = {}",
                                i, asset_idx, simd_val
                            );
                        }

                        if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                            //println!("SIMD: {:?}\n\nRegular: {:?}", &simd_output[..20], &regular_output[..20]);
                            panic!(
                                "SIMD vs Regular mismatch at index {} for asset {}: SIMD = {}, Regular = {}",
                                i, asset_idx, simd_val, regular_val
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_rsi_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::rsi::indicator_by_options;

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
                .expect("SIMD RSI 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD RSI 2-wide failed");

            // Combine SIMD results
            let mut all_simd_results = Vec::new();

            // Add 4-wide results
            for i in 0..4 {
                all_simd_results.push(simd_results_4[i].clone());
            }

            // Add 2-wide results
            for i in 0..2 {
                all_simd_results.push(simd_results_2[i].clone());
            }

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    indicator(&inputs, options, None).expect("Regular RSI indicator failed");

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
                            "SIMD RSI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD RSI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with appropriate epsilon
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD by options = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular RSI database tests passed!");
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }
}
