#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::trix::{indicator as rust_trix, min_data, TIndicatorState};
    use tulip_rs::indicators::trix::{indicator_by_assets, indicator_by_options};
    use tulip_test::c_bindings::{
        ti_dema, ti_dema_start, ti_ema, ti_ema_start, ti_tema, ti_tema_start, ti_trix,
        ti_trix_start,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILION: f64 = 1e-10;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [9.0], [14.0], [15.0], [20.0], [30.0]];

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
    fn test_trix_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C TRIX function
            let start_index = unsafe { ti_trix_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_trix_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut trix_output_vec_c = vec![0.0_f64; output_len_c];
            let trix_ptr: *mut f64 = trix_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![trix_ptr];
            let ret = unsafe {
                ti_trix(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_trix returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (outputs, _) =
                rust_trix(&inputs_rust, &options, None).expect("Rust TRIX indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in trix_output_vec_c
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
                        "Rust TRIX has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust TRIX has infinity at index {}: Rust = {}, Options = {:?}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILION) {
                    // Adjust epsilon if needed
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, trix_output_vec_c, outputs[0], options
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
    fn test_trix_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_trix_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_trix_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_trix(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_trix returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (outputs, _) =
                    rust_trix(&inputs_rust, &options, None).expect("Rust TRIX indicator failed");

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
                            "Rust TRIX has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust TRIX has infinity at index {}: Rust = {}, Options = {:?}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILION) {
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
    fn test_trix_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_trix(&inputs_rust, &options, None).expect("Rust TRIX indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) =
                    rust_trix(&chunk_inputs, &options, None).expect("Rust TRIX indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("TRIX batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("TRIX batch indicator failed");
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

    #[test]
    fn test_trix_simd_by_assets() {
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
                .expect("SIMD TRIX indicator failed");

            // Run regular implementation for comparison
            let inputs_rust = [close.as_slice()];
            let (regular_outputs, _) =
                rust_trix(&inputs_rust, &options, None).expect("Regular TRIX indicator failed");

            // Compare each SIMD asset output with regular output
            for (asset_idx, simd_output_data) in simd_outputs.iter().enumerate() {
                let simd_output = &simd_output_data[0];
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
                            "SIMD TRIX has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TRIX has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
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
    fn test_trix_simd_by_assets_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Group stocks in sets of 4
        let stock_data: Vec<_> = data.iter().collect();
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
                    .expect("SIMD TRIX indicator failed");

                // Compare each asset's SIMD output with its regular output
                for (asset_idx, close_data) in padded_close.iter().enumerate().take(chunk.len()) {
                    let inputs_rust = [close_data.as_slice()];
                    let (regular_outputs, _) = rust_trix(&inputs_rust, &options, None)
                        .expect("Regular TRIX indicator failed");

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
                                "SIMD TRIX has NaN at index {} for asset {}: SIMD = {}",
                                i, asset_idx, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD TRIX has infinity at index {} for asset {}: SIMD = {}",
                                i, asset_idx, simd_val
                            );
                        }

                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
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
    fn test_trix_simd_by_assets_optional_outputs() {
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for SIMD (4 assets with same data)
            let inputs: [&[&[f64]; 1]; 4] = [
                &[close.as_slice()],
                &[close.as_slice()],
                &[close.as_slice()],
                &[close.as_slice()],
            ];

            // Test with optional outputs (all three outputs)
            let (simd_outputs_opt, _) =
                indicator_by_assets::<4>(&inputs, &options, Some(&[true, true, true]))
                    .expect("SIMD TRIX indicator with optional outputs failed");

            // Run regular implementation for comparison with optional outputs
            let inputs_rust = [close.as_slice()];
            let (regular_outputs_opt, _) =
                rust_trix(&inputs_rust, &options, Some(&[true, true, true]))
                    .expect("Regular TRIX indicator with optional outputs failed");

            // Compare each SIMD asset output with regular output
            for (asset_idx, simd_output_opt_data) in simd_outputs_opt.iter().enumerate() {
                // Compare TRIX output (index 0)
                let simd_trix = &simd_output_opt_data[0];
                let regular_trix = &regular_outputs_opt[0];

                assert_eq!(
                    simd_trix.len(),
                    regular_trix.len(),
                    "TRIX output length mismatch for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                    asset_idx,
                    simd_trix.len(),
                    regular_trix.len(),
                    options
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_trix.iter().zip(regular_trix.iter()).enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD TRIX output has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TRIX output has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                            i, asset_idx, simd_val, options
                        );
                    }

                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "TRIX output mismatch at index {} for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                            i, asset_idx, simd_val, regular_val, options
                        );
                    }
                }

                // Compare second EMA output (index 1) if it exists
                if simd_outputs_opt[asset_idx].len() > 1 && regular_outputs_opt.len() > 1 {
                    let simd_ema2 = &simd_outputs_opt[asset_idx][1];
                    let regular_ema2 = &regular_outputs_opt[1];

                    // Skip empty optional outputs
                    if !simd_ema2.is_empty() && !regular_ema2.is_empty() {
                        assert_eq!(
                            simd_ema2.len(),
                            regular_ema2.len(),
                            "Second EMA output length mismatch for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                            asset_idx,
                            simd_ema2.len(),
                            regular_ema2.len(),
                            options
                        );

                        for (i, (&simd_val, &regular_val)) in
                            simd_ema2.iter().zip(regular_ema2.iter()).enumerate()
                        {
                            if simd_val.is_nan() {
                                panic!(
                                    "SIMD second EMA output has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                                    i, asset_idx, simd_val, options
                                );
                            }

                            if simd_val.is_infinite() {
                                panic!(
                                    "SIMD second EMA output has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                                    i, asset_idx, simd_val, options
                                );
                            }

                            if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                                panic!(
                                    "Second EMA output mismatch at index {} for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                                    i, asset_idx, simd_val, regular_val, options
                                );
                            }
                        }
                    }
                }

                // Compare first EMA output (index 2) if it exists
                if simd_outputs_opt[asset_idx].len() > 2 && regular_outputs_opt.len() > 2 {
                    let simd_ema1 = &simd_outputs_opt[asset_idx][2];
                    let regular_ema1 = &regular_outputs_opt[2];

                    // Skip empty optional outputs
                    if !simd_ema1.is_empty() && !regular_ema1.is_empty() {
                        assert_eq!(
                            simd_ema1.len(),
                            regular_ema1.len(),
                            "First EMA output length mismatch for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                            asset_idx,
                            simd_ema1.len(),
                            regular_ema1.len(),
                            options
                        );

                        for (i, (&simd_val, &regular_val)) in
                            simd_ema1.iter().zip(regular_ema1.iter()).enumerate()
                        {
                            if simd_val.is_nan() {
                                panic!(
                                    "SIMD first EMA output has NaN at index {} for asset {}: SIMD = {}, Options = {:?}",
                                    i, asset_idx, simd_val, options
                                );
                            }

                            if simd_val.is_infinite() {
                                panic!(
                                    "SIMD first EMA output has infinity at index {} for asset {}: SIMD = {}, Options = {:?}",
                                    i, asset_idx, simd_val, options
                                );
                            }

                            if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                                panic!(
                                    "First EMA output mismatch at index {} for asset {}: SIMD = {}, Regular = {}, Options = {:?}",
                                    i, asset_idx, simd_val, regular_val, options
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_trix_simd_by_assets_database_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Group stocks in sets of 4
        let stock_data: Vec<_> = data.iter().collect();
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

                // Get SIMD by assets result with optional outputs
                let (simd_results_opt, _) =
                    indicator_by_assets::<4>(&inputs, &options, Some(&[true, true, true]))
                        .expect("SIMD by assets TRIX indicator with optional outputs failed");

                // Compare each SIMD result with regular indicator for each stock
                for (asset_idx, close_data) in padded_close.iter().enumerate().take(chunk.len()) {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [close_data.as_slice()];
                    let (regular_results_opt, _) =
                        rust_trix(&stock_inputs, &options, Some(&[true, true, true]))
                            .expect("Regular TRIX indicator with optional outputs failed");

                    let simd_trix = &simd_results_opt[asset_idx][0];
                    let regular_trix = &regular_results_opt[0];

                    // Compare TRIX output lengths
                    assert_eq!(
                        simd_trix.len(),
                        regular_trix.len(),
                        "TRIX output length mismatch for asset {}: SIMD = {}, Regular = {}",
                        asset_idx,
                        simd_trix.len(),
                        regular_trix.len()
                    );

                    // Compare TRIX values
                    for (i, (&simd_val, &regular_val)) in
                        simd_trix.iter().zip(regular_trix.iter()).enumerate()
                    {
                        if simd_val.is_nan() {
                            panic!(
                                "SIMD TRIX has NaN at index {} for asset {}: SIMD = {}",
                                i, asset_idx, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD TRIX has infinity at index {} for asset {}: SIMD = {}",
                                i, asset_idx, simd_val
                            );
                        }

                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                            panic!(
                                "TRIX mismatch at index {} for asset {}: SIMD = {}, Regular = {}",
                                i, asset_idx, simd_val, regular_val
                            );
                        }
                    }

                    // Compare second EMA output if available
                    if simd_results_opt[asset_idx].len() > 1 && regular_results_opt.len() > 1 {
                        let simd_ema2 = &simd_results_opt[asset_idx][1];
                        let regular_ema2 = &regular_results_opt[1];

                        if !simd_ema2.is_empty() && !regular_ema2.is_empty() {
                            assert_eq!(
                                simd_ema2.len(),
                                regular_ema2.len(),
                                "Second EMA output length mismatch for asset {}: SIMD = {}, Regular = {}",
                                asset_idx,
                                simd_ema2.len(),
                                regular_ema2.len()
                            );

                            for (i, (&simd_val, &regular_val)) in
                                simd_ema2.iter().zip(regular_ema2.iter()).enumerate()
                            {
                                if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                                    panic!(
                                        "Second EMA mismatch at index {} for asset {}: SIMD = {}, Regular = {}",
                                        i, asset_idx, simd_val, regular_val
                                    );
                                }
                            }
                        }
                    }

                    // Compare first EMA output if available
                    if simd_results_opt[asset_idx].len() > 2 && regular_results_opt.len() > 2 {
                        let simd_ema1 = &simd_results_opt[asset_idx][2];
                        let regular_ema1 = &regular_results_opt[2];

                        if !simd_ema1.is_empty() && !regular_ema1.is_empty() {
                            assert_eq!(
                                simd_ema1.len(),
                                regular_ema1.len(),
                                "First EMA output length mismatch for asset {}: SIMD = {}, Regular = {}",
                                asset_idx,
                                simd_ema1.len(),
                                regular_ema1.len()
                            );

                            for (i, (&simd_val, &regular_val)) in
                                simd_ema1.iter().zip(regular_ema1.iter()).enumerate()
                            {
                                if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                                    panic!(
                                        "First EMA mismatch at index {} for asset {}: SIMD = {}, Regular = {}",
                                        i, asset_idx, simd_val, regular_val
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_trix_simd_by_options_vs_regular_database() {
        // by_options functionality is not available for trix
        // Test body commented out
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
                .expect("SIMD TRIX 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD TRIX 2-wide failed");

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
                let (regular_results, _) =
                    rust_trix(&inputs, options, None).expect("Regular TRIX indicator failed");

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
                            "SIMD TRIX has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TRIX has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular TRIX database tests passed!");
    }

    #[test]
    fn test_trix_simd_by_options_vs_regular_database_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            // Test with EMA, DEMA, and TEMA optional outputs
            let optional_outputs = Some(&[true, true, true][..]);

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD TRIX 4-wide with optional outputs failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) =
                indicator_by_options::<2>(&inputs, &options_2, optional_outputs)
                    .expect("SIMD TRIX 2-wide with optional outputs failed");

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
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_trix(&inputs, options, optional_outputs)
                    .expect("Regular TRIX indicator with optional outputs failed");

                let simd_trix_result = &all_simd_results[idx][0];
                let regular_trix_result = &regular_results[0];

                let simd_ema_result = &all_simd_results[idx][1];
                let regular_ema_result = &regular_results[1];

                let simd_dema_result = &all_simd_results[idx][2];
                let regular_dema_result = &regular_results[2];

                let simd_tema_result = &all_simd_results[idx][3];
                let regular_tema_result = &regular_results[3];

                // Compare TRIX output lengths
                assert_eq!(
                    simd_trix_result.len(),
                    regular_trix_result.len(),
                    "TRIX output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_trix_result.len(),
                    regular_trix_result.len()
                );

                // Compare EMA output lengths
                assert_eq!(
                    simd_ema_result.len(),
                    regular_ema_result.len(),
                    "EMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_ema_result.len(),
                    regular_ema_result.len()
                );

                // Compare DEMA output lengths
                assert_eq!(
                    simd_dema_result.len(),
                    regular_dema_result.len(),
                    "DEMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_dema_result.len(),
                    regular_dema_result.len()
                );

                // Compare TEMA output lengths
                assert_eq!(
                    simd_tema_result.len(),
                    regular_tema_result.len(),
                    "TEMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_tema_result.len(),
                    regular_tema_result.len()
                );

                // Compare TRIX values
                for (i, (&simd_val, &regular_val)) in simd_trix_result
                    .iter()
                    .zip(regular_trix_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD TRIX has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TRIX has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "TRIX mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare EMA values
                for (i, (&simd_val, &regular_val)) in simd_ema_result
                    .iter()
                    .zip(regular_ema_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD EMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD EMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "EMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare DEMA values
                for (i, (&simd_val, &regular_val)) in simd_dema_result
                    .iter()
                    .zip(regular_dema_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD DEMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD DEMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "DEMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare TEMA values
                for (i, (&simd_val, &regular_val)) in simd_tema_result
                    .iter()
                    .zip(regular_tema_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD TEMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TEMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILION) {
                        panic!(
                            "TEMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular TRIX database tests with optional outputs passed!"
        );
    }

    #[test]
    fn test_trix_tema_optional_output_vs_c_tulip() {
        const EPSILON: f64 = EPSILION;
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Get Rust TRIX with TEMA optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) = rust_trix(&inputs_rust, &options, Some(&[true, false, false]))
                .expect("Rust TRIX indicator failed");

            assert!(!outputs.is_empty(), "TRIX outputs should not be empty");
            assert!(
                outputs.len() >= 2,
                "TRIX should have at least 2 outputs when optional outputs enabled"
            );

            let rust_tema_output = &outputs[1]; // TEMA is at index 1

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_tema_output.is_empty(),
                "TEMA optional output vector should not be empty"
            );

            // Get C TEMA reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_tema_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_tema_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut tema_output_vec_c = vec![0.0_f64; output_len_c];
            let tema_ptr: *mut f64 = tema_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![tema_ptr];
            let ret = unsafe {
                ti_tema(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_tema returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in tema_output_vec_c
                .iter()
                .rev()
                .zip(rust_tema_output.iter().rev())
                .enumerate()
            {
                let index = rust_tema_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust TEMA optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust TEMA optional output has infinity at index {}: Rust = {}, Options = {:?}",
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
                    panic!(
                        "TEMA optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_trix_dema_optional_output_vs_c_tulip() {
        const EPSILON: f64 = EPSILION;
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Get Rust TRIX with DEMA optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) = rust_trix(&inputs_rust, &options, Some(&[false, true, false]))
                .expect("Rust TRIX indicator failed");

            assert!(!outputs.is_empty(), "TRIX outputs should not be empty");
            assert!(
                outputs.len() >= 3,
                "TRIX should have at least 3 outputs when optional outputs enabled"
            );

            let rust_dema_output = &outputs[2]; // DEMA is at index 2

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_dema_output.is_empty(),
                "DEMA optional output vector should not be empty"
            );

            // Get C DEMA reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_dema_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_dema_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut dema_output_vec_c = vec![0.0_f64; output_len_c];
            let dema_ptr: *mut f64 = dema_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![dema_ptr];
            let ret = unsafe {
                ti_dema(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_dema returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in dema_output_vec_c
                .iter()
                .rev()
                .zip(rust_dema_output.iter().rev())
                .enumerate()
            {
                let index = rust_dema_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust DEMA optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust DEMA optional output has infinity at index {}: Rust = {}, Options = {:?}",
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
                    panic!(
                        "DEMA optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_trix_ema_optional_output_vs_c_tulip() {
        const EPSILON: f64 = EPSILION;
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Get Rust TRIX with EMA optional output enabled
            let inputs_rust = [close.as_slice()];
            let (outputs, _) = rust_trix(&inputs_rust, &options, Some(&[false, false, true]))
                .expect("Rust TRIX indicator failed");

            assert!(!outputs.is_empty(), "TRIX outputs should not be empty");
            assert!(
                outputs.len() >= 4,
                "TRIX should have at least 4 outputs when optional outputs enabled"
            );

            let rust_ema_output = &outputs[3]; // EMA is at index 3

            // Panic if the optional output vector is empty (indicates a bug)
            assert!(
                !rust_ema_output.is_empty(),
                "EMA optional output vector should not be empty"
            );

            // Get C EMA reference implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
            let start_index = unsafe { ti_ema_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_ema_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut ema_output_vec_c = vec![0.0_f64; output_len_c];
            let ema_ptr: *mut f64 = ema_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![ema_ptr];
            let ret = unsafe {
                ti_ema(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_ema returned error code {}", ret);

            // Compare outputs from the end backwards
            for (i, (&c_val, &rust_val)) in ema_output_vec_c
                .iter()
                .rev()
                .zip(rust_ema_output.iter().rev())
                .enumerate()
            {
                let index = rust_ema_output.len() - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust EMA optional output has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust EMA optional output has infinity at index {}: Rust = {}, Options = {:?}",
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
                    panic!(
                        "EMA optional output mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    #[test]
    fn test_trix_database_optional_tema() {
        const EPSILON: f64 = EPSILION;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get TRIX with TEMA optional output
                let optional_outputs = Some(&[true, false, false][..]);
                let (trix_result, _) = tulip_rs::indicators::trix::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_tema = &trix_result[1];

                // Calculate expected TEMA using C Tulip ti_tema
                let tema_options = [options[0]];
                let start_index = unsafe { ti_tema_start(tema_options.as_ptr()) };
                assert!(start_index >= 0, "ti_tema_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_tema_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_tema_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_tema(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        tema_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_tema failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_tema.len().min(c_tema_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_tema.len() - 1 - i;
                    let c_idx = c_tema_output.len() - 1 - i;

                    let rust_val = rust_tema[rust_idx];
                    let c_val = c_tema_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust TEMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "TRIX TEMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
                        rust_idx,
                        options,
                        rust_val,
                        c_val,
                        (rust_val - c_val).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn test_trix_database_optional_dema() {
        const EPSILON: f64 = EPSILION;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get TRIX with DEMA optional output
                let optional_outputs = Some(&[false, true, false][..]);
                let (trix_result, _) = tulip_rs::indicators::trix::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_dema = &trix_result[2];

                // Calculate expected DEMA using C Tulip ti_dema
                let dema_options = [options[0]];
                let start_index = unsafe { ti_dema_start(dema_options.as_ptr()) };
                assert!(start_index >= 0, "ti_dema_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_dema_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_dema_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_dema(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        dema_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_dema failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_dema.len().min(c_dema_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_dema.len() - 1 - i;
                    let c_idx = c_dema_output.len() - 1 - i;

                    let rust_val = rust_dema[rust_idx];
                    let c_val = c_dema_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust DEMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "TRIX DEMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
                        rust_idx,
                        options,
                        rust_val,
                        c_val,
                        (rust_val - c_val).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn test_trix_database_optional_ema() {
        const EPSILON: f64 = EPSILION;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let close = get_close_array(stock_data);

            for &options in &OPTIONS_LIST {
                // Get TRIX with EMA optional output
                let optional_outputs = Some(&[false, false, true][..]);
                let (trix_result, _) = tulip_rs::indicators::trix::indicator(
                    &[&close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_ema = &trix_result[3];

                // Calculate expected EMA using C Tulip ti_ema
                let ema_options = [options[0]];
                let start_index = unsafe { ti_ema_start(ema_options.as_ptr()) };
                assert!(start_index >= 0, "ti_ema_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut c_ema_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_ema_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_ema(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        ema_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_ema failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_ema.len().min(c_ema_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_ema.len() - 1 - i;
                    let c_idx = c_ema_output.len() - 1 - i;

                    let rust_val = rust_ema[rust_idx];
                    let c_val = c_ema_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust EMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "TRIX EMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
                        rust_idx,
                        options,
                        rust_val,
                        c_val,
                        (rust_val - c_val).abs()
                    );
                }
            }
        }
    }
}
