#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::willr::{indicator as rust_willr, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_willr, ti_willr_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 8] = [
        [5.0],
        [7.0],
        [10.0],
        [14.0],
        [20.0],
        [25.0],
        [50.0],
        [100.0],
    ];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..100 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
        }
        (high_vec, low_vec, close_vec)
    }

    #[test]
    fn test_willr_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            // Determine the offset required by the C WILLR function
            let start_index = unsafe { ti_willr_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_willr_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut willr_output_vec_c = vec![0.0_f64; output_len_c];
            let willr_ptr: *mut f64 = willr_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![willr_ptr];
            let ret = unsafe {
                ti_willr(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_willr returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (outputs, _) =
                rust_willr(&inputs_rust, &options, None).expect("Rust WILLR indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in willr_output_vec_c
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
                        "Rust WILLR has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust WILLR has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val.abs(), rust_val.abs(), epsilon = 1e-12) {
                    // Compare absolute values
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, willr_output_vec_c, outputs[0], options
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
    fn test_willr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

                let start_index = unsafe { ti_willr_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_willr_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_willr(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_willr returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (outputs, _) =
                    rust_willr(&inputs_rust, &options, None).expect("Rust WILLR indicator failed");

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
                            "Rust WILLR has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust WILLR has infinity at index {}: Rust = {}, Options = {:?}",
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

                    if !approx_eq!(f64, c_val.abs(), rust_val.abs(), epsilon = 1e-12) {
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
    fn test_willr_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_willr(&inputs_rust, &options, None).expect("Rust WILLR indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let high_vec = high[..min_data_val].to_vec();
                let low_vec = low[..min_data_val].to_vec();
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [
                    high_vec.as_slice(),
                    low_vec.as_slice(),
                    close_vec.as_slice(),
                ];

                let (first_outputs, mut state) =
                    rust_willr(&chunk_inputs, &options, None).expect("Rust WILLR indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let remaining_len = high.len() - min_data_val;
                let mut chunks_processed = 0;
                while chunks_processed * CHUNK_SIZE < remaining_len {
                    let start_idx = min_data_val + chunks_processed * CHUNK_SIZE;
                    let end_idx = (start_idx + CHUNK_SIZE).min(high.len());

                    let high_vec = high[start_idx..end_idx].to_vec();
                    let low_vec = low[start_idx..end_idx].to_vec();
                    let close_vec = close[start_idx..end_idx].to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                    ];

                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("WILLR batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    chunks_processed += 1;
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

    #[test]
    fn test_willr_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::willr::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, close) = get_hlc_arrays(data);
                (symbol.clone(), high, low, close)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 3]; 4] = [
            &[&stock_data[0].1, &stock_data[0].2, &stock_data[0].3], // high, low, close
            &[&stock_data[1].1, &stock_data[1].2, &stock_data[1].3], // high, low, close
            &[&stock_data[2].1, &stock_data[2].2, &stock_data[2].3], // high, low, close
            &[&stock_data[3].1, &stock_data[3].2, &stock_data[3].3], // high, low, close
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets WILLR indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (regular_results, _) = rust_willr(&stock_inputs, &options, None)
                    .expect("Regular WILLR indicator failed");

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
                            "SIMD WILLR has NaN at index {}: SIMD = {}, Stock = {}, Options = {:?}",
                            value_idx, simd_val, stock_symbol, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD WILLR has infinity at index {}: SIMD = {}, Stock = {}, Options = {:?}",
                            value_idx, simd_val, stock_symbol, options
                        );
                    }

                    // Compare values (using absolute values like the other tests)
                    if !approx_eq!(f64, simd_val.abs(), regular_val.abs(), epsilon = 1e-12) {
                        let start = value_idx.saturating_sub(10);
                        println!(
                            "SIMD WILLR results: {:?} \n\nRegular WILLR Results: {:?} \n\n",
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

        println!("✓ All SIMD by assets vs Regular WILLR database tests passed!");
    }

    #[test]
    fn test_willr_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::willr::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Process all 8 options with 8-lane SIMD
            let options_8 = [
                &OPTIONS_LIST[0], // [5.0]
                &OPTIONS_LIST[1], // [7.0]
                &OPTIONS_LIST[2], // [10.0]
                &OPTIONS_LIST[3], // [14.0]
                &OPTIONS_LIST[4], // [20.0]
                &OPTIONS_LIST[5], // [25.0]
                &OPTIONS_LIST[6], // [50.0]
                &OPTIONS_LIST[7], // [100.0]
            ];

            let (simd_results, _) = indicator_by_options::<8>(&inputs, &options_8, None)
                .expect("SIMD by options WILLR indicator failed");

            // Test all 8 options
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result for this option set
                let (regular_results, _) =
                    rust_willr(&inputs, options, None).expect("Regular WILLR indicator failed");

                let simd_result = &simd_results[idx][0];
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
                            "SIMD WILLR has NaN at index {}: SIMD = {}, Stock = {}, Options = {:?}",
                            value_idx, simd_val, stock_symbol, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD WILLR has infinity at index {}: SIMD = {}, Stock = {}, Options = {:?}",
                            value_idx, simd_val, stock_symbol, options
                        );
                    }

                    // Compare values (using absolute values like the other tests)
                    if !approx_eq!(f64, simd_val.abs(), regular_val.abs(), epsilon = 1e-12) {
                        let start = value_idx.saturating_sub(10);
                        println!(
                            "SIMD WILLR results: {:?} \n\nRegular WILLR Results: {:?} \n\n",
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

        println!("✓ All 8 SIMD by options vs Regular WILLR database tests passed!");
    }

    fn get_hlc_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }
}
