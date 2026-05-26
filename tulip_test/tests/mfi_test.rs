#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::mfi::indicator_by_options;
    use tulip_rs::indicators::mfi::{indicator as rust_mfi, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_mfi, ti_mfi_start, ti_typprice, ti_typprice_start};
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
    const VOLUME: [f64; 15] = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];

    const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [10.0], [14.0], [20.0], [25.0], [30.0]];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        let mut volume_vec = VOLUME.to_vec();
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
            volume_vec.extend_from_slice(&VOLUME);
        }
        (high_vec, low_vec, close_vec, volume_vec)
    }

    #[test]
    fn test_mfi_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close, volume) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> =
                vec![high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr()];

            // Determine the offset required by the C MFI function
            let start_index = unsafe { ti_mfi_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_mfi_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut mfi_output_vec_c = vec![0.0_f64; output_len_c];
            let mfi_ptr: *mut f64 = mfi_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![mfi_ptr];
            let ret = unsafe {
                ti_mfi(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_mfi returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];
            let (outputs, _) =
                rust_mfi(&inputs_rust, &options, None).expect("Rust MFI indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in mfi_output_vec_c
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
                        "Rust MFI has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust MFI has infinity at index {}: Rust = {}",
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
                        index, mfi_output_vec_c, outputs[0], options
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
    fn test_mfi_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(&stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> =
                    vec![high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr()];

                let start_index = unsafe { ti_mfi_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_mfi_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_mfi(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_mfi returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [
                    high.as_slice(),
                    low.as_slice(),
                    close.as_slice(),
                    volume.as_slice(),
                ];
                let (outputs, _) =
                    rust_mfi(&inputs_rust, &options, None).expect("Rust MFI indicator failed");

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
                            "Rust MFI has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust MFI has infinity at index {}: Rust = {}",
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

    fn get_hlcv_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
        (high, low, close, volume)
    }

    #[test]
    fn test_mfi_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(&stock_data);
            let inputs_rust = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = rust_mfi(&inputs_rust, &options, None)
                    .expect("Failed to run MFI indicator on full data");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let high_vec = high[..min_data_val].to_vec();
                let low_vec = low[..min_data_val].to_vec();
                let close_vec = close[..min_data_val].to_vec();
                let volume_vec = volume[..min_data_val].to_vec();
                let chunk_inputs = [
                    high_vec.as_slice(),
                    low_vec.as_slice(),
                    close_vec.as_slice(),
                    volume_vec.as_slice(),
                ];

                let (first_outputs, mut state) = rust_mfi(&chunk_inputs, &options, None)
                    .expect("Failed to run MFI indicator on first chunk");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks using state
                let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume[min_data_val..].chunks_exact(CHUNK_SIZE);

                for (((high_chunk, low_chunk), close_chunk), volume_chunk) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                    .zip(volume_chunks.by_ref())
                {
                    let high_vec = high_chunk.to_vec();
                    let low_vec = low_chunk.to_vec();
                    let close_vec = close_chunk.to_vec();
                    let volume_vec = volume_chunk.to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                        volume_vec.as_slice(),
                    ];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("MFI batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Process remainder if any
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                let volume_rem = volume_chunks.remainder();
                if !high_rem.is_empty() {
                    let high_vec = high_rem.to_vec();
                    let low_vec = low_rem.to_vec();
                    let close_vec = close_rem.to_vec();
                    let volume_vec = volume_rem.to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                        volume_vec.as_slice(),
                    ];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("MFI batch indicator failed");
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
                        "Mismatch in MFI output at index {}: full = {}, batch = {}, Stock: {}, Options: {:?}",
                        i, full_val, batch_val, stock_symbol, options
                    );
                }
            }
        }
    }

    #[test]
    fn test_mfi_typprice_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let (high, low, close, volume) = expand_inputs();
        let inputs = [
            high.as_slice(),
            low.as_slice(),
            close.as_slice(),
            volume.as_slice(),
        ];
        let options = [14.0]; // period = 14
        let optional_outputs = Some([true].as_slice()); // Request typprice output

        // Get Rust MFI output with typprice optional output
        let result = rust_mfi(&inputs, &options, optional_outputs).unwrap();
        let rust_typprice = &result.0[1]; // typprice is at index 1

        // Fail fast if Rust output is empty
        if rust_typprice.is_empty() {
            panic!("Rust MFI typprice optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip typprice output for comparison
        let typprice_inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
        let typprice_options: Vec<f64> = vec![]; // typprice takes no options
        let typprice_start_index = unsafe { ti_typprice_start(typprice_options.as_ptr()) };
        let typprice_output_len = high.len() - (typprice_start_index as usize);
        let mut c_typprice = vec![0.0; typprice_output_len];
        let mut typprice_outputs_c = vec![c_typprice.as_mut_ptr()];

        let ret = unsafe {
            ti_typprice(
                high.len() as i32,
                typprice_inputs_c.as_ptr(),
                typprice_options.as_ptr(),
                typprice_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_typprice returned error code {}", ret);

        // Compare typprice outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing MFI typprice optional output vs C Tulip typprice:");
        println!(
            "Rust typprice length: {}, C typprice length: {}",
            rust_typprice.len(),
            c_typprice.len()
        );

        for (i, (rust_val, c_val)) in rust_typprice
            .iter()
            .rev()
            .zip(c_typprice.iter().rev())
            .enumerate()
        {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust typprice output contains NaN/infinity at position {}: {}",
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
                    "MFI typprice mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ MFI typprice optional output matches C Tulip typprice output");
    }

    #[test]
    fn test_mfi_database_optional_typprice() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let (high, low, close, volume) = get_hlcv_arrays(&stock_data);

            for &options in &OPTIONS_LIST {
                // Get MFI with typprice optional output
                let optional_outputs = Some(&[true][..]);
                let (mfi_result, _) = tulip_rs::indicators::mfi::indicator(
                    &[&high, &low, &close, &volume],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_typprice = &mfi_result[1];

                // Calculate expected typprice using C Tulip ti_typprice
                let typprice_options: Vec<f64> = vec![];
                let start_index = unsafe { ti_typprice_start(typprice_options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_typprice_start returned a negative index"
                );
                let output_len_c = high.len() - (start_index as usize);

                let mut c_typprice_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_typprice_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_typprice(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        typprice_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_typprice failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_typprice.len().min(c_typprice_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_typprice.len() - 1 - i;
                    let c_idx = c_typprice_output.len() - 1 - i;

                    let rust_val = rust_typprice[rust_idx];
                    let c_val = c_typprice_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust typprice output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "MFI typprice optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_mfi_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::mfi::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, close, volume) = get_hlcv_arrays(data);
                (symbol.clone(), high, low, close, volume)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 4]; 4] = [
            &[
                &stock_data[0].1, // high
                &stock_data[0].2, // low
                &stock_data[0].3, // close
                &stock_data[0].4, // volume
            ],
            &[
                &stock_data[1].1, // high
                &stock_data[1].2, // low
                &stock_data[1].3, // close
                &stock_data[1].4, // volume
            ],
            &[
                &stock_data[2].1, // high
                &stock_data[2].2, // low
                &stock_data[2].3, // close
                &stock_data[2].4, // volume
            ],
            &[
                &stock_data[3].1, // high
                &stock_data[3].2, // low
                &stock_data[3].3, // close
                &stock_data[3].4, // volume
            ],
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets MFI indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low, stock_close, stock_volume)) in
                stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [
                    stock_high.as_slice(),
                    stock_low.as_slice(),
                    stock_close.as_slice(),
                    stock_volume.as_slice(),
                ];
                let (regular_results, _) =
                    rust_mfi(&stock_inputs, &options, None).expect("Regular MFI indicator failed");

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
                            "SIMD by assets MFI has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets MFI has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for MFI
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-10) {
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

        println!("✓ All SIMD by assets vs Regular MFI database tests passed!");
    }

    #[test]
    fn test_mfi_simd_by_assets_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::mfi::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, close, volume) = get_hlcv_arrays(data);
                (symbol.clone(), high, low, close, volume)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 4]; 4] = [
            &[
                &stock_data[0].1, // high
                &stock_data[0].2, // low
                &stock_data[0].3, // close
                &stock_data[0].4, // volume
            ],
            &[
                &stock_data[1].1, // high
                &stock_data[1].2, // low
                &stock_data[1].3, // close
                &stock_data[1].4, // volume
            ],
            &[
                &stock_data[2].1, // high
                &stock_data[2].2, // low
                &stock_data[2].3, // close
                &stock_data[2].4, // volume
            ],
            &[
                &stock_data[3].1, // high
                &stock_data[3].2, // low
                &stock_data[3].3, // close
                &stock_data[3].4, // volume
            ],
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result with optional outputs
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, Some(&[true]))
                .expect("SIMD by assets MFI indicator with optional outputs failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low, stock_close, stock_volume)) in
                stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [
                    stock_high.as_slice(),
                    stock_low.as_slice(),
                    stock_close.as_slice(),
                    stock_volume.as_slice(),
                ];
                let (regular_results, _) = rust_mfi(&stock_inputs, &options, Some(&[true]))
                    .expect("Regular MFI indicator with optional outputs failed");

                let simd_result = &simd_results[stock_idx][0];
                let regular_result = &regular_results[0];

                // Compare main outputs
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "Main output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result.len(),
                    regular_result.len()
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-10) {
                        panic!(
                            "Main output mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare optional outputs (typprice) - index 1 in results vector
                let simd_typprice = &simd_results[stock_idx][1];
                let regular_typprice = &regular_results[1];

                assert_eq!(
                    simd_typprice.len(),
                    regular_typprice.len(),
                    "Typprice output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_typprice.len(),
                    regular_typprice.len()
                );

                for (i, (&simd_val, &regular_val)) in simd_typprice
                    .iter()
                    .zip(regular_typprice.iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-10) {
                        panic!(
                            "Typprice output mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                println!(
                    "✓ SIMD by assets vs Regular test with optional outputs passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!(
            "✓ All SIMD by assets vs Regular MFI database tests with optional outputs passed!"
        );
    }

    #[test]
    fn test_mfi_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(&stock_data);
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            if high.is_empty() {
                continue;
            }

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];

            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD MFI by options 4-wide failed");

            // Process last 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];

            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD MFI by options 2-wide failed");

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
                let (regular_results, _) =
                    rust_mfi(&inputs, options, None).expect("Regular MFI indicator failed");

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
                            "SIMD by options MFI has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by options MFI has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, options, simd_val
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

                println!(
                    "✓ SIMD by options vs Regular test passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All SIMD by options vs Regular MFI database tests passed!");
    }

    #[test]
    fn test_mfi_simd_by_options_vs_regular_database_optional_outputs() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(&stock_data);
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            if high.is_empty() {
                continue;
            }

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];

            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, Some(&[true]))
                .expect("SIMD MFI by options 4-wide with optional outputs failed");

            // Process last 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];

            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, Some(&[true]))
                .expect("SIMD MFI by options 2-wide with optional outputs failed");

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) = rust_mfi(&inputs, options, Some(&[true]))
                    .expect("Regular MFI indicator with optional outputs failed");
                let regular_mfi = &regular_results[0];
                let regular_typprice = &regular_results[1];

                let (simd_mfi, simd_typprice) = if idx < 4 {
                    (&simd_results_4[idx][0], &simd_results_4[idx][1])
                } else {
                    (&simd_results_2[idx - 4][0], &simd_results_2[idx - 4][1])
                };

                // Compare MFI results
                assert_eq!(
                    regular_mfi.len(),
                    simd_mfi.len(),
                    "MFI length mismatch for stock {} options {:?}",
                    stock_symbol,
                    options
                );

                for (i, (&regular_val, &simd_val)) in
                    regular_mfi.iter().zip(simd_mfi.iter()).enumerate()
                {
                    if regular_val.is_nan() && simd_val.is_nan() {
                        continue;
                    }
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "MFI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare typprice results
                assert_eq!(
                    regular_typprice.len(),
                    simd_typprice.len(),
                    "Typprice length mismatch for stock {} options {:?}",
                    stock_symbol,
                    options
                );

                for (i, (&regular_val, &simd_val)) in regular_typprice
                    .iter()
                    .zip(simd_typprice.iter())
                    .enumerate()
                {
                    if regular_val.is_nan() && simd_val.is_nan() {
                        continue;
                    }
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Typprice mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular MFI database tests with optional outputs passed!"
        );
    }

    #[test]
    fn test_mfi_simd_state_handover_by_options() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // number of bars to process with SIMD first
        let first_bars = 2000usize;

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(&stock_data);
            let total_len = high.len();
            if total_len == 0 {
                continue;
            }

            let split = first_bars.min(total_len);

            // prepare slices for first part and remaining
            let first_inputs = [
                &high[..split],
                &low[..split],
                &close[..split],
                &volume[..split],
            ];
            let remaining_inputs = if split < total_len {
                Some([
                    &high[split..],
                    &low[split..],
                    &close[split..],
                    &volume[split..],
                ])
            } else {
                None
            };

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, states_4) =
                indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD MFI 4-wide failed on first chunk");

            // Process last 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, states_2) =
                indicator_by_options::<2>(&first_inputs, &options_2, None)
                    .expect("SIMD MFI 2-wide failed on first chunk");

            // Combine SIMD results for first part and prepare to extend with batch_indicator outputs
            let mut all_simd_results: Vec<Vec<f64>> = Vec::new();
            for i in 0..4 {
                all_simd_results.push(simd_results_4[i][0].clone());
            }
            for i in 0..2 {
                all_simd_results.push(simd_results_2[i][0].clone());
            }

            // If there is remaining data, use the returned states to process it
            if let Some(rem_inputs) = remaining_inputs {
                // states_4 are Vec<IndicatorState>
                for (i, mut st) in states_4.into_iter().enumerate() {
                    let chunk_out = st.batch_indicator(&rem_inputs, None).expect("batch failed");
                    all_simd_results[i].extend_from_slice(&chunk_out[0]);
                }

                for (i, mut st) in states_2.into_iter().enumerate() {
                    let chunk_out = st.batch_indicator(&rem_inputs, None).expect("batch failed");
                    all_simd_results[i + 4].extend_from_slice(&chunk_out[0]);
                }
            }

            // Compare each SIMD result with regular indicator over the full data
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) = rust_mfi(
                    &[
                        high.as_slice(),
                        low.as_slice(),
                        close.as_slice(),
                        volume.as_slice(),
                    ],
                    options,
                    None,
                )
                .expect("Regular MFI indicator failed");
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

        println!("✓ All MFI SIMD state handover by options tests passed!");
    }
}
