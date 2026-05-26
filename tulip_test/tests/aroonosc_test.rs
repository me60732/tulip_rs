#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::aroonosc::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_aroon, ti_aroon_start, ti_aroonosc, ti_aroonosc_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const EPSILON: f64 = 1e-12;
    const AROON_EPSILON: f64 = 1e-12; // Use epsilon from aroon_test.rs
    const CHUNK_SIZE: usize = 100;

    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];

    const OPTIONS_LIST: [[f64; 1]; 8] = [
        [5.0],
        [8.0],
        [10.0],
        [14.0],
        [25.0],
        [35.0],
        [50.0],
        [100.0],
    ];

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
        for _ in 0..100 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
        }
        (high_vec, low_vec)
    }

    #[test]
    fn test_aroonosc_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            // Determine the offset required by the C AROONOSC function
            let start_index = unsafe { ti_aroonosc_start(options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_aroonosc_start returned a negative index"
            );
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut aroonosc_output_vec_c = vec![0.0_f64; output_len_c];
            let aroonosc_ptr: *mut f64 = aroonosc_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![aroonosc_ptr];
            let ret = unsafe {
                ti_aroonosc(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_aroonosc returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Rust AROONOSC indicator failed");

            let output_len_rust = outputs[0].len();
            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in aroonosc_output_vec_c
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
                        "Rust AROONOSC has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust AROONOSC has infinity at index {}: Rust = {}",
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

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = EPSILON),
                    "Mismatch at index {}: C = {}, Rust = {} for options {:?}, \nRust Output: {:?}, \nC Output: {:?}",
                    index,
                    c_val,
                    rust_val,
                    options,
                    outputs[0],
                    aroonosc_output_vec_c,
                );
            }
        }
    }

    #[test]
    fn test_aroonosc_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

                let start_index = unsafe { ti_aroonosc_start(options.as_ptr()) };
                assert!(
                    start_index >= 0,
                    "ti_aroonosc_start returned a negative index"
                );
                let output_len_c = high.len() - (start_index as usize);

                let mut aroonosc_output_vec_c = vec![0.0_f64; output_len_c];
                let aroonosc_ptr: *mut f64 = aroonosc_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![aroonosc_ptr];
                let ret = unsafe {
                    ti_aroonosc(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_aroonosc returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice()];
                let (outputs, _) = indicator(&inputs_rust, &options, None)
                    .expect("Rust AROONOSC indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare results
                for (i, (&c_val, &rust_val)) in aroonosc_output_vec_c
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
                            "Rust AROONOSC has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust AROONOSC has infinity at index {}: Rust = {}",
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
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, aroonosc_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_aroonosc_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = indicator(&inputs_rust, &options, None)
                    .expect("Failed to run AROONOSC indicator on full data");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) = indicator(&inputs_rust, &options, None)
                        .expect("Failed to run AROONOSC indicator");
                    batch_full_output.extend_from_slice(&outputs[0]);
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let high_vec = high[..min_data_val].to_vec();
                    let low_vec = low[..min_data_val].to_vec();
                    let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];

                    let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("Failed to run AROONOSC indicator on first chunk");
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
                            .expect("AROONOSC batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }

                    // Process remainder if any
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();

                    if !high_rem.is_empty() {
                        let high_vec = high_rem.to_vec();
                        let low_vec = low_rem.to_vec();
                        let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("AROONOSC batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }
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
                        "Mismatch at index {} for stock {} with options {:?}: full={}, batch={}",
                        i, stock_symbol, options, full_val, batch_val
                    );
                }
            }
        }
    }

    #[test]
    fn test_aroonosc_optional_outputs_vs_c_tulip() {
        // Test both aroon_up and aroon_down optional outputs against C Tulip aroon
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            println!(
                "Testing AROONOSC optional outputs with options: {:?}",
                options
            );

            // Get Rust AROONOSC with both optional outputs enabled
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[true, true]))
                .expect("Rust AROONOSC indicator with optional outputs failed");

            let rust_aroon_up = &rust_outputs[2]; // aroon_up is at index 1
            let rust_aroon_down = &rust_outputs[1]; // aroon_down is at index 2

            // Run C Tulip aroon to get both aroon_up and aroon_down
            let aroon_inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];
            let aroon_start_index = unsafe { ti_aroon_start(options.as_ptr()) };
            let aroon_output_len = high.len() - (aroon_start_index as usize);

            // C Tulip aroon returns both aroon_up and aroon_down
            let mut c_aroon_up_output = vec![0.0_f64; aroon_output_len];
            let mut c_aroon_down_output = vec![0.0_f64; aroon_output_len];
            let mut aroon_outputs_c: Vec<*mut f64> = vec![
                c_aroon_down_output.as_mut_ptr(),
                c_aroon_up_output.as_mut_ptr(),
            ];
            let ret = unsafe {
                ti_aroon(
                    high.len() as i32,
                    aroon_inputs_c.as_ptr(),
                    options.as_ptr(),
                    aroon_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_aroon returned error code {}", ret);

            // Compare aroon_up outputs from the end backwards for better alignment
            let compare_len = rust_aroon_up.len().min(c_aroon_up_output.len());
            for i in 0..compare_len {
                let rust_idx = rust_aroon_up.len() - 1 - i;
                let c_idx = c_aroon_up_output.len() - 1 - i;
                let rust_val = rust_aroon_up[rust_idx];
                let c_val = c_aroon_up_output[c_idx];

                if rust_val.is_nan() {
                    panic!(
                        "Rust aroon_up has NaN at index {} (from end): Rust = {}, Options = {:?}",
                        i, rust_val, options
                    );
                }
                if rust_val.is_infinite() {
                    panic!(
                        "Rust aroon_up has infinity at index {} (from end): Rust = {}",
                        i, rust_val
                    );
                }
                if c_val.is_nan() && !rust_val.is_nan() {
                    continue;
                }
                if c_val.is_infinite() && !rust_val.is_infinite() {
                    continue;
                }

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = EPSILON),
                    "Aroon Up mismatch at index {} (from end): C = {}, Rust = {} for options {:?}",
                    i,
                    c_val,
                    rust_val,
                    options
                );
            }

            // Compare aroon_down outputs from the end backwards for better alignment
            let compare_len = rust_aroon_down.len().min(c_aroon_down_output.len());
            for i in 0..compare_len {
                let rust_idx = rust_aroon_down.len() - 1 - i;
                let c_idx = c_aroon_down_output.len() - 1 - i;
                let rust_val = rust_aroon_down[rust_idx];
                let c_val = c_aroon_down_output[c_idx];

                if rust_val.is_nan() {
                    panic!(
                        "Rust aroon_down has NaN at index {} (from end): Rust = {}, Options = {:?}",
                        i, rust_val, options
                    );
                }
                if rust_val.is_infinite() {
                    panic!(
                        "Rust aroon_down has infinity at index {} (from end): Rust = {}",
                        i, rust_val
                    );
                }
                if c_val.is_nan() && !rust_val.is_nan() {
                    continue;
                }
                if c_val.is_infinite() && !rust_val.is_infinite() {
                    continue;
                }

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = EPSILON),
                    "Aroon Down mismatch at index {} (from end): C = {}, Rust = {} for options {:?}",
                    i,
                    c_val,
                    rust_val,
                    options
                );
            }

            println!(
                "✓ Both optional outputs validated for options {:?}",
                options
            );
        }

        println!("✓ All AROONOSC optional outputs tests passed!");
    }

    #[test]
    fn test_aroonosc_optional_outputs_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);

            for options in OPTIONS_LIST {
                println!(
                    "Testing AROONOSC optional outputs with database stock {} and options: {:?}",
                    stock_symbol, options
                );

                // Get Rust AROONOSC with both optional outputs enabled
                let inputs_rust = [high.as_slice(), low.as_slice()];
                let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[true, true]))
                    .expect("Rust AROONOSC indicator with optional outputs failed");

                let rust_aroon_up = &rust_outputs[1]; // aroon_up is at index 1
                let rust_aroon_down = &rust_outputs[2]; // aroon_down is at index 2

                if rust_aroon_up.is_empty() {
                    panic!(
                        "Rust aroon_up optional output is empty for stock {}",
                        stock_symbol
                    );
                }
                if rust_aroon_down.is_empty() {
                    panic!(
                        "Rust aroon_down optional output is empty for stock {}",
                        stock_symbol
                    );
                }

                // Get C Tulip Aroon output for comparison (returns both up and down)
                let aroon_inputs_c = vec![high.as_ptr(), low.as_ptr()];
                let aroon_start_index = unsafe { ti_aroon_start(options.as_ptr()) };
                let aroon_output_len = high.len() - (aroon_start_index as usize);
                let mut c_aroon_up = vec![0.0; aroon_output_len];
                let mut c_aroon_down = vec![0.0; aroon_output_len];
                let mut aroon_outputs_c = vec![c_aroon_up.as_mut_ptr(), c_aroon_down.as_mut_ptr()];

                let ret = unsafe {
                    ti_aroon(
                        high.len() as i32,
                        aroon_inputs_c.as_ptr(),
                        options.as_ptr(),
                        aroon_outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(
                    ret, 0,
                    "ti_aroon returned error code {} for stock {}",
                    ret, stock_symbol
                );

                // Compare aroon_up from the end backwards
                let compare_len = rust_aroon_up.len().min(c_aroon_up.len());
                for i in 0..compare_len {
                    let rust_idx = rust_aroon_up.len() - 1 - i;
                    let c_idx = c_aroon_up.len() - 1 - i;
                    let rust_val = rust_aroon_up[rust_idx];
                    let c_val = c_aroon_up[c_idx];

                    if !rust_val.is_finite() {
                        panic!(
                            "Rust aroon_up output has NaN/infinity at index {} (from end): Rust = {} for stock {} options {:?}",
                            i, rust_val, stock_symbol, options
                        );
                    }

                    if !c_val.is_finite() {
                        continue; // Skip C library bugs
                    }

                    assert!(
                        approx_eq!(f64, c_val, rust_val, epsilon = AROON_EPSILON),
                        "Aroon_up mismatch at index {} (from end): C = {}, Rust = {} for stock {} options {:?}",
                        i,
                        c_val,
                        rust_val,
                        stock_symbol,
                        options
                    );
                }

                // Compare aroon_down from the end backwards
                let compare_len = rust_aroon_down.len().min(c_aroon_down.len());
                for i in 0..compare_len {
                    let rust_idx = rust_aroon_down.len() - 1 - i;
                    let c_idx = c_aroon_down.len() - 1 - i;
                    let rust_val = rust_aroon_down[rust_idx];
                    let c_val = c_aroon_down[c_idx];

                    if !rust_val.is_finite() {
                        panic!(
                            "Rust aroon_down output has NaN/infinity at index {} (from end): Rust = {} for stock {} options {:?}",
                            i, rust_val, stock_symbol, options
                        );
                    }

                    if !c_val.is_finite() {
                        continue; // Skip C library bugs
                    }

                    assert!(
                        approx_eq!(f64, c_val, rust_val, epsilon = AROON_EPSILON),
                        "Aroon_down mismatch at index {} (from end): C = {}, Rust = {} for stock {} options {:?}",
                        i,
                        c_val,
                        rust_val,
                        stock_symbol,
                        options
                    );
                }

                println!(
                    "✓ Optional outputs validated for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All AROONOSC optional outputs database tests passed!");
    }

    #[test]
    fn test_aroonosc_simd_vs_regular_database() {
        use tulip_rs::indicators::aroonosc::indicator_by_assets;

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
            // Test without optional outputs
            {
                // Get SIMD by assets result
                let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                    .expect("SIMD by assets AROONOSC indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, high, low)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock
                    let stock_inputs = [high.as_slice(), low.as_slice()];
                    let (regular_results, _) = indicator(&stock_inputs, &options, None)
                        .expect("Regular AROONOSC indicator failed");

                    let simd_result = &simd_results[stock_idx][0];
                    let regular_result = &regular_results[0];

                    // Compare output lengths
                    assert_eq!(
                        simd_result.len(),
                        regular_result.len(),
                        "AroonOsc output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
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
                                "SIMD by assets AROONOSC has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets AROONOSC has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with epsilon tolerance for AROONOSC
                        if (simd_val - regular_val).abs() > EPSILON {
                            println!(
                                "SIMD: {:?}\n\nRegular: {:?}",
                                &simd_result[..20.min(simd_result.len())],
                                &regular_result[..20.min(regular_result.len())]
                            );
                            panic!(
                                "AroonOsc mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
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
        }

        println!("✓ All SIMD by assets vs Regular AROONOSC database tests passed!");
    }

    #[test]
    fn test_aroonosc_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::aroonosc::indicator_by_assets;

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
            // Test with optional outputs enabled (Aroon Down and Aroon Up)
            {
                // Get SIMD by assets result with optional outputs
                let (simd_results, _) =
                    indicator_by_assets::<4>(&inputs, &options, Some(&[true, true]))
                        .expect("SIMD by assets AROONOSC indicator failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, high, low)) in stock_data.iter().enumerate() {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [high.as_slice(), low.as_slice()];
                    let (regular_results, _) =
                        indicator(&stock_inputs, &options, Some(&[true, true]))
                            .expect("Regular AROONOSC indicator failed");

                    let simd_aroonosc_result = &simd_results[stock_idx][0];
                    let simd_aroon_down_result = &simd_results[stock_idx][1];
                    let simd_aroon_up_result = &simd_results[stock_idx][2];
                    let regular_aroonosc_result = &regular_results[0];
                    let regular_aroon_down_result = &regular_results[1];
                    let regular_aroon_up_result = &regular_results[2];

                    // Compare AroonOsc output lengths
                    assert_eq!(
                        simd_aroonosc_result.len(),
                        regular_aroonosc_result.len(),
                        "AroonOsc output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                        stock_symbol,
                        options,
                        simd_aroonosc_result.len(),
                        regular_aroonosc_result.len()
                    );

                    // Compare Aroon Down output lengths
                    assert_eq!(
                        simd_aroon_down_result.len(),
                        regular_aroon_down_result.len(),
                        "Aroon Down output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                        stock_symbol,
                        options,
                        simd_aroon_down_result.len(),
                        regular_aroon_down_result.len()
                    );

                    // Compare Aroon Up output lengths
                    assert_eq!(
                        simd_aroon_up_result.len(),
                        regular_aroon_up_result.len(),
                        "Aroon Up output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                        stock_symbol,
                        options,
                        simd_aroon_up_result.len(),
                        regular_aroon_up_result.len()
                    );

                    // Compare AroonOsc values
                    for (i, (&simd_val, &regular_val)) in simd_aroonosc_result
                        .iter()
                        .zip(regular_aroonosc_result.iter())
                        .enumerate()
                    {
                        if (simd_val - regular_val).abs() > EPSILON {
                            panic!(
                                "AroonOsc mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    // Compare Aroon Down values
                    for (i, (&simd_val, &regular_val)) in simd_aroon_down_result
                        .iter()
                        .zip(regular_aroon_down_result.iter())
                        .enumerate()
                    {
                        if (simd_val - regular_val).abs() > AROON_EPSILON {
                            panic!(
                                "Aroon Down mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    // Compare Aroon Up values
                    for (i, (&simd_val, &regular_val)) in simd_aroon_up_result
                        .iter()
                        .zip(regular_aroon_up_result.iter())
                        .enumerate()
                    {
                        if (simd_val - regular_val).abs() > AROON_EPSILON {
                            panic!(
                                "Aroon Up mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    println!(
                        "✓ SIMD by assets vs Regular optional outputs test passed for stock {} with options {:?}",
                        stock_symbol, options
                    );
                }
            }
        }

        println!(
            "✓ All SIMD by assets vs Regular AROONOSC optional outputs database tests passed!"
        );
    }

    #[test]
    fn test_aroonosc_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::aroonosc::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let inputs = [high.as_slice(), low.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4_first = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4_first, _) =
                indicator_by_options::<4>(&inputs, &options_4_first, None)
                    .expect("SIMD AROONOSC 4-wide first failed");

            // Process second 4 options with 4-wide SIMD
            let options_4_second = [
                &OPTIONS_LIST[4],
                &OPTIONS_LIST[5],
                &OPTIONS_LIST[6],
                &OPTIONS_LIST[7],
            ];
            let (simd_results_4_second, _) =
                indicator_by_options::<4>(&inputs, &options_4_second, None)
                    .expect("SIMD AROONOSC 4-wide second failed");

            // Combine all SIMD results
            let mut all_simd_results = simd_results_4_first;
            all_simd_results.extend(simd_results_4_second);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    indicator(&inputs, options, None).expect("Regular AROONOSC indicator failed");

                let simd_result = &all_simd_results[idx];
                let regular_result = &regular_results;

                // Compare output lengths for AroonOsc
                assert_eq!(
                    simd_result[0].len(),
                    regular_result[0].len(),
                    "AroonOsc output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[0].len(),
                    regular_result[0].len()
                );

                // Compare AroonOsc values
                for (i, (&simd_val, &regular_val)) in simd_result[0]
                    .iter()
                    .zip(regular_result[0].iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD AroonOsc has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD AroonOsc has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if (simd_val - regular_val).abs() > EPSILON {
                        panic!(
                            "AroonOsc mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular AROONOSC database tests passed!");
    }

    #[test]
    fn test_aroonosc_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::aroonosc::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let inputs = [high.as_slice(), low.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4_first = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4_first, _) =
                indicator_by_options::<4>(&inputs, &options_4_first, Some(&[true, true]))
                    .expect("SIMD AROONOSC 4-wide first failed");

            // Process second 4 options with 4-wide SIMD
            let options_4_second = [
                &OPTIONS_LIST[4],
                &OPTIONS_LIST[5],
                &OPTIONS_LIST[6],
                &OPTIONS_LIST[7],
            ];
            let (simd_results_4_second, _) =
                indicator_by_options::<4>(&inputs, &options_4_second, Some(&[true, true]))
                    .expect("SIMD AROONOSC 4-wide second failed");

            // Combine all SIMD results
            let mut all_simd_results = simd_results_4_first;
            all_simd_results.extend(simd_results_4_second);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = indicator(&inputs, options, Some(&[true, true]))
                    .expect("Regular AROONOSC indicator failed");

                let simd_result = &all_simd_results[idx];
                let regular_result = &regular_results;

                // Compare AroonOsc output lengths
                assert_eq!(
                    simd_result[0].len(),
                    regular_result[0].len(),
                    "AroonOsc output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[0].len(),
                    regular_result[0].len()
                );

                // Compare Aroon Down output lengths
                assert_eq!(
                    simd_result[1].len(),
                    regular_result[1].len(),
                    "Aroon Down output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[1].len(),
                    regular_result[1].len()
                );

                // Compare Aroon Up output lengths
                assert_eq!(
                    simd_result[2].len(),
                    regular_result[2].len(),
                    "Aroon Up output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[2].len(),
                    regular_result[2].len()
                );

                // Compare AroonOsc values
                for (i, (&simd_val, &regular_val)) in simd_result[0]
                    .iter()
                    .zip(regular_result[0].iter())
                    .enumerate()
                {
                    if (simd_val - regular_val).abs() > EPSILON {
                        panic!(
                            "AroonOsc mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Aroon Down values
                for (i, (&simd_val, &regular_val)) in simd_result[1]
                    .iter()
                    .zip(regular_result[1].iter())
                    .enumerate()
                {
                    if (simd_val - regular_val).abs() > AROON_EPSILON {
                        panic!(
                            "Aroon Down mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Aroon Up values
                for (i, (&simd_val, &regular_val)) in simd_result[2]
                    .iter()
                    .zip(regular_result[2].iter())
                    .enumerate()
                {
                    if (simd_val - regular_val).abs() > AROON_EPSILON {
                        panic!(
                            "Aroon Up mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular AROONOSC optional outputs database tests passed!"
        );
    }

    //add test code here
}
