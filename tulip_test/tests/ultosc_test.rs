#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::ultosc::indicator_by_options;
    use tulip_rs::indicators::ultosc::{indicator as rust_ultosc, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_ultosc, ti_ultosc_start};
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

    const OPTIONS_LIST: [[f64; 3]; 4] = [
        [2.0, 3.0, 5.0],
        [10.0, 14.0, 20.0],
        [14.0, 20.0, 50.0],
        [20.0, 50.0, 100.0],
    ];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..20 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
        }
        (high_vec, low_vec, close_vec)
    }

    #[test]
    fn test_ultosc_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            // Determine the offset required by the C ULTOSC function
            let start_index = unsafe { ti_ultosc_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_ultosc_start returned a negative index"
            );
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut ultosc_output_vec_c = vec![0.0_f64; output_len_c];
            let ultosc_ptr: *mut f64 = ultosc_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![ultosc_ptr];
            let ret = unsafe {
                ti_ultosc(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_ultosc returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (outputs, _) =
                rust_ultosc(&inputs_rust, &options, None).expect("Rust ULTOSC indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in ultosc_output_vec_c
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
                        "Rust ULTOSC has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust ULTOSC has infinity at index {}: Rust = {}, Options = {:?}",
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
                    /*println!(
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}",
                        index, ultosc_output_vec_c, outputs[0], options
                    );*/
                    panic!(
                        "Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_ultosc_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

                let start_index = unsafe { ti_ultosc_start(options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_ultosc_start returned a negative index"
                );
                let output_len_c = high.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_ultosc(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_ultosc returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (outputs, _) = rust_ultosc(&inputs_rust, &options, None)
                    .expect("Rust ULTOSC indicator failed");

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
                            "Rust ULTOSC has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust ULTOSC has infinity at index {}: Rust = {}, Options = {:?}",
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
                        /*println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, output_vec_c, outputs[0], options, stock_symbol
                        );*/
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
    fn test_ultosc_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) = rust_ultosc(&inputs_rust, &options, None)
                    .expect("Rust ULTOSC indicator failed");

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

                let (first_outputs, mut state) = rust_ultosc(&chunk_inputs, &options, None)
                    .expect("Rust ULTOSC indicator failed");
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
                        .expect("ULTOSC batch indicator failed");
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
    fn test_ultosc_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::ultosc::indicator_by_assets;

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
                .expect("SIMD by assets ULTOSC indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low, stock_close)) in
                stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [
                    stock_high.as_slice(),
                    stock_low.as_slice(),
                    stock_close.as_slice(),
                ];
                let (regular_results, _) = rust_ultosc(&stock_inputs, &options, None)
                    .expect("Regular ULTOSC indicator failed");

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
                            "SIMD by assets ULTOSC has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets ULTOSC has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for ULTOSC
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

        println!("✓ All SIMD by assets vs Regular ULTOSC database tests passed!");
    }

    #[test]
    fn test_ultosc_simd_by_assets_state_handover() {
        use tulip_rs::indicators::ultosc::indicator_by_assets;

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

        const CHUNK_SIZE: usize = 2000;

        // Check if we have enough data for chunking
        let min_len = stock_data
            .iter()
            .map(|(_, h, l, c)| h.len().min(l.len()).min(c.len()))
            .min()
            .unwrap_or(0);

        if min_len < CHUNK_SIZE * 2 {
            println!(
                "Skipping state handover test - insufficient data (need {}, have {})",
                CHUNK_SIZE * 2,
                min_len
            );
            return;
        }

        for options in OPTIONS_LIST {
            // Split data into two chunks for each stock
            let mut first_chunks: Vec<(&[f64], &[f64], &[f64])> = Vec::new();
            let mut second_chunks: Vec<(&[f64], &[f64], &[f64])> = Vec::new();

            for (_, high, low, close) in &stock_data {
                let (high1, high2) = high.split_at(CHUNK_SIZE);
                let (low1, low2) = low.split_at(CHUNK_SIZE);
                let (close1, close2) = close.split_at(CHUNK_SIZE);

                first_chunks.push((high1, low1, close1));
                second_chunks.push((high2, low2, close2));
            }

            // Prepare inputs for first chunk (SIMD by assets format)
            let inputs_first: [&[&[f64]; 3]; 4] = [
                &[first_chunks[0].0, first_chunks[0].1, first_chunks[0].2],
                &[first_chunks[1].0, first_chunks[1].1, first_chunks[1].2],
                &[first_chunks[2].0, first_chunks[2].1, first_chunks[2].2],
                &[first_chunks[3].0, first_chunks[3].1, first_chunks[3].2],
            ];

            // Process first chunk with SIMD by assets
            let (first_results, states) = indicator_by_assets::<4>(&inputs_first, &options, None)
                .expect("SIMD by assets first chunk failed");

            // Process second chunk using state handover for each asset
            let mut combined_results = Vec::new();
            for (asset_idx, mut state) in states.into_iter().enumerate() {
                let inputs_second = [
                    second_chunks[asset_idx].0,
                    second_chunks[asset_idx].1,
                    second_chunks[asset_idx].2,
                ];
                let second_result = state
                    .batch_indicator(&inputs_second, None)
                    .expect("State handover failed");

                // Combine results
                let mut combined = first_results[asset_idx][0].clone(); // Get first (and only) output
                combined.extend(second_result[0].iter().cloned());
                combined_results.push(combined);
            }

            // Compare against regular indicator for each asset
            for (asset_idx, (stock_symbol, stock_high, stock_low, stock_close)) in
                stock_data.iter().enumerate()
            {
                let stock_inputs = [
                    stock_high.as_slice(),
                    stock_low.as_slice(),
                    stock_close.as_slice(),
                ];
                let (regular_results, _) = rust_ultosc(&stock_inputs, &options, None)
                    .expect("Regular ULTOSC indicator failed");

                let combined_values = &combined_results[asset_idx];
                let regular_result = &regular_results[0];

                assert_eq!(
                    combined_values.len(),
                    regular_result.len(),
                    "Stock {}: Length mismatch for options {:?}",
                    stock_symbol,
                    options
                );

                for (i, (&combined_val, &regular_val)) in
                    combined_values.iter().zip(regular_result).enumerate()
                {
                    assert!(
                        approx_eq!(f64, combined_val, regular_val, epsilon = 1e-12),
                        "Stock {}: SIMD by assets state handover mismatch at index {}: Combined={}, Regular={}",
                        stock_symbol,
                        i,
                        combined_val,
                        regular_val
                    );
                }
            }
        }

        println!("✓ All SIMD by assets state handover ULTOSC database tests passed!");
    }

    #[test]
    fn test_ultosc_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Using all 4 options from OPTIONS_LIST
        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_idx, (_stock_symbol, stock_data)) in data.iter().take(4).enumerate() {
            let (high, low, close) = get_hlc_arrays(stock_data);

            // Test with SIMD by options (4-wide)
            let inputs = [&high[..], &low[..], &close[..]];
            let (simd_result, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by options ULTOSC indicator failed");

            // Compare against regular indicator for each option
            for (option_idx, option) in OPTIONS_LIST.iter().enumerate() {
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (regular_result, _) = rust_ultosc(&inputs_rust, option, None)
                    .expect("Regular ULTOSC indicator failed");

                let simd_values = &simd_result[option_idx][0]; // Get first (and only) output
                assert_eq!(
                    simd_values.len(),
                    regular_result[0].len(),
                    "Stock {}: Option {}: Length mismatch",
                    stock_idx,
                    option_idx
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_values.iter().zip(&regular_result[0]).enumerate()
                {
                    assert!(
                        approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12),
                        "Stock {}: Option {}: Mismatch at index {}: SIMD={}, Regular={}",
                        stock_idx,
                        option_idx,
                        i,
                        simd_val,
                        regular_val
                    );
                }
            }
        }

        println!("✓ All SIMD by options vs Regular ULTOSC database tests passed!");
    }

    #[test]
    fn test_ultosc_simd_state_handover_by_options() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        const CHUNK_SIZE: usize = 2000;
        // Using all 4 options from OPTIONS_LIST
        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_idx, (_stock_symbol, stock_data)) in data.iter().take(4).enumerate() {
            let (high, low, close) = get_hlc_arrays(stock_data);

            if high.len() < CHUNK_SIZE {
                continue; // Skip if not enough data
            }

            // Split data into two chunks
            let (high1, high2) = high.split_at(CHUNK_SIZE);
            let (low1, low2) = low.split_at(CHUNK_SIZE);
            let (close1, close2) = close.split_at(CHUNK_SIZE);

            // Process first chunk with SIMD by options
            let inputs1 = [high1, low1, close1];
            let (first_results, states) = indicator_by_options::<4>(&inputs1, &options_4, None)
                .expect("SIMD by options first chunk failed");

            // Process second chunk using state handover for each option
            let mut combined_results = Vec::new();
            for (option_idx, mut state) in states.into_iter().enumerate() {
                let inputs2 = [high2, low2, close2];
                let second_result = state
                    .batch_indicator(&inputs2, None)
                    .expect("State handover failed");

                // Combine results
                let mut combined = first_results[option_idx][0].clone(); // Get first output
                combined.extend(second_result[0].iter().cloned());
                combined_results.push(combined);
            }

            // Compare against regular indicator for each option
            for (option_idx, option) in OPTIONS_LIST.iter().enumerate() {
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (regular_result, _) = rust_ultosc(&inputs_rust, option, None)
                    .expect("Regular ULTOSC indicator failed");

                let combined_values = &combined_results[option_idx];
                assert_eq!(
                    combined_values.len(),
                    regular_result[0].len(),
                    "Stock {}: Option {}: Length mismatch",
                    stock_idx,
                    option_idx
                );

                for (i, (&combined_val, &regular_val)) in
                    combined_values.iter().zip(&regular_result[0]).enumerate()
                {
                    assert!(
                        approx_eq!(f64, combined_val, regular_val, epsilon = 1e-12),
                        "Stock {}: Option {}: State handover mismatch at index {}: Combined={}, Regular={}",
                        stock_idx,
                        option_idx,
                        i,
                        combined_val,
                        regular_val
                    );
                }
            }
        }

        println!("✓ All SIMD state handover by options ULTOSC database tests passed!");
    }

    fn get_hlc_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }

    // -------------------------------------------------------------------------
    // Optional outputs: TR and BP length checks; TR matches standalone rust_tr
    // across the full output range (warmup + main loop).
    // -------------------------------------------------------------------------

    #[test]
    fn test_ultosc_optional_outputs() {
        use tulip_rs::indicators::tr::indicator as rust_tr;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                if n < min_data(&options) {
                    continue;
                }

                // ULTOSC with both optional outputs enabled.
                let (ultosc_outputs, _) = rust_ultosc(&inputs, &options, Some(&[true, true]))
                    .expect("ULTOSC with optional outputs failed");

                // Standalone TR (OPTIONS_WIDTH = 0).
                let (tr_outputs, _) =
                    rust_tr(&inputs, &[], None).expect("Rust TR indicator failed");

                let expected_tr_len = n - 1;

                // TR and BP cover all bars (warmup + main loop), same length as standalone TR.
                assert_eq!(
                    ultosc_outputs[1].len(),
                    expected_tr_len,
                    "TR length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    ultosc_outputs[2].len(),
                    expected_tr_len,
                    "BP length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    tr_outputs[0].len(),
                    expected_tr_len,
                    "standalone TR length mismatch: stock={stock_symbol}, options={options:?}"
                );

                // TR is filled across the full range — compare all values.
                for i in 0..expected_tr_len {
                    if !approx_eq!(f64, ultosc_outputs[1][i], tr_outputs[0][i], epsilon = 1e-12) {
                        let start = if i >= 10 { i - 10 } else { 0 };
                        let end = if ultosc_outputs[1].len() - 10 > i {
                            i + 10
                        } else {
                            ultosc_outputs[1].len()
                        };
                        println!(
                            "Test failed at index {i}: \nULTOSC TR = {:?}, \n\nStandalone TR = {:?}, Options = {:?}",
                            &ultosc_outputs[1][start..end], &tr_outputs[0][start..end], options
                        );
                        panic!(
                            "TR mismatch at index {i}: ultosc_tr={}, standalone_tr={}, stock={stock_symbol}, options={options:?}",
                            ultosc_outputs[1][i], tr_outputs[0][i]
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets optional outputs: TR and BP from indicator_by_assets must
    // match the scalar indicator's optional TR and BP for every stock.
    // -------------------------------------------------------------------------

    #[test]
    fn test_ultosc_simd_by_assets_optional_outputs() {
        use tulip_rs::indicators::ultosc::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Collect the first 4 stocks.
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, d)| {
                let (high, low, close) = get_hlc_arrays(d);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            // Build the SIMD by-assets input array.
            let inputs: [&[&[f64]; 3]; 4] = [
                &[&stock_data[0].1, &stock_data[0].2, &stock_data[0].3],
                &[&stock_data[1].1, &stock_data[1].2, &stock_data[1].3],
                &[&stock_data[2].1, &stock_data[2].2, &stock_data[2].3],
                &[&stock_data[3].1, &stock_data[3].2, &stock_data[3].3],
            ];

            // Run SIMD with both optional outputs (TR=true, BP=true).
            let (simd_results, _) =
                indicator_by_assets::<4>(&inputs, &options, Some(&[true, true]))
                    .expect("SIMD by-assets ULTOSC with optional outputs failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                // Run scalar indicator with both optional outputs enabled.
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_results, _) =
                    rust_ultosc(&scalar_inputs, &options, Some(&[true, true]))
                        .expect("Scalar ULTOSC with optional outputs failed");

                // --- ULTOSC (primary output) ---
                let simd_ultosc = &simd_results[asset_idx][0];
                let scalar_ultosc = &scalar_results[0];
                assert_eq!(
                    simd_ultosc.len(),
                    scalar_ultosc.len(),
                    "ULTOSC length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_ultosc.iter().zip(scalar_ultosc).enumerate() {
                    assert!(
                        approx_eq!(f64, sv, rv, epsilon = 1e-12),
                        "ULTOSC mismatch at index {i}: simd={sv}, scalar={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                // --- TR optional output ---
                let simd_tr = &simd_results[asset_idx][1];
                let scalar_tr = &scalar_results[1];
                assert_eq!(
                    simd_tr.len(),
                    scalar_tr.len(),
                    "TR length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_tr.iter().zip(scalar_tr).enumerate() {
                    if !approx_eq!(f64, sv, rv, epsilon = 1e-12) {
                        let start = i.saturating_sub(5);
                        let end = (i + 6).min(simd_tr.len());
                        println!(
                            "TR mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                            &simd_tr[start..end],
                            &scalar_tr[start..end]
                        );
                        panic!(
                            "TR mismatch at index {i}: simd={sv}, scalar={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }

                // --- BP optional output ---
                let simd_bp = &simd_results[asset_idx][2];
                let scalar_bp = &scalar_results[2];
                assert_eq!(
                    simd_bp.len(),
                    scalar_bp.len(),
                    "BP length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_bp.iter().zip(scalar_bp).enumerate() {
                    if !approx_eq!(f64, sv, rv, epsilon = 1e-12) {
                        let start = i.saturating_sub(5);
                        let end = (i + 6).min(simd_bp.len());
                        println!(
                            "BP mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                            &simd_bp[start..end],
                            &scalar_bp[start..end]
                        );
                        panic!(
                            "BP mismatch at index {i}: simd={sv}, scalar={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }

                println!(
                    "✓ SIMD by-assets optional outputs match scalar for stock={stock_symbol}, options={options:?}"
                );
            }
        }

        println!("✓ All SIMD by-assets optional output ULTOSC tests passed!");
    }

    // -------------------------------------------------------------------------
    // SIMD by-options optional outputs: TR and BP from indicator_by_options must
    // match the scalar indicator's optional TR and BP for every option set.
    // -------------------------------------------------------------------------

    #[test]
    fn test_ultosc_simd_by_options_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (_stock_symbol, stock_data) in data.iter().take(4) {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Run SIMD by-options with both optional outputs enabled.
            let (simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(&[true, true]))
                    .expect("SIMD by-options ULTOSC with optional outputs failed");

            for (option_idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Run scalar indicator for the same option set with optional outputs.
                let (scalar_results, _) = rust_ultosc(&inputs, options, Some(&[true, true]))
                    .expect("Scalar ULTOSC with optional outputs failed");

                // --- ULTOSC (primary output) ---
                let simd_ultosc = &simd_results[option_idx][0];
                let scalar_ultosc = &scalar_results[0];
                assert_eq!(
                    simd_ultosc.len(),
                    scalar_ultosc.len(),
                    "ULTOSC length mismatch: option_idx={option_idx}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_ultosc.iter().zip(scalar_ultosc).enumerate() {
                    assert!(
                        approx_eq!(f64, sv, rv, epsilon = 1e-12),
                        "ULTOSC mismatch at index {i}: simd={sv}, scalar={rv}, option_idx={option_idx}"
                    );
                }

                // --- TR optional output ---
                let simd_tr = &simd_results[option_idx][1];
                let scalar_tr = &scalar_results[1];
                assert_eq!(
                    simd_tr.len(),
                    scalar_tr.len(),
                    "TR length mismatch: option_idx={option_idx}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_tr.iter().zip(scalar_tr).enumerate() {
                    if !approx_eq!(f64, sv, rv, epsilon = 1e-12) {
                        let start = i.saturating_sub(5);
                        let end = (i + 6).min(simd_tr.len());
                        println!(
                            "TR mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                            &simd_tr[start..end],
                            &scalar_tr[start..end]
                        );
                        panic!(
                            "TR mismatch at index {i}: simd={sv}, scalar={rv}, option_idx={option_idx}"
                        );
                    }
                }

                // --- BP optional output ---
                let simd_bp = &simd_results[option_idx][2];
                let scalar_bp = &scalar_results[2];
                assert_eq!(
                    simd_bp.len(),
                    scalar_bp.len(),
                    "BP length mismatch: option_idx={option_idx}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_bp.iter().zip(scalar_bp).enumerate() {
                    if !approx_eq!(f64, sv, rv, epsilon = 1e-12) {
                        let start = i.saturating_sub(5);
                        let end = (i + 6).min(simd_bp.len());
                        println!(
                            "BP mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                            &simd_bp[start..end],
                            &scalar_bp[start..end]
                        );
                        panic!(
                            "BP mismatch at index {i}: simd={sv}, scalar={rv}, option_idx={option_idx}"
                        );
                    }
                }

                println!(
                    "✓ SIMD by-options optional outputs match scalar for option_idx={option_idx}, options={options:?}"
                );
            }
        }

        println!("✓ All SIMD by-options optional output ULTOSC tests passed!");
    }
}
