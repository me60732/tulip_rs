#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::cvi::{indicator as rust_cvi, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_cvi, ti_cvi_start};
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

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [30.0]];

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
    fn test_cvi_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            // Determine the offset required by the C CVI function
            let start_index = unsafe { ti_cvi_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_cvi_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut cvi_output_vec_c = vec![0.0_f64; output_len_c];
            let cvi_ptr: *mut f64 = cvi_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![cvi_ptr];
            let ret = unsafe {
                ti_cvi(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_cvi returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (outputs, _) =
                rust_cvi(&inputs_rust, &options, None).expect("Rust CVI indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in cvi_output_vec_c
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
                        "Rust CVI has NaN at index {}: Rust = {}, Options = {:?}",
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
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, cvi_output_vec_c, outputs[0], options
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
    fn test_cvi_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

                let start_index = unsafe { ti_cvi_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_cvi_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut cvi_output_vec_c = vec![0.0_f64; output_len_c];
                let cvi_ptr: *mut f64 = cvi_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![cvi_ptr];
                let ret = unsafe {
                    ti_cvi(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_cvi returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice()];
                let (outputs, _) =
                    rust_cvi(&inputs_rust, &options, None).expect("Rust CVI indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare results
                for (i, (&c_val, &rust_val)) in cvi_output_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust)
                    .zip(outputs[0].iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust CVI has NaN or INF at index {}: Rust = {}, Options = {:?}, Stock: {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-10) {
                        println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, cvi_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_cvi_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice()];

                // Get full output
                let (full_outputs, _) =
                    rust_cvi(&inputs_rust, &options, None).expect("Rust CVI indicator failed");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) = rust_cvi(&inputs_rust, &options, None)
                        .expect("Failed to run CVI indicator");
                    batch_full_output.extend_from_slice(&outputs[0]);
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let high_vec = high[..min_data_val].to_vec();
                    let low_vec = low[..min_data_val].to_vec();
                    let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];

                    let (first_outputs, mut state) = rust_cvi(&chunk_inputs, &options, None)
                        .expect("Failed to run CVI indicator on first chunk");
                    batch_full_output.extend_from_slice(&first_outputs[0]);

                    // Process remaining data in chunks using state
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                        let high_vec = high_chunk.to_vec();
                        let low_vec = low_chunk.to_vec();
                        let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("CVI batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
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
                            .expect("CVI batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }
                }

                // Compare outputs
                for (i, (&full_val, &batch_val)) in full_outputs[0]
                    .iter()
                    .zip(batch_full_output.iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "CVI mismatch at index {}: full = {}, batch = {}, options = {:?}, stock = {}",
                        i, full_val, batch_val, options, stock_symbol
                    );
                }
            }
        }
    }
    #[test]
    fn test_cvi_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::cvi::indicator_by_assets;

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
            &[&stock_data[0].1, &stock_data[0].2], // high, low
            &[&stock_data[1].1, &stock_data[1].2], // high, low
            &[&stock_data[2].1, &stock_data[2].2], // high, low
            &[&stock_data[3].1, &stock_data[3].2], // high, low
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets CVI indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low)) in stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_high.as_slice(), stock_low.as_slice()];
                let (regular_results, _) =
                    rust_cvi(&stock_inputs, &options, None).expect("Regular CVI indicator failed");

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
                            "SIMD by assets CVI has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets CVI has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for CVI
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

        println!("✓ All SIMD by assets vs Regular CVI database tests passed!");
    }

    #[test]
    fn test_cvi_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::cvi::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let inputs = [high.as_slice(), low.as_slice()];

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD CVI 4-wide failed");

            // Use SIMD results directly (all 4 options processed)
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_cvi(&inputs, options, None).expect("Regular CVI indicator failed");

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
                            "SIMD CVI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD CVI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
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

        println!("✓ All SIMD by options vs Regular CVI database tests passed!");
    }

    #[test]
    fn test_cvi_simd_state_handover_by_options() {
        use tulip_rs::indicators::cvi::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // number of bars to process with SIMD first
        let first_bars = 2000usize;

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let total_len = high.len();
            if total_len == 0 {
                continue;
            }

            let split = first_bars.min(total_len);

            // prepare slices for first part and remaining
            let first_inputs = [&high[..split], &low[..split]];
            let remaining_inputs = if split < total_len {
                Some([&high[split..], &low[split..]])
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
                    .expect("SIMD CVI 4-wide failed on first chunk");

            // Combine SIMD results for first part and prepare to extend with batch_indicator outputs
            let mut all_simd_results: Vec<Vec<f64>> = Vec::new();
            for i in 0..4 {
                all_simd_results.push(simd_results_4[i][0].clone());
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
                let (regular_results, _) =
                    rust_cvi(&[high.as_slice(), low.as_slice()], options, None)
                        .expect("Regular CVI indicator failed");
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

        println!("✓ All CVI SIMD state handover by options tests passed!");
    }
}
