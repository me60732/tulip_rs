#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::adx::{indicator as rust_adx, min_data, TIndicatorState};
    use tulip_rs::indicators::adx::{indicator_by_assets, indicator_by_options};
    use tulip_test::c_bindings::{
        ti_adx, ti_adx_start, ti_atr, ti_atr_start, ti_dx, ti_dx_start, ti_tr, ti_tr_start,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const EPSILON: f64 = 1e-12;
    const DX_EPSILON: f64 = 1e-12; // Use epsilon from dx_test.rs
    const ATR_EPSILON: f64 = 1e-12; // Use epsilon from atr_test.rs
    const TR_EPSILON: f64 = 1e-12; // Use epsilon from tr_test.rs
    const CHUNK_SIZE: usize = 100;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [24.0], [30.0]];

    fn get_hlc_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut close_vec = CLOSE.to_vec();
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        for _ in 0..200 {
            close_vec.extend_from_slice(&CLOSE);
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
        }
        (high_vec, low_vec, close_vec)
    }

    #[test]
    fn test_adx_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            // Determine the offset required by the C ADX function
            let start_index = unsafe { ti_adx_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_adx_start returned a negative index");
            let output_len = close.len() - (start_index as usize);

            // Run the C implementation
            let mut output_vec_c = vec![0.0_f64; output_len];
            let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
            let ret = unsafe {
                ti_adx(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_adx returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (outputs, _) =
                rust_adx(&inputs_rust, &options, None).expect("Rust ADX indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
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
                        "Rust ADX has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust ADX has infinity at index {}: Rust = {}, Options = {:?}",
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
                        "Test failed at index {}: \nC = {:?}, \n\n\nRust = {:?}, Options = {:?}",
                        index, output_vec_c, outputs[0], options
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
    fn test_adx_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

                let start_index = unsafe { ti_adx_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_adx_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_adx(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_adx returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (outputs, _) =
                    rust_adx(&inputs_rust, &options, None).expect("Rust ADX indicator failed");

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
                            "Rust ADX has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust ADX has infinity at index {}: Rust = {}, Options = {:?}, Stock: {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-12) {
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
    fn test_adx_dx_optional_output_vs_c_tulip() {
        // Test ADX's DX optional output against C Tulip's DX implementation
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            println!("Testing ADX DX optional output with options: {:?}", options);

            // Run the Rust implementation with DX optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (rust_outputs, _) = rust_adx(&inputs_rust, &options, Some(&[true, false, false]))
                .expect("Rust ADX indicator failed");

            // Extract the DX optional output (second output)
            let rust_dx = &rust_outputs[1];

            // Fail immediately if DX output is empty (indicator bug)
            if rust_dx.is_empty() {
                panic!(
                    "Rust DX optional output is empty with options {:?} - indicator bug in optional output handling",
                    options
                );
            }

            // Run the C implementation for DX
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            let start_index = unsafe { ti_dx_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_dx_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            let mut dx_output_vec_c = vec![0.0_f64; output_len_c];
            let dx_ptr: *mut f64 = dx_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![dx_ptr];

            let ret = unsafe {
                ti_dx(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_dx returned error code {}", ret);

            // Compare DX outputs from the end backwards for better alignment
            let comparison_length = rust_dx.len().min(dx_output_vec_c.len());

            for (i, (rust_val, c_val)) in rust_dx
                .iter()
                .rev()
                .zip(dx_output_vec_c.iter().rev())
                .take(comparison_length)
                .enumerate()
            {
                // Check for NaN or infinite values in Rust output (should not happen)
                if rust_val.is_nan() || rust_val.is_infinite() {
                    panic!(
                        "Rust DX optional output contains NaN/infinite value {} at reverse index {} for options {:?}",
                        rust_val, i, options
                    );
                }

                // Skip comparison if C output has NaN/infinite (C implementation bug)
                if c_val.is_nan() || c_val.is_infinite() {
                    continue;
                }

                if !approx_eq!(f64, *rust_val, *c_val, epsilon = EPSILON) {
                    panic!(
                        "DX optional output mismatch at reverse index {}: Rust = {}, C = {}, diff = {}, options = {:?}",
                        i, rust_val, c_val, (rust_val - c_val).abs(), options
                    );
                }
            }

            println!(
                "✓ DX optional output matches C Tulip for {} comparisons with options {:?}",
                comparison_length, options
            );
        }

        println!("✓ All ADX DX optional output vs C Tulip tests passed!");
    }

    #[test]
    fn test_adx_atr_optional_output_vs_c_tulip() {
        // Test ADX's ATR optional output against C Tulip's ATR implementation
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            println!(
                "Testing ADX ATR optional output with options: {:?}",
                options
            );

            // Run the Rust implementation with ATR optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (rust_outputs, _) = rust_adx(&inputs_rust, &options, Some(&[false, true, false]))
                .expect("Rust ADX indicator failed");

            // Extract the ATR optional output (third output)
            let rust_atr = &rust_outputs[2];

            // Fail immediately if ATR output is empty (indicator bug)
            if rust_atr.is_empty() {
                panic!(
                    "Rust ATR optional output is empty with options {:?} - indicator bug in optional output handling",
                    options
                );
            }

            // Run the C implementation for ATR
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            let start_index = unsafe { ti_atr_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_atr_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            let mut atr_output_vec_c = vec![0.0_f64; output_len_c];
            let atr_ptr: *mut f64 = atr_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![atr_ptr];

            let ret = unsafe {
                ti_atr(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_atr returned error code {}", ret);

            // Compare ATR outputs from the end backwards for better alignment
            let comparison_length = rust_atr.len().min(atr_output_vec_c.len());

            for (i, (rust_val, c_val)) in rust_atr
                .iter()
                .rev()
                .zip(atr_output_vec_c.iter().rev())
                .take(comparison_length)
                .enumerate()
            {
                // Check for NaN or infinite values in Rust output (should not happen)
                if rust_val.is_nan() || rust_val.is_infinite() {
                    panic!(
                        "Rust ATR optional output contains NaN/infinite value {} at reverse index {} for options {:?}",
                        rust_val, i, options
                    );
                }

                // Skip comparison if C output has NaN/infinite (C implementation bug)
                if c_val.is_nan() || c_val.is_infinite() {
                    continue;
                }

                if !approx_eq!(f64, *rust_val, *c_val, epsilon = EPSILON) {
                    panic!(
                        "ATR optional output mismatch at reverse index {}: Rust = {}, C = {}, diff = {}, options = {:?}",
                        i, rust_val, c_val, (rust_val - c_val).abs(), options
                    );
                }
            }

            println!(
                "✓ ATR optional output matches C Tulip for {} comparisons with options {:?}",
                comparison_length, options
            );
        }

        println!("✓ All ADX ATR optional output vs C Tulip tests passed!");
    }

    #[test]
    fn test_adx_tr_optional_output_vs_c_tulip() {
        // Test ADX's TR optional output against C Tulip's TR implementation
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            println!("Testing ADX TR optional output with options: {:?}", options);

            // Run the Rust implementation with TR optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (rust_outputs, _) = rust_adx(&inputs_rust, &options, Some(&[false, false, true]))
                .expect("Rust ADX indicator failed");

            // Extract the TR optional output (fourth output)
            let rust_tr = &rust_outputs[3];

            // Fail immediately if TR output is empty (indicator bug)
            if rust_tr.is_empty() {
                panic!(
                    "Rust TR optional output is empty with options {:?} - indicator bug in optional output handling",
                    options
                );
            }

            // Run the C implementation for TR - TR takes no options (empty options array)
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
            let tr_options: Vec<f64> = vec![];

            let start_index = unsafe { ti_tr_start(tr_options.as_ptr()) };
            assert!(start_index >= 0, "ti_tr_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            let mut tr_output_vec_c = vec![0.0_f64; output_len_c];
            let tr_ptr: *mut f64 = tr_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![tr_ptr];

            let ret = unsafe {
                ti_tr(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    tr_options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_tr returned error code {}", ret);

            // Compare TR outputs from the end backwards for better alignment
            let comparison_length = rust_tr.len().min(tr_output_vec_c.len());

            for (i, (rust_val, c_val)) in rust_tr
                .iter()
                .rev()
                .zip(tr_output_vec_c.iter().rev())
                .take(comparison_length)
                .enumerate()
            {
                // Check for NaN or infinite values in Rust output (should not happen)
                if rust_val.is_nan() || rust_val.is_infinite() {
                    panic!(
                        "Rust TR optional output contains NaN/infinite value {} at reverse index {} for options {:?}",
                        rust_val, i, options
                    );
                }

                // Skip comparison if C output has NaN/infinite (C implementation bug)
                if c_val.is_nan() || c_val.is_infinite() {
                    continue;
                }

                if !approx_eq!(f64, *rust_val, *c_val, epsilon = EPSILON) {
                    panic!(
                        "TR optional output mismatch at reverse index {}: Rust = {}, C = {}, diff = {}, options = {:?}",
                        i, rust_val, c_val, (rust_val - c_val).abs(), options
                    );
                }
            }

            println!(
                "✓ TR optional output matches C Tulip for {} comparisons with options {:?}",
                comparison_length, options
            );
        }

        println!("✓ All ADX TR optional output vs C Tulip tests passed!");
    }

    #[test]
    fn test_adx_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_adx(&inputs_rust, &options, None).expect("Rust ADX indicator failed");

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
                    rust_adx(&chunk_inputs, &options, None).expect("Rust ADX indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for ((high_chunk, low_chunk), close_chunk) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let high_vec = high_chunk.to_vec();
                    let low_vec = low_chunk.to_vec();
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                    ];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("Rust ADX batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();

                if !high_rem.is_empty() {
                    let high_vec = high_rem.to_vec();
                    let low_vec = low_rem.to_vec();
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                    ];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("Rust ADX batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Compare full output with batch output
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
    fn test_adx_dx_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                println!(
                    "Testing ADX DX optional output with database stock {} and options: {:?}",
                    stock_symbol, options
                );

                // Get Rust ADX with DX optional output enabled
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (rust_outputs, _) =
                    rust_adx(&inputs_rust, &options, Some(&[true, false, false]))
                        .expect("Rust ADX indicator with DX optional output failed");

                let rust_dx = &rust_outputs[1]; // DX is at index 1

                if rust_dx.is_empty() {
                    panic!(
                        "Rust DX optional output is empty for stock {}",
                        stock_symbol
                    );
                }

                // Get C Tulip DX output for comparison
                let dx_inputs_c = [high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let dx_start_index = unsafe { ti_dx_start(options.as_ptr()) };
                let dx_output_len = high.len() - (dx_start_index as usize);
                let mut c_dx = vec![0.0; dx_output_len];
                let mut dx_outputs_c = vec![c_dx.as_mut_ptr()];

                let ret = unsafe {
                    ti_dx(
                        high.len() as i32,
                        dx_inputs_c.as_ptr(),
                        options.as_ptr(),
                        dx_outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(
                    ret, 0,
                    "ti_dx returned error code {} for stock {}",
                    ret, stock_symbol
                );

                // Compare from the end backwards
                let compare_len = rust_dx.len().min(c_dx.len());
                for i in 0..compare_len {
                    let rust_idx = rust_dx.len() - 1 - i;
                    let c_idx = c_dx.len() - 1 - i;
                    let rust_val = rust_dx[rust_idx];
                    let c_val = c_dx[c_idx];

                    if !rust_val.is_finite() {
                        panic!(
                        "Rust DX output has NaN/infinity at index {} (from end): Rust = {} for stock {} options {:?}",
                        i, rust_val, stock_symbol, options
                    );
                    }

                    if !c_val.is_finite() {
                        continue; // Skip C library bugs
                    }

                    assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = DX_EPSILON),
                    "DX mismatch at index {} (from end): C = {}, Rust = {} for stock {} options {:?}",
                    i, c_val, rust_val, stock_symbol, options
                );
                }

                println!(
                    "✓ DX optional output validated for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All ADX DX optional output database tests passed!");
    }

    #[test]
    fn test_adx_atr_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                println!(
                    "Testing ADX ATR optional output with database stock {} and options: {:?}",
                    stock_symbol, options
                );

                // Get Rust ADX with ATR optional output enabled
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (rust_outputs, _) =
                    rust_adx(&inputs_rust, &options, Some(&[false, true, false]))
                        .expect("Rust ADX indicator with ATR optional output failed");

                let rust_atr = &rust_outputs[2]; // ATR is at index 2

                if rust_atr.is_empty() {
                    panic!(
                        "Rust ATR optional output is empty for stock {}",
                        stock_symbol
                    );
                }

                // Get C Tulip ATR output for comparison
                let atr_inputs_c = [high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let atr_start_index = unsafe { ti_atr_start(options.as_ptr()) };
                let atr_output_len = high.len() - (atr_start_index as usize);
                let mut c_atr = vec![0.0; atr_output_len];
                let mut atr_outputs_c = vec![c_atr.as_mut_ptr()];

                let ret = unsafe {
                    ti_atr(
                        high.len() as i32,
                        atr_inputs_c.as_ptr(),
                        options.as_ptr(),
                        atr_outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(
                    ret, 0,
                    "ti_atr returned error code {} for stock {}",
                    ret, stock_symbol
                );

                // Compare from the end backwards
                let compare_len = rust_atr.len().min(c_atr.len());
                for i in 0..compare_len {
                    let rust_idx = rust_atr.len() - 1 - i;
                    let c_idx = c_atr.len() - 1 - i;
                    let rust_val = rust_atr[rust_idx];
                    let c_val = c_atr[c_idx];

                    if !rust_val.is_finite() {
                        panic!(
                        "Rust ATR output has NaN/infinity at index {} (from end): Rust = {} for stock {} options {:?}",
                        i, rust_val, stock_symbol, options
                    );
                    }

                    if !c_val.is_finite() {
                        continue; // Skip C library bugs
                    }

                    assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = ATR_EPSILON),
                    "ATR mismatch at index {} (from end): C = {}, Rust = {} for stock {} options {:?}",
                    i, c_val, rust_val, stock_symbol, options
                );
                }

                println!(
                    "✓ ATR optional output validated for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All ADX ATR optional output database tests passed!");
    }

    #[test]
    fn test_adx_tr_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                println!(
                    "Testing ADX TR optional output with database stock {} and options: {:?}",
                    stock_symbol, options
                );

                // Get Rust ADX with TR optional output enabled
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (rust_outputs, _) =
                    rust_adx(&inputs_rust, &options, Some(&[false, false, true]))
                        .expect("Rust ADX indicator with TR optional output failed");

                let rust_tr = &rust_outputs[3]; // TR is at index 3

                if rust_tr.is_empty() {
                    panic!(
                        "Rust TR optional output is empty for stock {}",
                        stock_symbol
                    );
                }

                // Get C Tulip TR output for comparison
                let tr_inputs_c = [high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let tr_start_index = unsafe { ti_tr_start(std::ptr::null()) };
                let tr_output_len = high.len() - (tr_start_index as usize);
                let mut c_tr = vec![0.0; tr_output_len];
                let mut tr_outputs_c = vec![c_tr.as_mut_ptr()];

                let ret = unsafe {
                    ti_tr(
                        high.len() as i32,
                        tr_inputs_c.as_ptr(),
                        std::ptr::null(),
                        tr_outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(
                    ret, 0,
                    "ti_tr returned error code {} for stock {}",
                    ret, stock_symbol
                );

                // Compare from the end backwards
                let compare_len = rust_tr.len().min(c_tr.len());
                for i in 0..compare_len {
                    let rust_idx = rust_tr.len() - 1 - i;
                    let c_idx = c_tr.len() - 1 - i;
                    let rust_val = rust_tr[rust_idx];
                    let c_val = c_tr[c_idx];

                    if !rust_val.is_finite() {
                        panic!(
                        "Rust TR output has NaN/infinity at index {} (from end): Rust = {} for stock {} options {:?}",
                        i, rust_val, stock_symbol, options
                    );
                    }

                    if !c_val.is_finite() {
                        continue; // Skip C library bugs
                    }

                    assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = TR_EPSILON),
                    "TR mismatch at index {} (from end): C = {}, Rust = {} for stock {} options {:?}",
                    i, c_val, rust_val, stock_symbol, options
                );
                }

                println!(
                    "✓ TR optional output validated for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All ADX TR optional output database tests passed!");
    }

    #[test]
    fn test_adx_simd_vs_regular_database() {
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

        // Test each period
        for options in &OPTIONS_LIST {
            // Prepare inputs in the format expected by indicator_by_assets
            let inputs: [&[&[f64]; 3]; 4] = [
                &[
                    &stock_data[0].1, // high
                    &stock_data[0].2, // low
                    &stock_data[0].3, // close
                ],
                &[
                    &stock_data[1].1, // high
                    &stock_data[1].2, // low
                    &stock_data[1].3, // close
                ],
                &[
                    &stock_data[2].1, // high
                    &stock_data[2].2, // low
                    &stock_data[2].3, // close
                ],
                &[
                    &stock_data[3].1, // high
                    &stock_data[3].2, // low
                    &stock_data[3].3, // close
                ],
            ];

            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, options, None)
                .expect("SIMD by assets ADX indicator failed");

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
                let (regular_outputs, _) = rust_adx(&stock_inputs, options, None).unwrap_or_else(|_| panic!("Regular ADX failed for {} with period {}",
                    stock_symbol, options[0]));

                // Compare ADX result (only one main output)
                assert_eq!(
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len(),
                    "ADX output length mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[0]
                    .iter()
                    .zip(simd_results[stock_idx][0].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "ADX mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_adx_simd_vs_regular_database_optional_dx() {
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

        // Test each period with optional DX output
        for options in &OPTIONS_LIST {
            let optional_outputs = Some([true, false, false].as_slice()); // Enable DX only

            // Prepare inputs in the format expected by indicator_by_assets
            let inputs: [&[&[f64]; 3]; 4] = [
                &[
                    &stock_data[0].1, // high
                    &stock_data[0].2, // low
                    &stock_data[0].3, // close
                ],
                &[
                    &stock_data[1].1, // high
                    &stock_data[1].2, // low
                    &stock_data[1].3, // close
                ],
                &[
                    &stock_data[2].1, // high
                    &stock_data[2].2, // low
                    &stock_data[2].3, // close
                ],
                &[
                    &stock_data[3].1, // high
                    &stock_data[3].2, // low
                    &stock_data[3].3, // close
                ],
            ];

            // Get SIMD by assets result with optional DX output
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, options, optional_outputs)
                .expect("SIMD by assets ADX indicator with optional DX failed");

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
                let (regular_outputs, _) = rust_adx(&stock_inputs, options, optional_outputs)
                    .unwrap_or_else(|_| panic!("Regular ADX with optional DX failed for {} with period {}",
                        stock_symbol, options[0]));

                // Compare number of outputs (should be 2: adx, dx)
                assert_eq!(
                    regular_outputs.len(),
                    simd_results[stock_idx].len(),
                    "Number of outputs mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs.len(),
                    simd_results[stock_idx].len()
                );

                // Compare ADX output (index 0)
                for (i, (&regular_val, &simd_val)) in regular_outputs[0]
                    .iter()
                    .zip(simd_results[stock_idx][0].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "ADX mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }

                // Compare DX output (index 1)
                for (i, (&regular_val, &simd_val)) in regular_outputs[1]
                    .iter()
                    .zip(simd_results[stock_idx][1].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = DX_EPSILON) {
                        panic!(
                            "DX mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_adx_simd_vs_regular_database_optional_atr() {
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

        // Test each period with optional ATR output
        for options in &OPTIONS_LIST {
            let optional_outputs = Some([false, true, false].as_slice()); // Enable ATR only

            // Prepare inputs in the format expected by indicator_by_assets
            let inputs: [&[&[f64]; 3]; 4] = [
                &[
                    &stock_data[0].1, // high
                    &stock_data[0].2, // low
                    &stock_data[0].3, // close
                ],
                &[
                    &stock_data[1].1, // high
                    &stock_data[1].2, // low
                    &stock_data[1].3, // close
                ],
                &[
                    &stock_data[2].1, // high
                    &stock_data[2].2, // low
                    &stock_data[2].3, // close
                ],
                &[
                    &stock_data[3].1, // high
                    &stock_data[3].2, // low
                    &stock_data[3].3, // close
                ],
            ];

            // Get SIMD by assets result with optional ATR output
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, options, optional_outputs)
                .expect("SIMD by assets ADX indicator with optional ATR failed");

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
                let (regular_outputs, _) = rust_adx(&stock_inputs, options, optional_outputs)
                    .unwrap_or_else(|_| panic!("Regular ADX with optional ATR failed for {} with period {}",
                        stock_symbol, options[0]));

                // Compare number of outputs (should be 2: adx, atr)
                assert_eq!(
                    regular_outputs.len(),
                    simd_results[stock_idx].len(),
                    "Number of outputs mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs.len(),
                    simd_results[stock_idx].len()
                );

                // Compare ADX output (index 0)
                for (i, (&regular_val, &simd_val)) in regular_outputs[0]
                    .iter()
                    .zip(simd_results[stock_idx][0].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "ADX mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }

                // Compare ATR output (index 1)
                for (i, (&regular_val, &simd_val)) in regular_outputs[1]
                    .iter()
                    .zip(simd_results[stock_idx][1].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = ATR_EPSILON) {
                        panic!(
                            "ATR mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_adx_simd_vs_regular_database_optional_tr() {
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

        // Test each period with optional TR output
        for options in &OPTIONS_LIST {
            let optional_outputs = Some([false, false, true].as_slice()); // Enable TR only

            // Prepare inputs in the format expected by indicator_by_assets
            let inputs: [&[&[f64]; 3]; 4] = [
                &[
                    &stock_data[0].1, // high
                    &stock_data[0].2, // low
                    &stock_data[0].3, // close
                ],
                &[
                    &stock_data[1].1, // high
                    &stock_data[1].2, // low
                    &stock_data[1].3, // close
                ],
                &[
                    &stock_data[2].1, // high
                    &stock_data[2].2, // low
                    &stock_data[2].3, // close
                ],
                &[
                    &stock_data[3].1, // high
                    &stock_data[3].2, // low
                    &stock_data[3].3, // close
                ],
            ];

            // Get SIMD by assets result with optional TR output
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, options, optional_outputs)
                .expect("SIMD by assets ADX indicator with optional TR failed");

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
                let (regular_outputs, _) = rust_adx(&stock_inputs, options, optional_outputs)
                    .unwrap_or_else(|_| panic!("Regular ADX with optional TR failed for {} with period {}",
                        stock_symbol, options[0]));

                // Compare number of outputs (should be 2: adx, tr)
                assert_eq!(
                    regular_outputs.len(),
                    simd_results[stock_idx].len(),
                    "Number of outputs mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs.len(),
                    simd_results[stock_idx].len()
                );

                // Compare ADX output (index 0)
                for (i, (&regular_val, &simd_val)) in regular_outputs[0]
                    .iter()
                    .zip(simd_results[stock_idx][0].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "ADX mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }

                // Compare TR output (index 1)
                for (i, (&regular_val, &simd_val)) in regular_outputs[1]
                    .iter()
                    .zip(simd_results[stock_idx][1].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = TR_EPSILON) {
                        panic!(
                            "TR mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_adx_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD ADX 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_adx(&inputs, options, None).expect("Regular ADX indicator failed");

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
                            "SIMD ADX has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD ADX has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular ADX database tests passed!");
    }

    #[test]
    fn test_adx_simd_by_options_vs_regular_database_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Test with DX, ATR, and TR optional outputs
            let optional_outputs = Some(&[true, true, true][..]);

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD ADX 4-wide with optional outputs failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_adx(&inputs, options, optional_outputs)
                    .expect("Regular ADX indicator with optional outputs failed");

                let simd_adx_result = &all_simd_results[idx][0];
                let regular_adx_result = &regular_results[0];

                let simd_dx_result = &all_simd_results[idx][1];
                let regular_dx_result = &regular_results[1];

                let simd_atr_result = &all_simd_results[idx][2];
                let regular_atr_result = &regular_results[2];

                let simd_tr_result = &all_simd_results[idx][3];
                let regular_tr_result = &regular_results[3];

                // Compare ADX output lengths
                assert_eq!(
                    simd_adx_result.len(),
                    regular_adx_result.len(),
                    "ADX output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_adx_result.len(),
                    regular_adx_result.len()
                );

                // Compare DX output lengths
                assert_eq!(
                    simd_dx_result.len(),
                    regular_dx_result.len(),
                    "DX output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_dx_result.len(),
                    regular_dx_result.len()
                );

                // Compare ATR output lengths
                assert_eq!(
                    simd_atr_result.len(),
                    regular_atr_result.len(),
                    "ATR output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_atr_result.len(),
                    regular_atr_result.len()
                );

                // Compare TR output lengths
                assert_eq!(
                    simd_tr_result.len(),
                    regular_tr_result.len(),
                    "TR output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_tr_result.len(),
                    regular_tr_result.len()
                );

                // Compare ADX values
                for (i, (&simd_val, &regular_val)) in simd_adx_result
                    .iter()
                    .zip(regular_adx_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD ADX has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD ADX has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "ADX mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare DX values
                for (i, (&simd_val, &regular_val)) in simd_dx_result
                    .iter()
                    .zip(regular_dx_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD DX has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD DX has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = DX_EPSILON) {
                        panic!(
                            "DX mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare ATR values
                for (i, (&simd_val, &regular_val)) in simd_atr_result
                    .iter()
                    .zip(regular_atr_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD ATR has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD ATR has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = ATR_EPSILON) {
                        panic!(
                            "ATR mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare TR values
                for (i, (&simd_val, &regular_val)) in simd_tr_result
                    .iter()
                    .zip(regular_tr_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD TR has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TR has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = TR_EPSILON) {
                        panic!(
                            "TR mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular ADX database tests with optional outputs passed!"
        );
    }
}
