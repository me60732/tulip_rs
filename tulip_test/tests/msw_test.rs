#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::msw::{indicator as rust_msw, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_msw, ti_msw_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILON: f64 = 1e-3;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [10.0], [14.0], [20.0]];

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
    fn test_msw_indicator() {
        // Use the same input data as in the benchmarks
        let close = expand_close();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

            // Determine the offset required by the C MSW function
            let start_index = unsafe { ti_msw_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_msw_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut sine_output_vec_c = vec![0.0_f64; output_len_c];
            let mut lead_output_vec_c = vec![0.0_f64; output_len_c];
            let sine_ptr: *mut f64 = sine_output_vec_c.as_mut_ptr();
            let lead_ptr: *mut f64 = lead_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![sine_ptr, lead_ptr];
            let ret = unsafe {
                ti_msw(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_msw returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice()];
            let (indicators, _) =
                rust_msw(&inputs_rust, &options, None).expect("Rust MSW indicator failed");

            let output_len_rust = indicators[0].len();

            // Compare the Sine outputs in reverse
            for (i, (&c_val, &rust_val)) in sine_output_vec_c
                .iter()
                .rev()
                .take(output_len_rust)
                .zip(indicators[0].iter().rev())
                .enumerate()
            {
                let index = output_len_rust - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust MSW_SINE has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust MSW has infinity at index {}: Rust = {}",
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
                    // Adjust epsilon if needed
                    /*println!(
                        "Test failed at index {}: \nC Sine = {:?}, \nRust Sine = {:?}, Options = {:?}",
                        index, sine_output_vec_c, indicators[0], options
                    );*/
                    panic!(
                        "Mismatch at index {}: C Sine = {}, Rust Sine = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }

            // Compare the Lead outputs in reverse
            for (i, (&c_val, &rust_val)) in lead_output_vec_c
                .iter()
                .rev()
                .take(output_len_rust)
                .zip(indicators[1].iter().rev())
                .enumerate()
            {
                let index = output_len_rust - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust MSW_LEAD has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust MSW has infinity at index {}: Rust = {}",
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
                    // Adjust epsilon if needed
                    /*println!(
                        "Test failed at index {}: \nC Lead = {:?}, \nRust Lead = {:?}, Options = {:?}",
                        index, lead_output_vec_c, indicators[1], options
                    );*/
                    panic!(
                        "Mismatch at index {}: C Lead = {}, Rust Lead = {}, Options = {:?}",
                        index, c_val, rust_val, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_msw_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr()];

                let start_index = unsafe { ti_msw_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_msw_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut sine_output_vec_c = vec![0.0_f64; output_len_c];
                let mut lead_output_vec_c = vec![0.0_f64; output_len_c];
                let sine_ptr: *mut f64 = sine_output_vec_c.as_mut_ptr();
                let lead_ptr: *mut f64 = lead_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![sine_ptr, lead_ptr];
                let ret = unsafe {
                    ti_msw(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_msw returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice()];
                let (indicators, _) =
                    rust_msw(&inputs_rust, &options, None).expect("Rust MSW indicator failed");

                let output_len_rust = indicators[0].len();

                // Compare Sine outputs
                for (i, (&c_val, &rust_val)) in sine_output_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust)
                    .zip(indicators[0].iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() {
                        panic!(
                            "Rust MSW_SINE has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Skip if only C has NaN (C bug)
                    if c_val.is_nan() && !rust_val.is_nan() {
                        continue;
                    }

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILON) {
                        /*println!(
                            "Test failed at index {}: \nC Sine = {:?}, \n\nRust Sine = {:?}, Options = {:?}, Stock: {}",
                            index, sine_output_vec_c, indicators[0], options, stock_symbol
                        );*/
                        panic!(
                            "Sine mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }

                // Compare Lead outputs
                for (i, (&c_val, &rust_val)) in lead_output_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust)
                    .zip(indicators[1].iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() {
                        panic!(
                            "Rust MSW_LEAD has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Skip if only C has NaN (C bug)
                    if c_val.is_nan() && !rust_val.is_nan() {
                        continue;
                    }

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILON) {
                        /*println!(
                            "Test failed at index {}: \nC Lead = {:?}, \n\nRust Lead = {:?}, Options = {:?}, Stock: {}",
                            index, lead_output_vec_c, indicators[1], options, stock_symbol
                        );*/
                        panic!(
                            "Lead mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }
            }
        }
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    #[test]
    fn test_msw_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let inputs_rust = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = rust_msw(&inputs_rust, &options, None)
                    .expect("Failed to run MSW indicator on full data");

                // Process in batches
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice()];

                let (first_outputs, mut state) = rust_msw(&chunk_inputs, &options, None)
                    .expect("Failed to run MSW indicator on first chunk");
                for output_idx in 0..first_outputs.len() {
                    batch_full_outputs[output_idx].extend_from_slice(&first_outputs[output_idx]);
                }

                // Process remaining data in chunks using state
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for close_chunk in close_chunks.by_ref() {
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("MSW batch indicator failed");
                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                // Process remainder if any
                let close_rem = close_chunks.remainder();
                if !close_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("MSW batch indicator failed");
                    for output_idx in 0..chunk_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&chunk_outputs[output_idx]);
                    }
                }

                // Compare all outputs
                for output_idx in 0..full_outputs.len() {
                    assert_eq!(
                        full_outputs[output_idx].len(),
                        batch_full_outputs[output_idx].len(),
                        "Output {} length mismatch for stock {} with options {:?}: full={}, batch={}",
                        output_idx,
                        stock_symbol,
                        options,
                        full_outputs[output_idx].len(),
                        batch_full_outputs[output_idx].len()
                    );

                    for (i, (&full_val, &batch_val)) in full_outputs[output_idx]
                        .iter()
                        .zip(batch_full_outputs[output_idx].iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, full_val, batch_val, epsilon = EPSILON) {
                            panic!(
                                "Mismatch in MSW output {} at index {}: full = {}, batch = {}, Stock: {}, Options: {:?}",
                                output_idx, i, full_val, batch_val, stock_symbol, options
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_msw_simd_by_assets() {
        use tulip_rs::indicators::msw::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
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
                .expect("SIMD by assets MSW indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) =
                    rust_msw(&stock_inputs, &options, None).expect("Regular MSW indicator failed");

                // MSW has 2 outputs: sine and lead
                for output_idx in 0..2 {
                    let output_name = if output_idx == 0 { "sine" } else { "lead" };
                    let simd_result = &simd_results[stock_idx][output_idx];
                    let regular_result = &regular_results[output_idx];

                    // Compare output lengths
                    assert_eq!(
                        simd_result.len(),
                        regular_result.len(),
                        "Output {} ({}) length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                        output_idx,
                        output_name,
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
                                "SIMD by assets MSW {} has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                output_name, i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets MSW {} has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                output_name, i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for MSW
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "MSW {} mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                output_name, i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    println!(
                        "✓ SIMD by assets vs Regular test passed for stock {} {} with options {:?}",
                        stock_symbol, output_name, options
                    );
                }
            }
        }

        println!("✓ All SIMD by assets vs Regular MSW database tests passed!");
    }

    #[test]
    fn test_msw_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::msw::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(&stock_data);
            let inputs = [close.as_slice()];

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (all_simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD MSW 4-wide failed");

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_msw(&inputs, options, None).expect("Regular MSW indicator failed");

                let simd_sine = &all_simd_results[idx][0];
                let simd_lead = &all_simd_results[idx][1];
                let regular_sine = &regular_results[0];
                let regular_lead = &regular_results[1];

                // Compare sine output lengths
                assert_eq!(
                    simd_sine.len(),
                    regular_sine.len(),
                    "Sine output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_sine.len(),
                    regular_sine.len()
                );

                // Compare lead output lengths
                assert_eq!(
                    simd_lead.len(),
                    regular_lead.len(),
                    "Lead output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_lead.len(),
                    regular_lead.len()
                );

                // Compare sine values
                for (i, (&simd_val, &regular_val)) in
                    simd_sine.iter().zip(regular_sine.iter()).enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD MSW Sine has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD MSW Sine has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Sine mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare lead values
                for (i, (&simd_val, &regular_val)) in
                    simd_lead.iter().zip(regular_lead.iter()).enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD MSW Lead has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD MSW Lead has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Lead mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular MSW database tests passed!");
    }
}
