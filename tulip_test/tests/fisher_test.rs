#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::fisher::{indicator as rust_fisher, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_fisher, ti_fisher_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILION: f64 = 1e-12;
    const CHUNK_SIZE: usize = 100;

    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 87.01,
        87.87, 87.60,
    ];

    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 86.54,
        87.66, 87.00,
    ];
    const OPTIONS_LIST: [[f64; 1]; 6] = [[10.0], [14.0], [25.0], [35.0], [50.0], [100.0]];
    //const OPTIONS_LIST: [[f64; 1]; 6] = [[100.0], [50.0], [35.0], [25.0], [14.0], [10.0]];
    //const OPTIONS_LIST: [[f64; 1]; 3] = [[5.0], [10.0], [14.0]];
    //const OPTIONS_LIST: [[f64; 1]; 1] = [[5.0]];
    fn get_high_low_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
        let high = stock_data.iter().map(|d| d.high).collect();
        let low = stock_data.iter().map(|d| d.low).collect();
        (high, low)
    }

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_high_low() -> (Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        for _ in 0..300 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
        }
        (high_vec, low_vec)
    }

    #[test]
    fn test_fisher_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low) = expand_high_low();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            // Determine the offset required by the C Fisher function
            let start_index = unsafe { ti_fisher_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_fisher_start returned a negative index"
            );
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut fisher_output_vec_c = vec![0.0_f64; output_len_c];
            let mut signal_output_vec_c = vec![0.0_f64; output_len_c];
            let fisher_ptr: *mut f64 = fisher_output_vec_c.as_mut_ptr();
            let signal_ptr: *mut f64 = signal_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![fisher_ptr, signal_ptr];
            let ret = unsafe {
                ti_fisher(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_fisher returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (outputs, _) =
                rust_fisher(&inputs_rust, &options, None).expect("Rust Fisher indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the Fisher outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in fisher_output_vec_c
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
                        "Rust Fisher has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust Fisher has infinity at index {}: Rust = {}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILION) {
                    println!(
                        "Fisher test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, fisher_output_vec_c, outputs[0], options
                    );
                    panic!(
                        "Fisher mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }

            // Compare the Signal outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in signal_output_vec_c
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
                        "Rust Fisher Signal has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust Fisher Signal has infinity at index {}: Rust = {}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILION) {
                    println!(
                        "Signal test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, signal_output_vec_c, outputs[1], options
                    );
                    panic!(
                        "Signal mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_fisher_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_high_low_arrays(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

                let start_index = unsafe { ti_fisher_start(options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_fisher_start returned a negative index"
                );
                let output_len_c = high.len() - (start_index as usize);

                let mut fisher_output_vec_c = vec![0.0_f64; output_len_c];
                let mut signal_output_vec_c = vec![0.0_f64; output_len_c];
                let fisher_ptr: *mut f64 = fisher_output_vec_c.as_mut_ptr();
                let signal_ptr: *mut f64 = signal_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![fisher_ptr, signal_ptr];
                let ret = unsafe {
                    ti_fisher(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_fisher returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice()];
                let (outputs, _) = rust_fisher(&inputs_rust, &options, None)
                    .expect("Rust Fisher indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare Fisher results
                for (i, (&c_val, &rust_val)) in fisher_output_vec_c
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
                            "Rust Fisher has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust Fisher has infinity at index {}: Rust = {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILION) {
                        println!(
                            "Fisher test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, fisher_output_vec_c, outputs[0], options, stock_symbol
                        );
                        panic!(
                            "Fisher mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }

                // Compare Signal results
                for (i, (&c_val, &rust_val)) in signal_output_vec_c
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
                            "Rust Fisher Signal has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust Fisher Signal has infinity at index {}: Rust = {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILION) {
                        println!(
                            "Signal test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, signal_output_vec_c, outputs[1], options, stock_symbol
                        );
                        panic!(
                            "Signal mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_fisher_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_high_low_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) = rust_fisher(&inputs_rust, &options, None)
                    .expect("Rust Fisher indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output_fisher = Vec::new();
                let mut batch_full_output_signal = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) = rust_fisher(&inputs_rust, &options, None)
                        .expect("Failed to run Fisher indicator");
                    batch_full_output_fisher.extend_from_slice(&outputs[0]);
                    batch_full_output_signal.extend_from_slice(&outputs[1]);
                } else {
                    // First chunk
                    let high_vec = high[..min_data_val].to_vec();
                    let low_vec = low[..min_data_val].to_vec();
                    let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];

                    let (first_outputs, mut state) = rust_fisher(&chunk_inputs, &options, None)
                        .expect("Failed to run Fisher indicator on first chunk");
                    batch_full_output_fisher.extend_from_slice(&first_outputs[0]);
                    batch_full_output_signal.extend_from_slice(&first_outputs[1]);

                    // Process remaining data in chunks using state
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                        let high_vec = high_chunk.to_vec();
                        let low_vec = low_chunk.to_vec();
                        let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("Fisher batch indicator failed");
                        batch_full_output_fisher.extend_from_slice(&chunk_outputs[0]);
                        batch_full_output_signal.extend_from_slice(&chunk_outputs[1]);
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
                            .expect("Fisher batch indicator failed");
                        batch_full_output_fisher.extend_from_slice(&chunk_outputs[0]);
                        batch_full_output_signal.extend_from_slice(&chunk_outputs[1]);
                    }
                }

                // Compare Fisher outputs
                assert_eq!(
                    full_outputs[0].len(),
                    batch_full_output_fisher.len(),
                    "Fisher output length mismatch for stock {} with options {:?}: full={}, batch={}",
                    stock_symbol,
                    options,
                    full_outputs[0].len(),
                    batch_full_output_fisher.len()
                );

                for (i, (&full_val, &batch_val)) in full_outputs[0]
                    .iter()
                    .zip(batch_full_output_fisher.iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "Fisher state handover mismatch at index {} for stock {} with options {:?}: full = {}, batch = {}",
                        i, stock_symbol, options, full_val, batch_val
                    );
                }

                // Compare Signal outputs
                assert_eq!(
                    full_outputs[1].len(),
                    batch_full_output_signal.len(),
                    "Signal output length mismatch for stock {} with options {:?}: full={}, batch={}",
                    stock_symbol,
                    options,
                    full_outputs[1].len(),
                    batch_full_output_signal.len()
                );

                for (i, (&full_val, &batch_val)) in full_outputs[1]
                    .iter()
                    .zip(batch_full_output_signal.iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "Signal state handover mismatch at index {} for stock {} with options {:?}: full = {}, batch = {}",
                        i, stock_symbol, options, full_val, batch_val
                    );
                }
            }
        }
    }

    #[test]
    fn test_fisher_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::fisher::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_high_low_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD Fisher 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD Fisher 2-wide failed");

            // Combine all SIMD results
            let mut all_simd_results = simd_results_4;
            all_simd_results.extend(simd_results_2);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_fisher(&inputs, options, None).expect("Regular Fisher indicator failed");

                let simd_result = &all_simd_results[idx];
                let regular_result = &regular_results;

                // Compare output lengths for both Fisher and Signal
                assert_eq!(
                    simd_result[0].len(),
                    regular_result[0].len(),
                    "Fisher output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[0].len(),
                    regular_result[0].len()
                );

                assert_eq!(
                    simd_result[1].len(),
                    regular_result[1].len(),
                    "Signal output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[1].len(),
                    regular_result[1].len()
                );

                // Compare Fisher values
                for (i, (&simd_val, &regular_val)) in simd_result[0]
                    .iter()
                    .zip(regular_result[0].iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Fisher has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Fisher has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "Fisher mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Signal values
                for (i, (&simd_val, &regular_val)) in simd_result[1]
                    .iter()
                    .zip(regular_result[1].iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Signal has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Signal has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "Signal mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular Fisher database tests passed!");
    }

    #[test]
    fn test_fisher_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::fisher::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' high/low data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low) = get_high_low_arrays(data);
                (symbol.clone(), high, low)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 2]; 4] = [
            &[&stock_data[0].1, &stock_data[0].2],
            &[&stock_data[1].1, &stock_data[1].2],
            &[&stock_data[2].1, &stock_data[2].2],
            &[&stock_data[3].1, &stock_data[3].2],
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets Fisher indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low)) in stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_high.as_slice(), stock_low.as_slice()];
                let (regular_results, _) = rust_fisher(&stock_inputs, &options, None)
                    .expect("Regular Fisher indicator failed");

                let simd_fisher_result = &simd_results[stock_idx][0];
                let simd_signal_result = &simd_results[stock_idx][1];
                let regular_fisher_result = &regular_results[0];
                let regular_signal_result = &regular_results[1];

                // Compare Fisher output lengths
                assert_eq!(
                    simd_fisher_result.len(),
                    regular_fisher_result.len(),
                    "Fisher output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_fisher_result.len(),
                    regular_fisher_result.len()
                );

                // Compare Signal output lengths
                assert_eq!(
                    simd_signal_result.len(),
                    regular_signal_result.len(),
                    "Signal output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_signal_result.len(),
                    regular_signal_result.len()
                );

                // Compare Fisher values
                for (i, (&simd_val, &regular_val)) in simd_fisher_result
                    .iter()
                    .zip(regular_fisher_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by assets Fisher has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets Fisher has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        let start = i.saturating_sub(10);
                        println!(
                            "SIMD: {:?}\n\nRegular: {:?}",
                            &simd_fisher_result[start..(i + 10).min(simd_fisher_result.len())],
                            &regular_fisher_result[start..(i + 10).min(simd_fisher_result.len())]
                        );
                        panic!(
                            "Fisher mismatch at index {} for stock {} options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Signal values
                for (i, (&simd_val, &regular_val)) in simd_signal_result
                    .iter()
                    .zip(regular_signal_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by assets Signal has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets Signal has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "Signal mismatch at index {} for stock {} options {:?}: SIMD by assets = {}, Regular = {}",
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

        println!("✓ All SIMD by assets vs Regular Fisher database tests passed!");
    }

    #[test]
    fn test_fisher_simd_by_options_state_handover() {
        use tulip_rs::indicators::fisher::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_high_low_arrays(stock_data);

            // Skip if data is too small
            if high.len() < 1500 {
                continue;
            }

            let inputs = [high.as_slice(), low.as_slice()];

            // Test with 4-wide SIMD processing first 1000 rows, then state handover
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];

            // Process first 1000 rows with SIMD by options
            let first_1000_inputs = [high[..1000].to_vec(), low[..1000].to_vec()];
            let first_1000_slice_inputs = [
                first_1000_inputs[0].as_slice(),
                first_1000_inputs[1].as_slice(),
            ];

            let (simd_first_results, simd_states) =
                indicator_by_options::<4>(&first_1000_slice_inputs, &options_4, None)
                    .expect("SIMD Fisher first 1000 rows failed");

            // Process remainder using state.batch_indicator for each option
            let mut combined_results = Vec::new();

            for (idx, mut state) in simd_states.into_iter().enumerate() {
                // Get the remainder data
                let remainder_inputs = [high[1000..].to_vec(), low[1000..].to_vec()];
                let remainder_slice_inputs = [
                    remainder_inputs[0].as_slice(),
                    remainder_inputs[1].as_slice(),
                ];

                let remainder_outputs = state
                    .batch_indicator(&remainder_slice_inputs, None)
                    .expect("Fisher batch indicator failed for remainder");

                // Combine first 1000 + remainder results
                let mut combined_fisher = simd_first_results[idx][0].clone();
                let mut combined_signal = simd_first_results[idx][1].clone();

                combined_fisher.extend_from_slice(&remainder_outputs[0]);
                combined_signal.extend_from_slice(&remainder_outputs[1]);

                combined_results.push([combined_fisher, combined_signal]);
            }

            // Compare with regular indicator processing all data at once
            for (idx, options) in options_4.iter().enumerate() {
                let (regular_results, _) =
                    rust_fisher(&inputs, options, None).expect("Regular Fisher indicator failed");

                let combined_result = &combined_results[idx];

                // Compare lengths
                assert_eq!(
                    combined_result[0].len(),
                    regular_results[0].len(),
                    "Fisher output length mismatch for stock {} options {:?}: Combined={}, Regular={}",
                    stock_symbol,
                    options,
                    combined_result[0].len(),
                    regular_results[0].len()
                );

                assert_eq!(
                    combined_result[1].len(),
                    regular_results[1].len(),
                    "Signal output length mismatch for stock {} options {:?}: Combined={}, Regular={}",
                    stock_symbol,
                    options,
                    combined_result[1].len(),
                    regular_results[1].len()
                );

                // Compare Fisher values
                for (i, (&combined_val, &regular_val)) in combined_result[0]
                    .iter()
                    .zip(regular_results[0].iter())
                    .enumerate()
                {
                    if combined_val.is_nan() && regular_val.is_nan() {
                        continue;
                    }
                    if combined_val.is_infinite()
                        && regular_val.is_infinite()
                        && combined_val.signum() == regular_val.signum()
                    {
                        continue;
                    }

                    assert!(
                        (combined_val - regular_val).abs() < EPSILION,
                        "Fisher state handover mismatch at index {} for stock {} options {:?}: Combined = {}, Regular = {}",
                        i, stock_symbol, options, combined_val, regular_val
                    );
                }

                // Compare Signal values
                for (i, (&combined_val, &regular_val)) in combined_result[1]
                    .iter()
                    .zip(regular_results[1].iter())
                    .enumerate()
                {
                    if combined_val.is_nan() && regular_val.is_nan() {
                        continue;
                    }
                    if combined_val.is_infinite()
                        && regular_val.is_infinite()
                        && combined_val.signum() == regular_val.signum()
                    {
                        continue;
                    }

                    assert!(
                        (combined_val - regular_val).abs() < EPSILION,
                        "Signal state handover mismatch at index {} for stock {} options {:?}: Combined = {}, Regular = {}",
                        i, stock_symbol, options, combined_val, regular_val
                    );
                }

                println!(
                    "✓ SIMD by options state handover test passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All SIMD by options state handover tests passed!");
    }

    //add test code here
}
