#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::dm::indicator_by_assets;
    use tulip_rs::indicators::dm::indicator_by_options;
    use tulip_rs::indicators::dm::{indicator as rust_dm, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_dm, ti_dm_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILON: f64 = 1e-10;
    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[24.0], [14.0], [5.0], [30.0]];

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
    fn test_dm_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            // Determine the offset required by the C DM function
            let start_index = unsafe { ti_dm_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_dm_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut plus_dm_vec_c = vec![0.0_f64; output_len_c];
            let mut minus_dm_vec_c = vec![0.0_f64; output_len_c];
            let plus_dm_ptr: *mut f64 = plus_dm_vec_c.as_mut_ptr();
            let minus_dm_ptr: *mut f64 = minus_dm_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![plus_dm_ptr, minus_dm_ptr];
            let ret = unsafe {
                ti_dm(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_dm returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (outputs, _) =
                rust_dm(&inputs_rust, &options, None).expect("Rust DM indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the +DM outputs in reverse
            for (i, (&c_val, &rust_val)) in plus_dm_vec_c
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
                        "Rust DM_PLUS has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust DM has infinity at index {}: Rust = {}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILON) {
                    println!(
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}",
                        index, plus_dm_vec_c, outputs[0], options
                    );
                    panic!(
                        "Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }

            // Compare the -DM outputs in reverse
            for (i, (&c_val, &rust_val)) in minus_dm_vec_c
                .iter()
                .rev()
                .take(output_len_rust)
                .zip(outputs[1].iter().rev())
                .enumerate()
            {
                let index = output_len_rust - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust DM_MINUS has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Skip if only C has NaN (C bug)
                if c_val.is_nan() && !rust_val.is_nan() {
                    continue;
                }

                if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILON) {
                    println!(
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}",
                        index, minus_dm_vec_c, outputs[1], options
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
    fn test_dm_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

                let start_index = unsafe { ti_dm_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_dm_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut plus_dm_vec_c = vec![0.0_f64; output_len_c];
                let mut minus_dm_vec_c = vec![0.0_f64; output_len_c];
                let plus_dm_ptr: *mut f64 = plus_dm_vec_c.as_mut_ptr();
                let minus_dm_ptr: *mut f64 = minus_dm_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![plus_dm_ptr, minus_dm_ptr];
                let ret = unsafe {
                    ti_dm(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_dm returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice()];
                let (outputs, _) =
                    rust_dm(&inputs_rust, &options, None).expect("Rust DM indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare +DM results
                for (i, (&c_val, &rust_val)) in plus_dm_vec_c
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
                            "Rust DM_PLUS has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust DM has infinity at index {}: Rust = {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILON) {
                        println!(
                            "DM +DM test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, plus_dm_vec_c, outputs[0], options, stock_symbol
                        );
                        panic!(
                            "DM +DM mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }

                // Compare -DM results
                for (i, (&c_val, &rust_val)) in minus_dm_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust)
                    .zip(outputs[1].iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() {
                        panic!(
                            "Rust DM_MINUS has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Skip if only C has NaN (C bug)
                    if c_val.is_nan() && !rust_val.is_nan() {
                        continue;
                    }

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILON) {
                        println!(
                            "DM -DM test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, minus_dm_vec_c, outputs[1], options, stock_symbol
                        );
                        panic!(
                            "DM -DM mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_dm_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice()];

                // Get full output
                let (full_outputs, _) =
                    rust_dm(&inputs_rust, &options, None).expect("Rust DM indicator failed");

                // Process in batches
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) =
                        rust_dm(&inputs_rust, &options, None).expect("Failed to run DM indicator");
                    for (output_idx, output) in outputs.iter().enumerate() {
                        batch_full_outputs[output_idx].extend_from_slice(output);
                    }
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let high_vec = high[..min_data_val].to_vec();
                    let low_vec = low[..min_data_val].to_vec();
                    let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];

                    let (first_outputs, mut state) = rust_dm(&chunk_inputs, &options, None)
                        .expect("Failed to run DM indicator on first chunk");
                    for (output_idx, output) in first_outputs.iter().enumerate() {
                        batch_full_outputs[output_idx].extend_from_slice(output);
                    }

                    // Process remaining data in chunks using state
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                        let high_vec = high_chunk.to_vec();
                        let low_vec = low_chunk.to_vec();
                        let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("DM batch indicator failed");
                        for (output_idx, output) in chunk_outputs.iter().enumerate() {
                            batch_full_outputs[output_idx].extend_from_slice(output);
                        }
                    }

                    // Process remainder if any
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();

                    if !high_rem.is_empty() && !low_rem.is_empty() {
                        let high_vec = high_rem.to_vec();
                        let low_vec = low_rem.to_vec();
                        let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("DM batch indicator failed");
                        for (output_idx, output) in chunk_outputs.iter().enumerate() {
                            batch_full_outputs[output_idx].extend_from_slice(output);
                        }
                    }
                }

                // Compare outputs (plus_dm and minus_dm)
                for output_idx in 0..2 {
                    for (i, (&full_val, &batch_val)) in full_outputs[output_idx]
                        .iter()
                        .zip(batch_full_outputs[output_idx].iter())
                        .enumerate()
                    {
                        assert_eq!(
                            full_val, batch_val,
                            "DM output {} mismatch at index {}: full = {}, batch = {}, options = {:?}, stock = {}",
                            output_idx, i, full_val, batch_val, options, stock_symbol
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_dm_simd_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low) = get_hl_arrays(data);
                (symbol.clone(), high, low)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 2]; 4] = [
            &[
                &stock_data[0].1, // high
                &stock_data[0].2, // low
            ],
            &[
                &stock_data[1].1, // high
                &stock_data[1].2, // low
            ],
            &[
                &stock_data[2].1, // high
                &stock_data[2].2, // low
            ],
            &[
                &stock_data[3].1, // high
                &stock_data[3].2, // low
            ],
        ];

        for options in &OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, options, None)
                .expect("SIMD by assets DM indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low)) in stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_high.as_slice(), stock_low.as_slice()];
                let (regular_outputs, _) = rust_dm(&stock_inputs, options, None).unwrap_or_else(|_| panic!("Regular DM failed for {} with options {:?}",
                    stock_symbol, options));

                // Compare number of outputs (should be 2: +DM and -DM)
                assert_eq!(
                    regular_outputs.len(),
                    simd_results[stock_idx].len(),
                    "Number of outputs mismatch for stock {} with options {:?}: regular = {}, simd = {}",
                    stock_symbol,
                    options,
                    regular_outputs.len(),
                    simd_results[stock_idx].len()
                );

                // Compare +DM output (index 0)
                assert_eq!(
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len(),
                    "+DM output length mismatch for stock {} with options {:?}: regular = {}, simd = {}",
                    stock_symbol,
                    options,
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[0]
                    .iter()
                    .zip(simd_results[stock_idx][0].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = 1e-12) {
                        panic!(
                            "+DM mismatch at index {} for stock {} with options {:?}: regular = {}, simd = {}",
                            i, stock_symbol, options, regular_val, simd_val
                        );
                    }
                }

                // Compare -DM output (index 1)
                assert_eq!(
                    regular_outputs[1].len(),
                    simd_results[stock_idx][1].len(),
                    "-DM output length mismatch for stock {} with options {:?}: regular = {}, simd = {}",
                    stock_symbol,
                    options,
                    regular_outputs[1].len(),
                    simd_results[stock_idx][1].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[1]
                    .iter()
                    .zip(simd_results[stock_idx][1].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = 1e-12) {
                        panic!(
                            "-DM mismatch at index {} for stock {} with options {:?}: regular = {}, simd = {}",
                            i, stock_symbol, options, regular_val, simd_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_dm_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice()];

            // Process all 3 options with 3-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_3, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD DM 3-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_3;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_dm(&inputs, options, None).expect("Regular DM indicator failed");

                let simd_plus_dm_result = &all_simd_results[idx][0];
                let regular_plus_dm_result = &regular_results[0];

                let simd_minus_dm_result = &all_simd_results[idx][1];
                let regular_minus_dm_result = &regular_results[1];

                // Compare Plus DM output lengths
                assert_eq!(
                    simd_plus_dm_result.len(),
                    regular_plus_dm_result.len(),
                    "Plus DM output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_plus_dm_result.len(),
                    regular_plus_dm_result.len()
                );

                // Compare Minus DM output lengths
                assert_eq!(
                    simd_minus_dm_result.len(),
                    regular_minus_dm_result.len(),
                    "Minus DM output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_minus_dm_result.len(),
                    regular_minus_dm_result.len()
                );

                // Compare Plus DM values
                for (i, (&simd_val, &regular_val)) in simd_plus_dm_result
                    .iter()
                    .zip(regular_plus_dm_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Plus DM has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Plus DM has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        println!("SIMD: {:?}", simd_plus_dm_result);
                        panic!(
                            "Plus DM mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Minus DM values
                for (i, (&simd_val, &regular_val)) in simd_minus_dm_result
                    .iter()
                    .zip(regular_minus_dm_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Minus DM has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Minus DM has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Minus DM mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular DM database tests passed!");
    }
}
