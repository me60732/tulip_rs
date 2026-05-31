#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::cci::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{
        ti_cci, ti_cci_start, ti_md, ti_md_start, ti_sma, ti_sma_start, ti_typprice,
        ti_typprice_start,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-8;
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

    const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [14.0], [20.0], [25.0], [30.0], [50.0]];

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
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..500 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
        }
        (high_vec, low_vec, close_vec)
    }

    #[test]
    fn test_cci_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            // Determine the offset required by the C CCI function
            let start_index = unsafe { ti_cci_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_cci_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut cci_output_vec_c = vec![0.0_f64; output_len_c];
            let cci_ptr: *mut f64 = cci_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![cci_ptr];
            let ret = unsafe {
                ti_cci(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_cci returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Rust CCI indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in cci_output_vec_c
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
                        "Rust CCI has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust CCI has infinity at index {}: Rust = {}, Options = {:?}",
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
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, cci_output_vec_c, outputs[0], options
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
    fn test_cci_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

                let start_index = unsafe { ti_cci_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_cci_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut cci_output_vec_c = vec![0.0_f64; output_len_c];
                let cci_ptr: *mut f64 = cci_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![cci_ptr];
                let ret = unsafe {
                    ti_cci(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_cci returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (outputs, _) =
                    indicator(&inputs_rust, &options, None).expect("Rust CCI indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare results
                for (i, (&c_val, &rust_val)) in cci_output_vec_c
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
                            "Rust CCI has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust CCI has infinity at index {}: Rust = {}, Options = {:?}, Stock: {}",
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

                    if !approx_eq!(f64, c_val, rust_val, epsilon = EPSILON) {
                        println!(
                            "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, cci_output_vec_c, outputs[0], options, stock_symbol
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
    fn test_cci_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = indicator(&inputs_rust, &options, None)
                    .expect("Failed to run CCI indicator on full data");

                // Process in batches
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) = indicator(&inputs_rust, &options, None)
                        .expect("Failed to run CCI indicator");
                    batch_full_output.extend_from_slice(&outputs[0]);
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let high_vec = high[..min_data_val].to_vec();
                    let low_vec = low[..min_data_val].to_vec();
                    let close_vec = close[..min_data_val].to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                    ];

                    let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("Failed to run CCI indicator on first chunk");
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
                            .expect("CCI batch indicator failed");
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
                            .expect("CCI batch indicator failed");
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
    fn test_cci_typprice_optional_output_vs_c_tulip() {
        // Test CCI's typprice optional output against C Tulip's typprice implementation
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            println!(
                "Testing CCI typprice optional output with options: {:?}",
                options
            );

            // Run the Rust implementation with typprice optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[false, false, true]))
                .expect("Rust CCI indicator failed");

            // Extract the typprice optional output (fourth output)
            let rust_typprice = &rust_outputs[3];

            // Fail immediately if typprice output is empty (indicator bug)
            if rust_typprice.is_empty() {
                panic!(
                    "Rust typprice optional output is empty with options {:?} - indicator bug in optional output handling",
                    options
                );
            }

            // Run the C implementation for typprice
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            // No options needed for typprice - it takes no parameters
            let typprice_options: Vec<f64> = vec![];
            let start_index = unsafe { ti_typprice_start(typprice_options.as_ptr()) };
            assert!(
                start_index >= 0,
                "ti_typprice_start returned a negative index"
            );
            let output_len_c = close.len() - (start_index as usize);

            let mut typprice_output_vec_c = vec![0.0_f64; output_len_c];
            let typprice_ptr: *mut f64 = typprice_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![typprice_ptr];

            let ret = unsafe {
                ti_typprice(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    typprice_options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_typprice returned error code {}", ret);

            // Compare outputs using reverse processing pattern
            let mut compared_values = 0;
            for (rust_val, c_val) in rust_typprice
                .iter()
                .rev()
                .zip(typprice_output_vec_c.iter().rev())
            {
                compared_values += 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust typprice has NaN: Rust = {}, Options = {:?}",
                        rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust typprice has infinity: Rust = {}, Options = {:?}",
                        rust_val, options
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

                if !approx_eq!(f64, *c_val, *rust_val, epsilon = EPSILON) {
                    println!(
                        "Typprice test failed: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        typprice_output_vec_c, rust_typprice, options
                    );
                    panic!(
                        "Typprice mismatch: C = {}, Rust = {}, Options = {:?}",
                        c_val, rust_val, options
                    );
                }
            }

            println!(
                "CCI typprice optional output test passed for options {:?} (compared {} values)",
                options, compared_values
            );
        }
    }

    #[test]
    fn test_cci_sma_optional_output_vs_c_tulip() {
        // Test CCI's sma optional output against C Tulip's SMA on typprice
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            println!(
                "Testing CCI SMA optional output with options: {:?}",
                options
            );

            // Run the Rust implementation with SMA optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[true, false, false]))
                .expect("Rust CCI indicator failed");

            // Extract the SMA optional output (second output)
            let rust_sma = &rust_outputs[1];

            // Fail immediately if SMA output is empty (indicator bug)
            if rust_sma.is_empty() {
                panic!(
                    "Rust SMA optional output is empty with options {:?} - indicator bug in optional output handling",
                    options
                );
            }

            // Generate typprice first using C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            let typprice_options: Vec<f64> = vec![];
            let typprice_start_index = unsafe { ti_typprice_start(typprice_options.as_ptr()) };
            let typprice_output_len_c = close.len() - (typprice_start_index as usize);

            let mut typprice_output_vec_c = vec![0.0_f64; typprice_output_len_c];
            let typprice_ptr: *mut f64 = typprice_output_vec_c.as_mut_ptr();
            let mut typprice_outputs_c: Vec<*mut f64> = vec![typprice_ptr];

            let ret = unsafe {
                ti_typprice(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    typprice_options.as_ptr(),
                    typprice_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_typprice returned error code {}", ret);

            // Now run SMA on the typprice
            let typprice_input_c: Vec<*const f64> = vec![typprice_output_vec_c.as_ptr()];

            let start_index = unsafe { ti_sma_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_sma_start returned a negative index");
            let output_len_c = typprice_output_len_c - (start_index as usize);

            let mut sma_output_vec_c = vec![0.0_f64; output_len_c];
            let sma_ptr: *mut f64 = sma_output_vec_c.as_mut_ptr();
            let mut sma_outputs_c: Vec<*mut f64> = vec![sma_ptr];

            let ret = unsafe {
                ti_sma(
                    typprice_output_len_c as i32,
                    typprice_input_c.as_ptr(),
                    options.as_ptr(),
                    sma_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_sma returned error code {}", ret);

            // Compare outputs using reverse processing pattern
            let mut compared_values = 0;
            for (rust_val, c_val) in rust_sma.iter().rev().zip(sma_output_vec_c.iter().rev()) {
                compared_values += 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust SMA has NaN: Rust = {}, Options = {:?}",
                        rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust SMA has infinity: Rust = {}, Options = {:?}",
                        rust_val, options
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

                if !approx_eq!(f64, *c_val, *rust_val, epsilon = EPSILON) {
                    println!(
                        "SMA test failed: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        sma_output_vec_c, rust_sma, options
                    );
                    panic!(
                        "SMA mismatch: C = {}, Rust = {}, Options = {:?}",
                        c_val, rust_val, options
                    );
                }
            }

            println!(
                "CCI SMA optional output test passed for options {:?} (compared {} values)",
                options, compared_values
            );
        }
    }

    #[test]
    fn test_cci_md_optional_output_vs_c_tulip() {
        // Test CCI's md optional output against C Tulip's MD implementation
        let (high, low, close) = expand_inputs();

        for options in OPTIONS_LIST {
            println!("Testing CCI MD optional output with options: {:?}", options);

            // Run the Rust implementation with MD optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &options, Some(&[false, true, false]))
                .expect("Rust CCI indicator failed");

            // Extract the MD optional output (third output)
            let rust_md = &rust_outputs[2];

            // Fail immediately if MD output is empty (indicator bug)
            if rust_md.is_empty() {
                panic!(
                    "Rust MD optional output is empty with options {:?} - indicator bug in optional output handling",
                    options
                );
            }

            // Generate typprice first using C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            let typprice_options: Vec<f64> = vec![];
            let typprice_start_index = unsafe { ti_typprice_start(typprice_options.as_ptr()) };
            let typprice_output_len_c = close.len() - (typprice_start_index as usize);

            let mut typprice_output_vec_c = vec![0.0_f64; typprice_output_len_c];
            let typprice_ptr: *mut f64 = typprice_output_vec_c.as_mut_ptr();
            let mut typprice_outputs_c: Vec<*mut f64> = vec![typprice_ptr];

            let ret = unsafe {
                ti_typprice(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    typprice_options.as_ptr(),
                    typprice_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_typprice returned error code {}", ret);

            // Now run MD on the typprice
            let typprice_input_c: Vec<*const f64> = vec![typprice_output_vec_c.as_ptr()];

            let start_index = unsafe { ti_md_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_md_start returned a negative index");
            let output_len_c = typprice_output_len_c - (start_index as usize);

            let mut md_output_vec_c = vec![0.0_f64; output_len_c];
            let md_ptr: *mut f64 = md_output_vec_c.as_mut_ptr();
            let mut md_outputs_c: Vec<*mut f64> = vec![md_ptr];

            let ret = unsafe {
                ti_md(
                    typprice_output_len_c as i32,
                    typprice_input_c.as_ptr(),
                    options.as_ptr(),
                    md_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_md returned error code {}", ret);

            // Compare outputs using reverse processing pattern
            let mut compared_values = 0;
            for (rust_val, c_val) in rust_md.iter().rev().zip(md_output_vec_c.iter().rev()) {
                compared_values += 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust MD has NaN: Rust = {}, Options = {:?}",
                        rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust MD has infinity: Rust = {}, Options = {:?}",
                        rust_val, options
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

                if !approx_eq!(f64, *c_val, *rust_val, epsilon = EPSILON) {
                    println!(
                        "MD test failed: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        md_output_vec_c, rust_md, options
                    );
                    panic!(
                        "MD mismatch: C = {}, Rust = {}, Options = {:?}",
                        c_val, rust_val, options
                    );
                }
            }

            println!(
                "CCI MD optional output test passed for options {:?} (compared {} values)",
                options, compared_values
            );
        }
    }

    #[test]
    fn test_cci_database_optional_typprice() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let (high, low, close) = get_hlc_arrays(stock_data);

            for &options in &OPTIONS_LIST {
                // Get CCI with typprice optional output
                let optional_outputs = Some(&[false, false, true][..]);
                let (cci_result, _) = tulip_rs::indicators::cci::indicator(
                    &[&high, &low, &close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_typprice = &cci_result[3];

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
                        "CCI typprice optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_cci_database_optional_sma() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let (high, low, close) = get_hlc_arrays(stock_data);

            for &options in &OPTIONS_LIST {
                // Get CCI with SMA optional output
                let optional_outputs = Some(&[true, false, false][..]);
                let (cci_result, _) = tulip_rs::indicators::cci::indicator(
                    &[&high, &low, &close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_sma = &cci_result[1];

                // Generate typprice first using C implementation
                let typprice_options: Vec<f64> = vec![];
                let typprice_start_index = unsafe { ti_typprice_start(typprice_options.as_ptr()) };
                let typprice_output_len_c = high.len() - (typprice_start_index as usize);

                let mut typprice_output_vec_c = vec![0.0_f64; typprice_output_len_c];
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let mut typprice_outputs_c: Vec<*mut f64> =
                    vec![typprice_output_vec_c.as_mut_ptr()];

                unsafe {
                    let ret = ti_typprice(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        typprice_options.as_ptr(),
                        typprice_outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_typprice failed");
                }

                // Calculate expected SMA using C Tulip ti_sma on typprice
                let period_options = [options[0]];
                let sma_start_index = unsafe { ti_sma_start(period_options.as_ptr()) };
                let sma_output_len_c = typprice_output_vec_c.len() - (sma_start_index as usize);

                let mut c_sma_output = vec![0.0; sma_output_len_c];
                let typprice_inputs_c: Vec<*const f64> = vec![typprice_output_vec_c.as_ptr()];
                let mut sma_outputs_c: Vec<*mut f64> = vec![c_sma_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_sma(
                        typprice_output_vec_c.len() as i32,
                        typprice_inputs_c.as_ptr(),
                        period_options.as_ptr(),
                        sma_outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_sma failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_sma.len().min(c_sma_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_sma.len() - 1 - i;
                    let c_idx = c_sma_output.len() - 1 - i;

                    let rust_val = rust_sma[rust_idx];
                    let c_val = c_sma_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust SMA output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "CCI SMA optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_cci_database_optional_md() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 50 {
                continue;
            }

            let (high, low, close) = get_hlc_arrays(stock_data);

            for &options in &OPTIONS_LIST {
                // Get CCI with MD optional output
                let optional_outputs = Some(&[false, true, false][..]);
                let (cci_result, _) = tulip_rs::indicators::cci::indicator(
                    &[&high, &low, &close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_md = &cci_result[2];

                // Generate typprice first using C implementation
                let typprice_options: Vec<f64> = vec![];
                let typprice_start_index = unsafe { ti_typprice_start(typprice_options.as_ptr()) };
                let typprice_output_len_c = high.len() - (typprice_start_index as usize);

                let mut typprice_output_vec_c = vec![0.0_f64; typprice_output_len_c];
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let mut typprice_outputs_c: Vec<*mut f64> =
                    vec![typprice_output_vec_c.as_mut_ptr()];

                unsafe {
                    let ret = ti_typprice(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        typprice_options.as_ptr(),
                        typprice_outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_typprice failed");
                }

                // Calculate expected MD using C Tulip ti_md on typprice
                let period_options = [options[0]];
                let md_start_index = unsafe { ti_md_start(period_options.as_ptr()) };
                let md_output_len_c = typprice_output_vec_c.len() - (md_start_index as usize);

                let mut c_md_output = vec![0.0; md_output_len_c];
                let typprice_inputs_c: Vec<*const f64> = vec![typprice_output_vec_c.as_ptr()];
                let mut md_outputs_c: Vec<*mut f64> = vec![c_md_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_md(
                        typprice_output_vec_c.len() as i32,
                        typprice_inputs_c.as_ptr(),
                        period_options.as_ptr(),
                        md_outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_md failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_md.len().min(c_md_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_md.len() - 1 - i;
                    let c_idx = c_md_output.len() - 1 - i;

                    let rust_val = rust_md[rust_idx];
                    let c_val = c_md_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust MD output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "CCI MD optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
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
    fn test_cci_simd_vs_regular_database() {
        use tulip_rs::indicators::cci::indicator_by_assets;

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

        for options in OPTIONS_LIST {
            // Test without optional outputs
            {
                // Get SIMD by assets result
                let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                    .expect("SIMD by assets CCI indicator failed");

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
                    let (regular_results, _) = indicator(&stock_inputs, &options, None)
                        .expect("Regular CCI indicator failed");

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
                                "SIMD by assets CCI has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        if simd_val.is_infinite() {
                            panic!(
                                "SIMD by assets CCI has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                                i, stock_symbol, options, simd_val
                            );
                        }

                        // Compare values with appropriate epsilon for CCI
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
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
        }

        println!("✓ All SIMD by assets vs Regular CCI database tests passed!");
    }

    #[test]
    fn test_cci_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::cci::indicator_by_assets;

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

        for options in OPTIONS_LIST {
            // Test with all optional outputs enabled (typprice, sma, md)
            {
                let optional_outputs = Some(&[true, true, true] as &[bool]);

                // Get SIMD by assets result with optional outputs
                let (simd_results, _) =
                    indicator_by_assets::<4>(&inputs, &options, optional_outputs)
                        .expect("SIMD by assets CCI indicator with optional outputs failed");

                // Compare each SIMD result with regular indicator for each stock
                for (stock_idx, (stock_symbol, stock_high, stock_low, stock_close)) in
                    stock_data.iter().enumerate()
                {
                    // Get regular indicator result for this stock with optional outputs
                    let stock_inputs = [
                        stock_high.as_slice(),
                        stock_low.as_slice(),
                        stock_close.as_slice(),
                    ];
                    let (regular_results, _) = indicator(&stock_inputs, &options, optional_outputs)
                        .expect("Regular CCI indicator with optional outputs failed");

                    // Compare CCI output (index 0)
                    let simd_cci_result = &simd_results[stock_idx][0];
                    let regular_cci_result = &regular_results[0];

                    assert_eq!(
                        simd_cci_result.len(),
                        regular_cci_result.len(),
                        "CCI output length mismatch for stock {} with options {:?}",
                        stock_symbol,
                        options
                    );

                    for (i, (&simd_val, &regular_val)) in simd_cci_result
                        .iter()
                        .zip(regular_cci_result.iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "CCI mismatch at index {} for stock {} with options {:?}: SIMD = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    // Compare TYPPRICE output (index 1)
                    let simd_typprice_result = &simd_results[stock_idx][1];
                    let regular_typprice_result = &regular_results[1];

                    assert_eq!(
                        simd_typprice_result.len(),
                        regular_typprice_result.len(),
                        "TYPPRICE output length mismatch for stock {} with options {:?}",
                        stock_symbol,
                        options
                    );

                    for (i, (&simd_val, &regular_val)) in simd_typprice_result
                        .iter()
                        .zip(regular_typprice_result.iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "TYPPRICE mismatch at index {} for stock {} with options {:?}: SIMD = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    // Compare SMA output (index 2)
                    let simd_sma_result = &simd_results[stock_idx][2];
                    let regular_sma_result = &regular_results[2];

                    assert_eq!(
                        simd_sma_result.len(),
                        regular_sma_result.len(),
                        "SMA output length mismatch for stock {} with options {:?}",
                        stock_symbol,
                        options
                    );

                    for (i, (&simd_val, &regular_val)) in simd_sma_result
                        .iter()
                        .zip(regular_sma_result.iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "SMA mismatch at index {} for stock {} with options {:?}: SIMD = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    // Compare MD output (index 3)
                    let simd_md_result = &simd_results[stock_idx][3];
                    let regular_md_result = &regular_results[3];

                    assert_eq!(
                        simd_md_result.len(),
                        regular_md_result.len(),
                        "MD output length mismatch for stock {} with options {:?}",
                        stock_symbol,
                        options
                    );

                    for (i, (&simd_val, &regular_val)) in simd_md_result
                        .iter()
                        .zip(regular_md_result.iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                            panic!(
                                "MD mismatch at index {} for stock {} with options {:?}: SIMD = {}, Regular = {}",
                                i, stock_symbol, options, simd_val, regular_val
                            );
                        }
                    }

                    println!(
                        "✓ SIMD by assets vs Regular test (with optional outputs) passed for stock {} with options {:?}",
                        stock_symbol, options
                    );
                }
            }
        }

        println!(
            "✓ All SIMD by assets vs Regular CCI database tests with optional outputs passed!"
        );
    }

    #[test]
    fn test_cci_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::cci::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD CCI 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD CCI 2-wide failed");

            // Combine all SIMD results
            let mut all_simd_results = simd_results_4;
            all_simd_results.extend(simd_results_2);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    indicator(&inputs, options, None).expect("Regular CCI indicator failed");

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
                            "SIMD CCI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD CCI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
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

        println!("✓ All SIMD by options vs Regular CCI database tests passed!");
    }

    #[test]
    fn test_cci_simd_by_options_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::cci::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Test with SMA, MD, and typprice optional outputs
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
                    .expect("SIMD CCI 4-wide with optional outputs failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) =
                indicator_by_options::<2>(&inputs, &options_2, optional_outputs)
                    .expect("SIMD CCI 2-wide with optional outputs failed");

            // Combine all SIMD results
            let mut all_simd_results = simd_results_4;
            all_simd_results.extend(simd_results_2);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = indicator(&inputs, options, optional_outputs)
                    .expect("Regular CCI indicator with optional outputs failed");

                let simd_cci_result = &all_simd_results[idx][0];
                let regular_cci_result = &regular_results[0];

                let simd_sma_result = &all_simd_results[idx][1];
                let regular_sma_result = &regular_results[1];

                let simd_md_result = &all_simd_results[idx][2];
                let regular_md_result = &regular_results[2];

                let simd_typprice_result = &all_simd_results[idx][3];
                let regular_typprice_result = &regular_results[3];

                // Compare CCI output lengths
                assert_eq!(
                    simd_cci_result.len(),
                    regular_cci_result.len(),
                    "CCI output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_cci_result.len(),
                    regular_cci_result.len()
                );

                // Compare SMA output lengths
                assert_eq!(
                    simd_sma_result.len(),
                    regular_sma_result.len(),
                    "SMA output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_sma_result.len(),
                    regular_sma_result.len()
                );

                // Compare MD output lengths
                assert_eq!(
                    simd_md_result.len(),
                    regular_md_result.len(),
                    "MD output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_md_result.len(),
                    regular_md_result.len()
                );

                // Compare typprice output lengths
                assert_eq!(
                    simd_typprice_result.len(),
                    regular_typprice_result.len(),
                    "Typprice output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_typprice_result.len(),
                    regular_typprice_result.len()
                );

                // Compare CCI values
                for (i, (&simd_val, &regular_val)) in simd_cci_result
                    .iter()
                    .zip(regular_cci_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD CCI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD CCI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "CCI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare SMA values
                for (i, (&simd_val, &regular_val)) in simd_sma_result
                    .iter()
                    .zip(regular_sma_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD SMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD SMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "SMA mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare MD values
                for (i, (&simd_val, &regular_val)) in simd_md_result
                    .iter()
                    .zip(regular_md_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD MD has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD MD has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "MD mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare typprice values
                for (i, (&simd_val, &regular_val)) in simd_typprice_result
                    .iter()
                    .zip(regular_typprice_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD typprice has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD typprice has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
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
            "✓ All SIMD by options vs Regular CCI database tests with optional outputs passed!"
        );
    }

    #[test]
    fn test_cci_simd_state_handover_by_options() {
        use tulip_rs::indicators::cci::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // number of bars to process with SIMD first
        let first_bars = 2000usize;

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let total_len = high.len();
            if total_len == 0 {
                continue;
            }

            let split = first_bars.min(total_len);

            // prepare slices for first part and remaining
            let first_inputs = [&high[..split], &low[..split], &close[..split]];
            let remaining_inputs = if split < total_len {
                Some([&high[split..], &low[split..], &close[split..]])
            } else {
                None
            };

            // process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, states_4) =
                indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD CCI 4-wide failed on first chunk");

            // process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, states_2) =
                indicator_by_options::<2>(&first_inputs, &options_2, None)
                    .expect("SIMD CCI 2-wide failed on first chunk");

            // Combine SIMD results for first part and prepare to extend with batch_indicator outputs
            let mut all_simd_results: Vec<Vec<f64>> = Vec::new();
            for result in &simd_results_4 {
                all_simd_results.push(result[0].clone());
            }
            for result in &simd_results_2 {
                all_simd_results.push(result[0].clone());
            }

            // If there is remaining data, use the returned states to process it
            if let Some(rem_inputs) = remaining_inputs {
                // states_4 and states_2 are Vec<IndicatorState>
                for (i, mut st) in states_4.into_iter().enumerate() {
                    let chunk_out = st.batch_indicator(&rem_inputs, None).expect("batch failed");
                    all_simd_results[i].extend_from_slice(&chunk_out[0]);
                }
                for (j, mut st) in states_2.into_iter().enumerate() {
                    let idx = 4 + j;
                    let chunk_out = st.batch_indicator(&rem_inputs, None).expect("batch failed");
                    all_simd_results[idx].extend_from_slice(&chunk_out[0]);
                }
            }

            // Compare each SIMD result with regular indicator over the full data
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) = indicator(
                    &[high.as_slice(), low.as_slice(), close.as_slice()],
                    options,
                    None,
                )
                .expect("Regular CCI indicator failed");
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
                    if !approx_eq!(f64, r, s, epsilon = EPSILON) {
                        panic!(
                            "Mismatch stock {} option {:?} index {}: regular = {}, simd = {}",
                            stock_symbol, options, k, r, s
                        );
                    }
                }
            }
        }

        println!("✓ All CCI SIMD state handover by options tests passed!");
    }
}
