#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::natr::{indicator as rust_natr, min_data, TIndicatorState};
    use tulip_test::c_bindings::{
        ti_atr, ti_atr_start, ti_natr, ti_natr_start, ti_tr, ti_tr_start,
    };
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

    const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [10.0], [14.0], [20.0], [25.0], [30.0]];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
        }
        (high_vec, low_vec, close_vec)
    }

    #[test]
    fn test_natr_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            // Determine the offset required by the C NATR function
            let start_index = unsafe { ti_natr_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_natr_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut natr_output_vec_c = vec![0.0_f64; output_len_c];
            let natr_ptr: *mut f64 = natr_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![natr_ptr];
            let ret = unsafe {
                ti_natr(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_natr returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (outputs, _) =
                rust_natr(&inputs_rust, &options, None).expect("Rust NATR indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in natr_output_vec_c
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
                        "Rust NATR has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust NATR has infinity at index {}: Rust = {}",
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

                if !approx_eq!(f64, c_val, rust_val, epsilon = 1e-12) {
                    // Adjust epsilon if needed
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, natr_output_vec_c, outputs[0], options
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
    fn test_natr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

                let start_index = unsafe { ti_natr_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_natr_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_natr(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_natr returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (outputs, _) =
                    rust_natr(&inputs_rust, &options, None).expect("Rust NATR indicator failed");

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
                            "Rust NATR has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust NATR has infinity at index {}: Rust = {}",
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

    fn get_hlc_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }

    #[test]
    fn test_natr_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = rust_natr(&inputs_rust, &options, None)
                    .expect("Failed to run NATR indicator on full data");

                // Process in batches
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

                let (first_outputs, mut state) = rust_natr(&chunk_inputs, &options, None)
                    .expect("Failed to run NATR indicator on first chunk");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks using state
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
                        .expect("NATR batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Process remainder if any
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
                        .expect("NATR batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
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
                        "Mismatch in NATR output at index {}: full = {}, batch = {}, Stock: {}, Options: {:?}",
                        i, full_val, batch_val, stock_symbol, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_natr_atr_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let options = [14.0]; // period = 14
        let optional_outputs = Some([true, false].as_slice()); // Request atr output

        // Get Rust NATR output with atr optional output
        let result = rust_natr(&inputs, &options, optional_outputs).unwrap();
        let rust_atr = &result.0[1]; // atr is at index 1

        // Fail fast if Rust output is empty
        if rust_atr.is_empty() {
            panic!("Rust NATR atr optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip ATR output for comparison
        let atr_inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
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
        assert_eq!(ret, 0, "ti_atr returned error code {}", ret);

        // Compare ATR outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing NATR atr optional output vs C Tulip ATR:");
        println!(
            "Rust atr length: {}, C ATR length: {}",
            rust_atr.len(),
            c_atr.len()
        );

        for (i, (rust_val, c_val)) in rust_atr.iter().rev().zip(c_atr.iter().rev()).enumerate() {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust atr output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            // Skip comparison if C output is NaN/infinite (assume C bug)
            if !c_val.is_finite() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > EPSILON {
                panic!(
                    "NATR atr mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ NATR atr optional output matches C Tulip ATR output");
    }

    #[test]
    fn test_natr_tr_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let options = [14.0]; // period = 14
        let optional_outputs = Some([false, true].as_slice()); // Request tr output

        // Get Rust NATR output with tr optional output
        let result = rust_natr(&inputs, &options, optional_outputs).unwrap();
        let rust_tr = &result.0[2]; // tr is at index 2

        // Fail fast if Rust output is empty
        if rust_tr.is_empty() {
            panic!("Rust NATR tr optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip TR output for comparison
        let tr_inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
        let tr_options: Vec<f64> = vec![]; // TR takes no options
        let tr_start_index = unsafe { ti_tr_start(tr_options.as_ptr()) };
        let tr_output_len = high.len() - (tr_start_index as usize);
        let mut c_tr = vec![0.0; tr_output_len];
        let mut tr_outputs_c = vec![c_tr.as_mut_ptr()];

        let ret = unsafe {
            ti_tr(
                high.len() as i32,
                tr_inputs_c.as_ptr(),
                tr_options.as_ptr(),
                tr_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_tr returned error code {}", ret);

        // Compare TR outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing NATR tr optional output vs C Tulip TR:");
        println!(
            "Rust tr length: {}, C TR length: {}",
            rust_tr.len(),
            c_tr.len()
        );

        for (i, (rust_val, c_val)) in rust_tr.iter().rev().zip(c_tr.iter().rev()).enumerate() {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust tr output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            // Skip comparison if C output is NaN/infinite (assume C bug)
            if !c_val.is_finite() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > EPSILON {
                panic!(
                    "NATR tr mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ NATR tr optional output matches C Tulip TR output");
    }

    #[test]
    fn test_natr_database_optional_atr() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let (high, low, close) = get_hlc_arrays(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get NATR with ATR optional output
                let optional_outputs = Some(&[true, false][..]);
                let (natr_result, _) = tulip_rs::indicators::natr::indicator(
                    &[&high, &low, &close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_atr = &natr_result[1];

                // Calculate expected ATR using C Tulip ti_atr
                let atr_options = vec![options[0]];
                let start_index = unsafe { ti_atr_start(atr_options.as_ptr()) };
                assert!(start_index >= 0, "ti_atr_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut c_atr_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_atr_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_atr(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        atr_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_atr failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_atr.len().min(c_atr_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_atr.len() - 1 - i;
                    let c_idx = c_atr_output.len() - 1 - i;

                    let rust_val = rust_atr[rust_idx];
                    let c_val = c_atr_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust ATR output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "NATR ATR optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_natr_database_optional_tr() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let (high, low, close) = get_hlc_arrays(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get NATR with TR optional output
                let optional_outputs = Some(&[false, true][..]);
                let (natr_result, _) = tulip_rs::indicators::natr::indicator(
                    &[&high, &low, &close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_tr = &natr_result[2];

                // Calculate expected TR using C Tulip ti_tr (TR takes no options)
                let tr_options: Vec<f64> = vec![];
                let start_index = unsafe { ti_tr_start(tr_options.as_ptr()) };
                assert!(start_index >= 0, "ti_tr_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut c_tr_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_tr_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_tr(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        tr_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_tr failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_tr.len().min(c_tr_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_tr.len() - 1 - i;
                    let c_idx = c_tr_output.len() - 1 - i;

                    let rust_val = rust_tr[rust_idx];
                    let c_val = c_tr_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust TR output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "NATR TR optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_natr_simd_vs_regular_database() {
        use tulip_rs::indicators::natr::indicator_by_assets;

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
                .expect("SIMD by assets NATR indicator failed");

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
                let (regular_outputs, _) =
                    rust_natr(&stock_inputs, options, None).expect(&format!(
                        "Regular NATR failed for {} with period {}",
                        stock_symbol, options[0]
                    ));

                // Compare SIMD result with regular result
                assert_eq!(
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len(),
                    "Output length mismatch for stock {} with period {}: regular = {}, simd = {}",
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
                    const EPSILON: f64 = 1e-8;
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "NATR mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by assets vs Regular NATR database tests passed!");
    }

    #[test]
    fn test_natr_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::natr::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD NATR 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD NATR 2-wide failed");

            // Combine results
            let mut all_simd_results = simd_results_4;
            all_simd_results.extend(simd_results_2);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_natr(&inputs, options, None).expect("Regular NATR indicator failed");

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
                            "SIMD NATR has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD NATR has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    const EPSILON: f64 = 1e-8;
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular NATR database tests passed!");
    }
}
