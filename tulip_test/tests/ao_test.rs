#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::ao::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{
        ti_ao, ti_ao_start, ti_medprice, ti_medprice_start, ti_sma, ti_sma_start,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const EPSILON: f64 = 1e-12;
    const SMA_EPSILON: f64 = 1e-10; // Use epsilon from sma_test.rs
    const MEDPRICE_EPSILON: f64 = 1e-12; // Use epsilon from medprice_test.rs
    const CHUNK_SIZE: usize = 100;

    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];

    const OPTIONS: [f64; 0] = [];

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
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
        }
        (high_vec, low_vec)
    }

    #[test]
    fn test_ao_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low) = expand_inputs();

        // Prepare inputs for the C implementation
        let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

        // Determine the offset required by the C AO function
        let start_index = unsafe { ti_ao_start(OPTIONS.as_ptr()) };
        assert!(start_index >= 0, "ti_ao_start returned a negative index");
        let output_len = high.len() - (start_index as usize);

        // Run the C implementation
        let mut output_vec_c = vec![0.0_f64; output_len];
        let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
        let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
        let ret = unsafe {
            ti_ao(
                high.len() as i32,
                inputs_c.as_ptr(),
                OPTIONS.as_ptr(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_ao returned error code {}", ret);

        // Run the Rust implementation
        let inputs_rust = [high.as_slice(), low.as_slice()];
        let (outputs, _) =
            indicator(&inputs_rust, &OPTIONS, None).expect("Rust AO indicator failed");

        // Compare the outputs
        // Compare the outputs in reverse for the length of the Rust outputs
        let output_len_rust = outputs[0].len();
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
                    "Rust AO has NaN at index {}: Rust = {}, Options = {:?}",
                    index, rust_val, OPTIONS
                );
            }

            // Fail test if Rust has infinity
            if rust_val.is_infinite() {
                panic!(
                    "Rust AO has infinity at index {}: Rust = {}",
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
                "Mismatch at index {}: C = {}, Rust = {}",
                index,
                c_val,
                rust_val
            );
        }
    }

    #[test]
    fn test_ao_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);

            // C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            let start_index = unsafe { ti_ao_start(std::ptr::null()) };
            assert!(start_index >= 0, "ti_ao_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            let mut output_vec_c = vec![0.0_f64; output_len_c];
            let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
            let ret = unsafe {
                ti_ao(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_ao returned error code {}", ret);

            // Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (outputs, _) =
                indicator(&inputs_rust, &[], None).expect("Rust AO indicator failed");

            let output_len_rust = outputs[0].len();
            let options: [f64; 0] = [];
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
                        "Rust AO has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                        index, rust_val, options, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust AO has infinity at index {}: Rust = {}",
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
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Stock: {}",
                        index, output_vec_c, outputs[0], stock_symbol
                    );
                    panic!(
                        "Mismatch at index {}: C = {}, Rust = {}",
                        index, c_val, rust_val
                    );
                }
            }
        }
    }

    #[test]
    fn test_ao_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let options: [f64; 0] = [];

            // Get full output
            let (full_outputs, _) = indicator(&inputs_rust, &options, None)
                .expect("Failed to run AO indicator on full data");

            // Process in batches
            let mut batch_full_output = Vec::new();

            let min_data_val = min_data(&options).max(CHUNK_SIZE);

            if high.len() <= min_data_val {
                // If data is too small, just run full calculation
                let (outputs, _) =
                    indicator(&inputs_rust, &options, None).expect("Failed to run AO indicator");
                batch_full_output.extend_from_slice(&outputs[0]);
            } else {
                // First chunk - convert to Vec<&Vec<f64>>
                let high_vec = high[..min_data_val].to_vec();
                let low_vec = low[..min_data_val].to_vec();
                let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];

                let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                    .expect("Failed to run AO indicator on first chunk");
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
                        .expect("AO batch indicator failed");
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
                        .expect("AO batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }
            }

            // Compare outputs
            assert_eq!(
                full_outputs[0].len(),
                batch_full_output.len(),
                "Output length mismatch for stock {}: full={}, batch={}",
                stock_symbol,
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
                    "Mismatch at index {} for stock {}: full={}, batch={}",
                    i, stock_symbol, full_val, batch_val
                );
            }
        }
    }

    #[test]
    fn test_ao_short_sma_optional_output_vs_c_tulip() {
        // Test the short SMA optional output against C Tulip SMA of medprice
        let (high, low) = expand_inputs();

        println!("Testing AO short SMA optional output");

        // Get Rust AO with short SMA optional output enabled
        let inputs_rust = [high.as_slice(), low.as_slice()];
        let (rust_outputs, _) = indicator(&inputs_rust, &OPTIONS, Some(&[true, false, false]))
            .expect("Rust AO indicator with short SMA optional output failed");

        let rust_short_sma = &rust_outputs[1]; // short_sma is at index 1

        // First, get medprice values from C Tulip to feed into SMA
        let inputs_c_medprice: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];
        let medprice_options = [];
        let medprice_start_index = unsafe { ti_medprice_start(medprice_options.as_ptr()) };
        let medprice_output_len = high.len() - (medprice_start_index as usize);

        let mut c_medprice_values = vec![0.0_f64; medprice_output_len];
        let mut medprice_outputs_c: Vec<*mut f64> = vec![c_medprice_values.as_mut_ptr()];
        let ret = unsafe {
            ti_medprice(
                high.len() as i32,
                inputs_c_medprice.as_ptr(),
                medprice_options.as_ptr(),
                medprice_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(
            ret, 0,
            "ti_medprice for SMA input returned error code {}",
            ret
        );

        // Now run SMA on the medprice values with period 5
        let sma_inputs_c: Vec<*const f64> = vec![c_medprice_values.as_ptr()];
        let short_sma_options = [5.0]; // 5-period SMA
        let sma_start_index = unsafe { ti_sma_start(short_sma_options.as_ptr()) };
        let sma_output_len = c_medprice_values.len() - (sma_start_index as usize);

        let mut c_short_sma_output = vec![0.0_f64; sma_output_len];
        let mut sma_outputs_c: Vec<*mut f64> = vec![c_short_sma_output.as_mut_ptr()];
        let ret = unsafe {
            ti_sma(
                c_medprice_values.len() as i32,
                sma_inputs_c.as_ptr(),
                short_sma_options.as_ptr(),
                sma_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(
            ret, 0,
            "ti_sma for short period returned error code {}",
            ret
        );

        // Compare short SMA outputs from the end backwards for better alignment
        let compare_len = rust_short_sma.len().min(c_short_sma_output.len());
        for i in 0..compare_len {
            let rust_idx = rust_short_sma.len() - 1 - i;
            let c_idx = c_short_sma_output.len() - 1 - i;
            let rust_val = rust_short_sma[rust_idx];
            let c_val = c_short_sma_output[c_idx];

            if rust_val.is_nan() {
                panic!(
                    "Rust short SMA has NaN at index {} (from end): Rust = {}",
                    i, rust_val
                );
            }
            if rust_val.is_infinite() {
                panic!(
                    "Rust short SMA has infinity at index {} (from end): Rust = {}",
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
                "Short SMA mismatch at index {} (from end): C = {}, Rust = {}",
                i,
                c_val,
                rust_val
            );
        }

        println!("✓ Short SMA optional output validated");
        println!("✓ All AO short SMA optional output tests passed!");
    }

    #[test]
    fn test_ao_long_sma_optional_output_vs_c_tulip() {
        // Test the long SMA optional output against C Tulip SMA of medprice
        let (high, low) = expand_inputs();

        println!("Testing AO long SMA optional output");

        // Get Rust AO with long SMA optional output enabled
        let inputs_rust = [high.as_slice(), low.as_slice()];
        let (rust_outputs, _) = indicator(&inputs_rust, &OPTIONS, Some(&[false, true, false]))
            .expect("Rust AO indicator with long SMA optional output failed");

        let rust_long_sma = &rust_outputs[2]; // long_sma is at index 2

        // First, get medprice values from C Tulip to feed into SMA
        let inputs_c_medprice: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];
        let medprice_options = [];
        let medprice_start_index = unsafe { ti_medprice_start(medprice_options.as_ptr()) };
        let medprice_output_len = high.len() - (medprice_start_index as usize);

        let mut c_medprice_values = vec![0.0_f64; medprice_output_len];
        let mut medprice_outputs_c: Vec<*mut f64> = vec![c_medprice_values.as_mut_ptr()];
        let ret = unsafe {
            ti_medprice(
                high.len() as i32,
                inputs_c_medprice.as_ptr(),
                medprice_options.as_ptr(),
                medprice_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(
            ret, 0,
            "ti_medprice for long SMA input returned error code {}",
            ret
        );

        // Now run SMA on the medprice values with period 34
        let sma_inputs_c: Vec<*const f64> = vec![c_medprice_values.as_ptr()];
        let long_sma_options = [34.0]; // 34-period SMA
        let sma_start_index = unsafe { ti_sma_start(long_sma_options.as_ptr()) };
        let sma_output_len = c_medprice_values.len() - (sma_start_index as usize);

        let mut c_long_sma_output = vec![0.0_f64; sma_output_len];
        let mut sma_outputs_c: Vec<*mut f64> = vec![c_long_sma_output.as_mut_ptr()];
        let ret = unsafe {
            ti_sma(
                c_medprice_values.len() as i32,
                sma_inputs_c.as_ptr(),
                long_sma_options.as_ptr(),
                sma_outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_sma for long period returned error code {}", ret);

        // Compare long SMA outputs from the end backwards for better alignment
        let compare_len = rust_long_sma.len().min(c_long_sma_output.len());
        for i in 0..compare_len {
            let rust_idx = rust_long_sma.len() - 1 - i;
            let c_idx = c_long_sma_output.len() - 1 - i;
            let rust_val = rust_long_sma[rust_idx];
            let c_val = c_long_sma_output[c_idx];

            if rust_val.is_nan() {
                panic!(
                    "Rust long SMA has NaN at index {} (from end): Rust = {}",
                    i, rust_val
                );
            }
            if rust_val.is_infinite() {
                panic!(
                    "Rust long SMA has infinity at index {} (from end): Rust = {}",
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
                "Long SMA mismatch at index {} (from end): C = {}, Rust = {}",
                i,
                c_val,
                rust_val
            );
        }

        println!("✓ Long SMA optional output validated");
        println!("✓ All AO long SMA optional output tests passed!");
    }

    #[test]
    fn test_ao_medprice_optional_output_vs_c_tulip() {
        // Test the medprice optional output against C Tulip medprice
        let (high, low) = expand_inputs();

        println!("Testing AO medprice optional output");

        // Get Rust AO with medprice optional output enabled
        let inputs_rust = [high.as_slice(), low.as_slice()];
        let (rust_outputs, _) = indicator(&inputs_rust, &OPTIONS, Some(&[false, false, true]))
            .expect("Rust AO indicator with medprice optional output failed");

        let rust_medprice = &rust_outputs[3]; // medprice is at index 3

        // Run C Tulip medprice for comparison
        let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];
        let medprice_options = [];
        let start_index = unsafe { ti_medprice_start(medprice_options.as_ptr()) };
        let output_len = high.len() - (start_index as usize);

        let mut c_medprice_output = vec![0.0_f64; output_len];
        let mut outputs_c: Vec<*mut f64> = vec![c_medprice_output.as_mut_ptr()];
        let ret = unsafe {
            ti_medprice(
                high.len() as i32,
                inputs_c.as_ptr(),
                medprice_options.as_ptr(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_medprice returned error code {}", ret);

        // Compare medprice outputs from the end backwards for better alignment
        let compare_len = rust_medprice.len().min(c_medprice_output.len());
        for i in 0..compare_len {
            let rust_idx = rust_medprice.len() - 1 - i;
            let c_idx = c_medprice_output.len() - 1 - i;
            let rust_val = rust_medprice[rust_idx];
            let c_val = c_medprice_output[c_idx];

            if rust_val.is_nan() {
                panic!(
                    "Rust medprice has NaN at index {} (from end): Rust = {}",
                    i, rust_val
                );
            }
            if rust_val.is_infinite() {
                panic!(
                    "Rust medprice has infinity at index {} (from end): Rust = {}",
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
                "Medprice mismatch at index {} (from end): C = {}, Rust = {}",
                i,
                c_val,
                rust_val
            );
        }

        println!("✓ Medprice optional output validated");
        println!("✓ All AO medprice optional output tests passed!");
    }

    #[test]
    fn test_ao_short_sma_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);

            println!(
                "Testing AO short SMA optional output with database stock {}",
                stock_symbol
            );

            // Get Rust AO with short SMA optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &[], Some(&[true, false, false]))
                .expect("Rust AO indicator with short SMA optional output failed");

            let rust_short_sma = &rust_outputs[1]; // short_sma is at index 1

            if rust_short_sma.is_empty() {
                panic!(
                    "Rust short SMA optional output is empty for stock {}",
                    stock_symbol
                );
            }

            // Get C Tulip medprice first
            let medprice_inputs_c = [high.as_ptr(), low.as_ptr()];
            let medprice_start_index = unsafe { ti_medprice_start(std::ptr::null()) };
            let medprice_output_len = high.len() - (medprice_start_index as usize);
            let mut c_medprice = vec![0.0; medprice_output_len];
            let mut medprice_outputs_c = vec![c_medprice.as_mut_ptr()];

            let ret = unsafe {
                ti_medprice(
                    high.len() as i32,
                    medprice_inputs_c.as_ptr(),
                    std::ptr::null(),
                    medprice_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(
                ret, 0,
                "ti_medprice returned error code {} for stock {}",
                ret, stock_symbol
            );

            // Now calculate SMA on medprice
            let sma_inputs_c = [c_medprice.as_ptr()];
            let sma_options = [5.0]; // short period
            let sma_start_index = unsafe { ti_sma_start(sma_options.as_ptr()) };
            let sma_output_len = c_medprice.len() - (sma_start_index as usize);
            let mut c_short_sma = vec![0.0; sma_output_len];
            let mut sma_outputs_c = vec![c_short_sma.as_mut_ptr()];

            let ret = unsafe {
                ti_sma(
                    c_medprice.len() as i32,
                    sma_inputs_c.as_ptr(),
                    sma_options.as_ptr(),
                    sma_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(
                ret, 0,
                "ti_sma returned error code {} for stock {}",
                ret, stock_symbol
            );

            // Compare from the end backwards
            let compare_len = rust_short_sma.len().min(c_short_sma.len());
            for i in 0..compare_len {
                let rust_idx = rust_short_sma.len() - 1 - i;
                let c_idx = c_short_sma.len() - 1 - i;
                let rust_val = rust_short_sma[rust_idx];
                let c_val = c_short_sma[c_idx];

                if !rust_val.is_finite() {
                    panic!(
                        "Rust short SMA output has NaN/infinity at index {} (from end): Rust = {} for stock {}",
                        i, rust_val, stock_symbol
                    );
                }

                if !c_val.is_finite() {
                    continue; // Skip C library bugs
                }

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = SMA_EPSILON),
                    "Short SMA mismatch at index {} (from end): C = {}, Rust = {} for stock {}",
                    i,
                    c_val,
                    rust_val,
                    stock_symbol
                );
            }

            println!(
                "✓ Short SMA optional output validated for stock {}",
                stock_symbol
            );
        }

        println!("✓ All AO short SMA optional output database tests passed!");
    }

    #[test]
    fn test_ao_long_sma_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);

            println!(
                "Testing AO long SMA optional output with database stock {}",
                stock_symbol
            );

            // Get Rust AO with long SMA optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &[], Some(&[false, true, false]))
                .expect("Rust AO indicator with long SMA optional output failed");

            let rust_long_sma = &rust_outputs[2]; // long_sma is at index 2

            if rust_long_sma.is_empty() {
                panic!(
                    "Rust long SMA optional output is empty for stock {}",
                    stock_symbol
                );
            }

            // Get C Tulip medprice first
            let medprice_inputs_c = [high.as_ptr(), low.as_ptr()];
            let medprice_start_index = unsafe { ti_medprice_start(std::ptr::null()) };
            let medprice_output_len = high.len() - (medprice_start_index as usize);
            let mut c_medprice = vec![0.0; medprice_output_len];
            let mut medprice_outputs_c = vec![c_medprice.as_mut_ptr()];

            let ret = unsafe {
                ti_medprice(
                    high.len() as i32,
                    medprice_inputs_c.as_ptr(),
                    std::ptr::null(),
                    medprice_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(
                ret, 0,
                "ti_medprice returned error code {} for stock {}",
                ret, stock_symbol
            );

            // Now calculate SMA on medprice
            let sma_inputs_c = [c_medprice.as_ptr()];
            let sma_options = [34.0]; // long period
            let sma_start_index = unsafe { ti_sma_start(sma_options.as_ptr()) };
            let sma_output_len = c_medprice.len() - (sma_start_index as usize);
            let mut c_long_sma = vec![0.0; sma_output_len];
            let mut sma_outputs_c = vec![c_long_sma.as_mut_ptr()];

            let ret = unsafe {
                ti_sma(
                    c_medprice.len() as i32,
                    sma_inputs_c.as_ptr(),
                    sma_options.as_ptr(),
                    sma_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(
                ret, 0,
                "ti_sma returned error code {} for stock {}",
                ret, stock_symbol
            );

            // Compare from the end backwards
            let compare_len = rust_long_sma.len().min(c_long_sma.len());
            for i in 0..compare_len {
                let rust_idx = rust_long_sma.len() - 1 - i;
                let c_idx = c_long_sma.len() - 1 - i;
                let rust_val = rust_long_sma[rust_idx];
                let c_val = c_long_sma[c_idx];

                if !rust_val.is_finite() {
                    panic!(
                        "Rust long SMA output has NaN/infinity at index {} (from end): Rust = {} for stock {}",
                        i, rust_val, stock_symbol
                    );
                }

                if !c_val.is_finite() {
                    continue; // Skip C library bugs
                }

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = SMA_EPSILON),
                    "Long SMA mismatch at index {} (from end): C = {}, Rust = {} for stock {}",
                    i,
                    c_val,
                    rust_val,
                    stock_symbol
                );
            }

            println!(
                "✓ Long SMA optional output validated for stock {}",
                stock_symbol
            );
        }

        println!("✓ All AO long SMA optional output database tests passed!");
    }

    #[test]
    fn test_ao_medprice_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);

            println!(
                "Testing AO medprice optional output with database stock {}",
                stock_symbol
            );

            // Get Rust AO with medprice optional output enabled
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &[], Some(&[false, false, true]))
                .expect("Rust AO indicator with medprice optional output failed");

            let rust_medprice = &rust_outputs[3]; // medprice is at index 3

            if rust_medprice.is_empty() {
                panic!(
                    "Rust medprice optional output is empty for stock {}",
                    stock_symbol
                );
            }

            // Get C Tulip medprice output for comparison
            let medprice_inputs_c = [high.as_ptr(), low.as_ptr()];
            let medprice_start_index = unsafe { ti_medprice_start(std::ptr::null()) };
            let medprice_output_len = high.len() - (medprice_start_index as usize);
            let mut c_medprice = vec![0.0; medprice_output_len];
            let mut medprice_outputs_c = vec![c_medprice.as_mut_ptr()];

            let ret = unsafe {
                ti_medprice(
                    high.len() as i32,
                    medprice_inputs_c.as_ptr(),
                    std::ptr::null(),
                    medprice_outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(
                ret, 0,
                "ti_medprice returned error code {} for stock {}",
                ret, stock_symbol
            );

            // Compare from the end backwards
            let compare_len = rust_medprice.len().min(c_medprice.len());
            for i in 0..compare_len {
                let rust_idx = rust_medprice.len() - 1 - i;
                let c_idx = c_medprice.len() - 1 - i;
                let rust_val = rust_medprice[rust_idx];
                let c_val = c_medprice[c_idx];

                if !rust_val.is_finite() {
                    panic!(
                        "Rust medprice output has NaN/infinity at index {} (from end): Rust = {} for stock {}",
                        i, rust_val, stock_symbol
                    );
                }

                if !c_val.is_finite() {
                    continue; // Skip C library bugs
                }

                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = MEDPRICE_EPSILON),
                    "Medprice mismatch at index {} (from end): C = {}, Rust = {} for stock {}",
                    i,
                    c_val,
                    rust_val,
                    stock_symbol
                );
            }

            println!(
                "✓ Medprice optional output validated for stock {}",
                stock_symbol
            );
        }

        println!("✓ All AO medprice optional output database tests passed!");
    }

    #[test]
    fn test_ao_simd_vs_regular_database() {
        use tulip_rs::indicators::ao::indicator_by_assets;

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

        // Test without optional outputs
        {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &OPTIONS, None)
                .expect("SIMD by assets AO indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, high, low)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock
                let stock_inputs = [high.as_slice(), low.as_slice()];
                let (regular_results, _) =
                    indicator(&stock_inputs, &OPTIONS, None).expect("Regular AO indicator failed");

                let simd_result = &simd_results[stock_idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "AO output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
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
                            "SIMD by assets AO has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, OPTIONS, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets AO has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, OPTIONS, simd_val
                        );
                    }

                    // Compare values with epsilon tolerance for AO
                    if (simd_val - regular_val).abs() > EPSILON {
                        println!(
                            "SIMD: {:?}\n\nRegular: {:?}",
                            &simd_result[..20.min(simd_result.len())],
                            &regular_result[..20.min(regular_result.len())]
                        );
                        panic!(
                            "AO mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
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

        println!("✓ All SIMD by assets vs Regular AO database tests passed!");
    }

    #[test]
    fn test_ao_simd_vs_regular_database_optional_outputs() {
        use tulip_rs::indicators::ao::indicator_by_assets;

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

        // Test with optional outputs enabled (short SMA, long SMA, medprice)
        {
            // Get SIMD by assets result with optional outputs
            let (simd_results, _) =
                indicator_by_assets::<4>(&inputs, &OPTIONS, Some(&[true, true, true]))
                    .expect("SIMD by assets AO indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, high, low)) in stock_data.iter().enumerate() {
                // Get regular indicator result for this stock with optional outputs
                let stock_inputs = [high.as_slice(), low.as_slice()];
                let (regular_results, _) =
                    indicator(&stock_inputs, &OPTIONS, Some(&[true, true, true]))
                        .expect("Regular AO indicator failed");

                let simd_ao_result = &simd_results[stock_idx][0];
                let simd_short_sma_result = &simd_results[stock_idx][1];
                let simd_long_sma_result = &simd_results[stock_idx][2];
                let simd_medprice_result = &simd_results[stock_idx][3];

                let regular_ao_result = &regular_results[0];
                let regular_short_sma_result = &regular_results[1];
                let regular_long_sma_result = &regular_results[2];
                let regular_medprice_result = &regular_results[3];

                // Compare AO output lengths
                assert_eq!(
                    simd_ao_result.len(),
                    regular_ao_result.len(),
                    "AO output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    OPTIONS,
                    simd_ao_result.len(),
                    regular_ao_result.len()
                );

                // Compare Short SMA output lengths
                assert_eq!(
                    simd_short_sma_result.len(),
                    regular_short_sma_result.len(),
                    "Short SMA output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    OPTIONS,
                    simd_short_sma_result.len(),
                    regular_short_sma_result.len()
                );

                // Compare Long SMA output lengths
                assert_eq!(
                    simd_long_sma_result.len(),
                    regular_long_sma_result.len(),
                    "Long SMA output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    OPTIONS,
                    simd_long_sma_result.len(),
                    regular_long_sma_result.len()
                );

                // Compare Medprice output lengths
                assert_eq!(
                    simd_medprice_result.len(),
                    regular_medprice_result.len(),
                    "Medprice output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    OPTIONS,
                    simd_medprice_result.len(),
                    regular_medprice_result.len()
                );

                // Compare AO values
                for (i, (&simd_val, &regular_val)) in simd_ao_result
                    .iter()
                    .zip(regular_ao_result.iter())
                    .enumerate()
                {
                    if (simd_val - regular_val).abs() > EPSILON {
                        panic!(
                            "AO mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, OPTIONS, simd_val, regular_val
                        );
                    }
                }

                // Compare Short SMA values
                for (i, (&simd_val, &regular_val)) in simd_short_sma_result
                    .iter()
                    .zip(regular_short_sma_result.iter())
                    .enumerate()
                {
                    if (simd_val - regular_val).abs() > SMA_EPSILON {
                        panic!(
                            "Short SMA mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, OPTIONS, simd_val, regular_val
                        );
                    }
                }

                // Compare Long SMA values
                for (i, (&simd_val, &regular_val)) in simd_long_sma_result
                    .iter()
                    .zip(regular_long_sma_result.iter())
                    .enumerate()
                {
                    if (simd_val - regular_val).abs() > SMA_EPSILON {
                        panic!(
                            "Long SMA mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, OPTIONS, simd_val, regular_val
                        );
                    }
                }

                // Compare Medprice values
                for (i, (&simd_val, &regular_val)) in simd_medprice_result
                    .iter()
                    .zip(regular_medprice_result.iter())
                    .enumerate()
                {
                    if (simd_val - regular_val).abs() > MEDPRICE_EPSILON {
                        panic!(
                            "Medprice mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, OPTIONS, simd_val, regular_val
                        );
                    }
                }

                println!(
                    "✓ SIMD by assets vs Regular optional outputs test passed for stock {} with options {:?}",
                    stock_symbol, OPTIONS
                );
            }
        }

        println!("✓ All SIMD by assets vs Regular AO optional outputs database tests passed!");
    }

    //add test code here
}
