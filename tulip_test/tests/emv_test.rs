#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::emv::{indicator as rust_emv, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_emv, ti_emv_start, ti_medprice, ti_medprice_start};
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
    const VOLUME: [f64; 15] = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];
    const OPTIONS: [f64; 0] = [];
    fn get_hlv_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
        (high, low, volume)
    }

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut volume_vec = VOLUME.to_vec();
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            volume_vec.extend_from_slice(&VOLUME);
        }
        (high_vec, low_vec, volume_vec)
    }

    #[test]
    fn test_emv_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, volume) = expand_inputs();

        // Prepare inputs for the C implementation
        let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), volume.as_ptr()];

        // Determine the offset required by the C EMV function
        let start_index = unsafe { ti_emv_start(std::ptr::null()) };
        assert!(start_index >= 0, "ti_emv_start returned a negative index");
        let output_len_c = high.len() - (start_index as usize);

        // Run the C implementation
        let mut emv_output_vec_c = vec![0.0_f64; output_len_c];
        let emv_ptr: *mut f64 = emv_output_vec_c.as_mut_ptr();
        let mut outputs_c: Vec<*mut f64> = vec![emv_ptr];
        let ret = unsafe {
            ti_emv(
                high.len() as i32,
                inputs_c.as_ptr(),
                std::ptr::null(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_emv returned error code {}", ret);

        // Run the Rust implementation
        let inputs_rust = [high.as_slice(), low.as_slice(), volume.as_slice()];
        let (outputs, _) = rust_emv(&inputs_rust, &[], None).expect("Rust EMV indicator failed");

        let output_len_rust = outputs[0].len();

        // Compare the outputs in reverse for the length of the Rust outputs
        for (i, (&c_val, &rust_val)) in emv_output_vec_c
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
                    "Rust EMV has NaN at index {}: Rust = {}, Options = {:?}",
                    index, rust_val, OPTIONS
                );
            }

            // Fail test if Rust has infinity
            if rust_val.is_infinite() {
                panic!(
                    "Rust EMV has infinity at index {}: Rust = {}",
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
                    "Test failed at index {}: \nC = {:?}, \nRust = {:?}",
                    index, emv_output_vec_c, outputs[0]
                );
                panic!(
                    "Mismatch at index {}: C = {}, Rust = {}",
                    index, c_val, rust_val
                );
            }
        }
    }

    #[test]
    fn test_emv_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(stock_data);

            // C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), volume.as_ptr()];

            let start_index = unsafe { ti_emv_start(std::ptr::null()) };
            assert!(start_index >= 0, "ti_emv_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            let mut emv_output_vec_c = vec![0.0_f64; output_len_c];
            let emv_ptr: *mut f64 = emv_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![emv_ptr];
            let ret = unsafe {
                ti_emv(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_emv returned error code {}", ret);

            // Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice(), volume.as_slice()];
            let (outputs, _) =
                rust_emv(&inputs_rust, &[], None).expect("Rust EMV indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare results
            for (i, (&c_val, &rust_val)) in emv_output_vec_c
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
                        "Rust EMV has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                        index, rust_val, OPTIONS, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust EMV has infinity at index {}: Rust = {}",
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
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Stock: {}",
                        index, emv_output_vec_c, outputs[0], stock_symbol
                    );
                    panic!(
                        "Mismatch at index {}: C = {}, Rust = {}, Stock: {}",
                        index, c_val, rust_val, stock_symbol
                    );
                }
            }
        }
    }

    #[test]
    fn test_emv_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(stock_data);

            let inputs_rust = [high.as_slice(), low.as_slice(), volume.as_slice()];

            // Get full output
            let (full_outputs, _) =
                rust_emv(&inputs_rust, &[], None).expect("Rust EMV indicator failed");

            // Process in batches
            let mut batch_full_output = Vec::new();

            let min_data_val = min_data(&[]).max(CHUNK_SIZE);

            if high.len() <= min_data_val {
                // If data is too small, just run full calculation
                let (outputs, _) =
                    rust_emv(&inputs_rust, &[], None).expect("Failed to run EMV indicator");
                batch_full_output.extend_from_slice(&outputs[0]);
            } else {
                // First chunk - convert to Vec<&Vec<f64>>
                let high_vec = high[..min_data_val].to_vec();
                let low_vec = low[..min_data_val].to_vec();
                let volume_vec = volume[..min_data_val].to_vec();
                let chunk_inputs = [
                    high_vec.as_slice(),
                    low_vec.as_slice(),
                    volume_vec.as_slice(),
                ];

                let (first_outputs, mut state) = rust_emv(&chunk_inputs, &[], None)
                    .expect("Failed to run EMV indicator on first chunk");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks using state
                let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume[min_data_val..].chunks_exact(CHUNK_SIZE);

                for ((high_chunk, low_chunk), volume_chunk) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(volume_chunks.by_ref())
                {
                    let high_vec = high_chunk.to_vec();
                    let low_vec = low_chunk.to_vec();
                    let volume_vec = volume_chunk.to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        volume_vec.as_slice(),
                    ];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("EMV batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Process remainder if any
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let volume_rem = volume_chunks.remainder();

                if !high_rem.is_empty() && !low_rem.is_empty() && !volume_rem.is_empty() {
                    let high_vec = high_rem.to_vec();
                    let low_vec = low_rem.to_vec();
                    let volume_vec = volume_rem.to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        volume_vec.as_slice(),
                    ];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("EMV batch indicator failed");
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
                    "EMV mismatch at index {}: full = {}, batch = {}, stock = {}",
                    i, full_val, batch_val, stock_symbol
                );
            }
        }
    }

    #[test]
    fn test_emv_simd_vs_regular_database() {
        use tulip_rs::indicators::emv::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, volume) = get_hlv_arrays(data);
                (symbol.clone(), high, low, volume)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 3]; 4] = [
            &[
                &stock_data[0].1, // high
                &stock_data[0].2, // low
                &stock_data[0].3, // volume
            ],
            &[
                &stock_data[1].1, // high
                &stock_data[1].2, // low
                &stock_data[1].3, // volume
            ],
            &[
                &stock_data[2].1, // high
                &stock_data[2].2, // low
                &stock_data[2].3, // volume
            ],
            &[
                &stock_data[3].1, // high
                &stock_data[3].2, // low
                &stock_data[3].3, // volume
            ],
        ];

        // Test without optional outputs
        {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &[], None)
                .expect("SIMD by assets EMV indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low, stock_volume)) in
                stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [
                    stock_high.as_slice(),
                    stock_low.as_slice(),
                    stock_volume.as_slice(),
                ];
                let (regular_results, _) =
                    rust_emv(&stock_inputs, &[], None).expect("Regular EMV indicator failed");

                let simd_result = &simd_results[stock_idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "Output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    OPTIONS,
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
                            "SIMD by assets EMV has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, OPTIONS, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets EMV has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, OPTIONS, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for EMV
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        panic!(
                            "Mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, OPTIONS, simd_val, regular_val
                        );
                    }
                }

                println!(
                    "✓ SIMD by assets vs Regular test passed for stock {} with options {:?}",
                    stock_symbol, OPTIONS
                );
            }
        }

        println!("✓ All SIMD by assets vs Regular EMV database tests passed!");
    }

    #[test]
    fn test_emv_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::emv::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (high, low, volume) = get_hlv_arrays(data);
                (symbol.clone(), high, low, volume)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 3]; 4] = [
            &[
                &stock_data[0].1, // high
                &stock_data[0].2, // low
                &stock_data[0].3, // volume
            ],
            &[
                &stock_data[1].1, // high
                &stock_data[1].2, // low
                &stock_data[1].3, // volume
            ],
            &[
                &stock_data[2].1, // high
                &stock_data[2].2, // low
                &stock_data[2].3, // volume
            ],
            &[
                &stock_data[3].1, // high
                &stock_data[3].2, // low
                &stock_data[3].3, // volume
            ],
        ];

        // Test with optional outputs
        {
            // Get SIMD by assets result with optional outputs
            let (simd_results_opt, _) = indicator_by_assets::<4>(&inputs, &[], Some(&[true]))
                .expect("SIMD by assets EMV indicator with optional outputs failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low, stock_volume)) in
                stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock with optional outputs
                let stock_inputs = [
                    stock_high.as_slice(),
                    stock_low.as_slice(),
                    stock_volume.as_slice(),
                ];
                let (regular_results_opt, _) = rust_emv(&stock_inputs, &[], Some(&[true]))
                    .expect("Regular EMV indicator with optional outputs failed");

                // Compare all outputs: EMV and medprice
                let output_names = ["EMV", "medprice"];
                for (output_idx, output_name) in output_names.iter().enumerate() {
                    let simd_result = &simd_results_opt[stock_idx][output_idx];
                    let regular_result = &regular_results_opt[output_idx];

                    // Skip empty optional outputs
                    if simd_result.is_empty() && regular_result.is_empty() {
                        continue;
                    }

                    // Compare output lengths
                    assert_eq!(
                        simd_result.len(),
                        regular_result.len(),
                        "Output length mismatch for {} output of stock {} with options {:?}: SIMD={}, Regular={}",
                        output_name,
                        stock_symbol,
                        OPTIONS,
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
                                "SIMD by assets {} has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                output_name, i, stock_symbol, OPTIONS, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets {} has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                output_name, i, stock_symbol, OPTIONS, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                            panic!(
                                "Mismatch in {} output at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                output_name, i, stock_symbol, OPTIONS, simd_val, regular_val
                            );
                        }
                    }
                }

                println!(
                    "✓ SIMD by assets vs Regular optional outputs test passed for stock {} with options {:?}",
                    stock_symbol, OPTIONS
                );
            }
        }

        println!("✓ All SIMD by assets vs Regular EMV optional outputs database tests passed!");
    }

    #[test]
    fn test_emv_medprice_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let (high, low, volume) = (HIGH.to_vec(), LOW.to_vec(), VOLUME.to_vec());
        let inputs = [high.as_slice(), low.as_slice(), volume.as_slice()];
        let options = OPTIONS;
        let optional_outputs = Some([true].as_slice()); // Request medprice output

        // Get Rust EMV output with medprice optional output
        let result = rust_emv(&inputs, &options, optional_outputs).unwrap();
        let rust_medprice = &result.0[1]; // medprice is at index 1

        // Fail fast if Rust output is empty
        if rust_medprice.is_empty() {
            panic!("Rust EMV medprice optional output is empty - this indicates an indicator bug");
        }
        // Get C Tulip medprice output for comparison
        let c_inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];
        let c_start_index = unsafe { ti_medprice_start(std::ptr::null()) } as usize;
        let c_output_len = high.len() - c_start_index;
        let mut c_medprice = vec![0.0; c_output_len];
        let mut c_outputs = vec![c_medprice.as_mut_ptr()];

        let ret = unsafe {
            ti_medprice(
                high.len() as i32,
                c_inputs.as_ptr(),
                std::ptr::null(),
                c_outputs.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_medprice returned error code {}", ret);

        // Compare outputs from the end backwards (reverse order comparison)
        // This avoids alignment issues due to different warm-up periods
        println!("Comparing EMV medprice optional output vs C Tulip medprice:");
        println!(
            "Rust medprice length: {}, C medprice length: {}",
            rust_medprice.len(),
            c_medprice.len()
        );

        for (i, (rust_val, c_val)) in rust_medprice
            .iter()
            .rev()
            .zip(c_medprice.iter().rev())
            .enumerate()
        {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust medprice output contains NaN/infinity at position {}: {}",
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
                    "EMV medprice mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ EMV medprice optional output matches C Tulip medprice output");
    }

    #[test]
    fn test_emv_database_optional_medprice() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let (high, low, volume) = get_hlv_arrays(stock_data);

            // Get EMV with medprice optional output
            let optional_outputs = Some(&[true][..]);
            let (emv_result, _) = tulip_rs::indicators::emv::indicator(
                &[&high, &low, &volume],
                &OPTIONS,
                optional_outputs,
            )
            .unwrap();

            let rust_medprice = &emv_result[1];

            // Calculate expected medprice using C Tulip ti_medprice
            let start_index = unsafe { ti_medprice_start(std::ptr::null()) } as usize;
            let output_len_c = high.len() - start_index;

            let mut c_medprice_output = vec![0.0; output_len_c];
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];
            let mut outputs_c: Vec<*mut f64> = vec![c_medprice_output.as_mut_ptr()];

            unsafe {
                let ret = ti_medprice(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                );
                assert_eq!(ret, 0, "ti_medprice failed");
            }

            // Compare from most recent values backwards
            let compare_len = rust_medprice.len().min(c_medprice_output.len());
            for i in 0..compare_len {
                let rust_idx = rust_medprice.len() - 1 - i;
                let c_idx = c_medprice_output.len() - 1 - i;

                let rust_val = rust_medprice[rust_idx];
                let c_val = c_medprice_output[c_idx];

                if rust_val.is_nan() || rust_val.is_infinite() {
                    panic!(
                        "Rust medprice output is NaN or infinite at index {}: {}",
                        rust_idx, rust_val
                    );
                }

                if c_val.is_nan() || c_val.is_infinite() {
                    continue; // Skip comparison if C output is invalid
                }

                assert!(
                    approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                    "EMV medprice optional output mismatch at index {}: rust={}, c={}, diff={}",
                    rust_idx,
                    rust_val,
                    c_val,
                    (rust_val - c_val).abs()
                );
            }
        }
    }
}
