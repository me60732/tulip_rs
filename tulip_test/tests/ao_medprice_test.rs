#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::ao_medprice::{indicator, min_data, TIndicatorState};
    use tulip_rs::indicators::medprice::indicator as medprice_indicator;
    use tulip_test::c_bindings::{ti_ao, ti_ao_start, ti_sma, ti_sma_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const EPSILON: f64 = 1e-12;
    const SMA_EPSILON: f64 = 1e-10; // Use epsilon from sma_test.rs
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
    fn test_ao_medprice_indicator() {
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
        let (outputs, _) = medprice_indicator(&inputs_rust, &OPTIONS, None)
            .expect("Rust Medprice indicator failed");
        let inputs_rust = [outputs[0].as_slice()];
        let (outputs, _) =
            indicator(&inputs_rust, &OPTIONS, None).expect("Rust AO indicator failed");
        // Compare the outputs
        // Compare the outputs in reverse for the length of the Rust outputs
        //println!("\nC output: {:?}", output_vec_c);
        //println!("\nRust output: {:?}", result_rust.indicators[0]);
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
                    "Rust AO_MEDPRICE has infinity at index {}: Rust = {}",
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
    fn test_ao_medprice_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);

            // C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            let start_index = unsafe { ti_ao_start(OPTIONS.as_ptr()) };
            assert!(start_index >= 0, "ti_ao_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            let mut output_vec_c = vec![0.0_f64; output_len_c];
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

            // Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (outputs, _) = medprice_indicator(&inputs_rust, &OPTIONS, None)
                .expect("Rust Medprice indicator failed");
            let inputs_rust = [outputs[0].as_slice()];
            let (outputs, _) =
                indicator(&inputs_rust, &OPTIONS, None).expect("Rust AO indicator failed");

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
                        "Rust AO has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                        index, rust_val, OPTIONS, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust AO_MEDPRICE has infinity at index {}: Rust = {}",
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
                        "Mismatch at index {}: C = {}, Rust = {}, Stock: {}",
                        index, c_val, rust_val, stock_symbol
                    );
                }
            }
        }
    }

    #[test]
    fn test_ao_medprice_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice()];

            // Calculate medprice first
            let medprice_vec = medprice_indicator(&inputs_rust, &[], None)
                .expect("Medprice Indicator Failed")
                .0[0]
                .clone();
            let medprice_inputs = [medprice_vec.as_slice()];
            let options: [f64; 0] = [];

            // Get full output
            let (full_outputs, _) = indicator(&medprice_inputs, &options, None)
                .expect("Failed to run AO_MEDPRICE indicator on full data");

            // Process in batches
            let mut batch_full_output = Vec::new();

            let min_data_val = min_data(&options).max(CHUNK_SIZE);

            if medprice_vec.len() <= min_data_val {
                // If data is too small, just run full calculation
                let (outputs, _) = indicator(&medprice_inputs, &options, None)
                    .expect("Failed to run AO_MEDPRICE indicator");
                batch_full_output.extend_from_slice(&outputs[0]);
            } else {
                // First chunk - convert to Vec<&Vec<f64>>
                let medprice_chunk_vec = medprice_vec[..min_data_val].to_vec();
                let chunk_inputs = [medprice_chunk_vec.as_slice()];

                let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                    .expect("Failed to run AO_MEDPRICE indicator on first chunk");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks using state
                let mut medprice_chunks = medprice_vec[min_data_val..].chunks_exact(CHUNK_SIZE);

                for medprice_chunk in medprice_chunks.by_ref() {
                    let medprice_chunk_vec = medprice_chunk.to_vec();
                    let chunk_inputs = [medprice_chunk_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("AO_MEDPRICE batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Process remainder if any
                let medprice_rem = medprice_chunks.remainder();

                if !medprice_rem.is_empty() {
                    let medprice_rem_vec = medprice_rem.to_vec();
                    let chunk_inputs = [medprice_rem_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("AO_MEDPRICE batch indicator failed");
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

    fn get_hl_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        (high, low)
    }

    #[test]
    fn test_ao_medprice_short_sma_optional_output_vs_c_tulip() {
        // Test the short SMA optional output against C Tulip SMA of medprice
        let (high, low) = expand_inputs();

        println!("Testing AO_MEDPRICE short SMA optional output");

        // First calculate medprice from high/low
        let inputs_rust = [high.as_slice(), low.as_slice()];
        let (medprice_outputs, _) = medprice_indicator(&inputs_rust, &OPTIONS, None)
            .expect("Rust Medprice indicator failed");
        let medprice_vec = &medprice_outputs[0];

        // Get Rust AO_MEDPRICE with short SMA optional output enabled
        let medprice_inputs = [medprice_vec.as_slice()];
        let (rust_outputs, _) = indicator(&medprice_inputs, &OPTIONS, Some(&[true, false]))
            .expect("Rust AO_MEDPRICE indicator with short SMA optional output failed");

        let rust_short_sma = &rust_outputs[1]; // short_sma is at index 1

        // Run C Tulip SMA on the medprice values with period 5
        let sma_inputs_c: Vec<*const f64> = vec![medprice_vec.as_ptr()];
        let short_sma_options = [5.0]; // 5-period SMA
        let sma_start_index = unsafe { ti_sma_start(short_sma_options.as_ptr()) };
        let sma_output_len = medprice_vec.len() - (sma_start_index as usize);

        let mut c_short_sma_output = vec![0.0_f64; sma_output_len];
        let mut sma_outputs_c: Vec<*mut f64> = vec![c_short_sma_output.as_mut_ptr()];
        let ret = unsafe {
            ti_sma(
                medprice_vec.len() as i32,
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
        println!("✓ All AO_MEDPRICE short SMA optional output tests passed!");
    }

    #[test]
    fn test_ao_medprice_long_sma_optional_output_vs_c_tulip() {
        // Test the long SMA optional output against C Tulip SMA of medprice
        let (high, low) = expand_inputs();

        println!("Testing AO_MEDPRICE long SMA optional output");

        // First calculate medprice from high/low
        let inputs_rust = [high.as_slice(), low.as_slice()];
        let (medprice_outputs, _) = medprice_indicator(&inputs_rust, &OPTIONS, None)
            .expect("Rust Medprice indicator failed");
        let medprice_vec = &medprice_outputs[0];

        // Get Rust AO_MEDPRICE with long SMA optional output enabled
        let medprice_inputs = [medprice_vec.as_slice()];
        let (rust_outputs, _) = indicator(&medprice_inputs, &OPTIONS, Some(&[false, true]))
            .expect("Rust AO_MEDPRICE indicator with long SMA optional output failed");

        let rust_long_sma = &rust_outputs[2]; // long_sma is at index 2

        // Run C Tulip SMA on the medprice values with period 34
        let sma_inputs_c: Vec<*const f64> = vec![medprice_vec.as_ptr()];
        let long_sma_options = [34.0]; // 34-period SMA
        let sma_start_index = unsafe { ti_sma_start(long_sma_options.as_ptr()) };
        let sma_output_len = medprice_vec.len() - (sma_start_index as usize);

        let mut c_long_sma_output = vec![0.0_f64; sma_output_len];
        let mut sma_outputs_c: Vec<*mut f64> = vec![c_long_sma_output.as_mut_ptr()];
        let ret = unsafe {
            ti_sma(
                medprice_vec.len() as i32,
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
        println!("✓ All AO_MEDPRICE long SMA optional output tests passed!");
    }

    #[test]
    fn test_ao_medprice_short_sma_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);

            println!(
                "Testing AO_MEDPRICE short SMA optional output with database stock {}",
                stock_symbol
            );

            // First calculate medprice from H/L
            let medprice_inputs = [high.as_slice(), low.as_slice()];
            let (medprice_outputs, _) = medprice_indicator(&medprice_inputs, &[], None)
                .expect("Medprice calculation failed");
            let medprice_data = &medprice_outputs[0];

            // Get Rust AO_MEDPRICE with short SMA optional output enabled
            let inputs_rust = [medprice_data.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &[], Some(&[true, false]))
                .expect("Rust AO_MEDPRICE indicator with short SMA optional output failed");

            let rust_short_sma = &rust_outputs[1]; // short_sma is at index 1

            if rust_short_sma.is_empty() {
                panic!(
                    "Rust short SMA optional output is empty for stock {}",
                    stock_symbol
                );
            }

            // Get C Tulip SMA output on medprice
            let sma_inputs_c = vec![medprice_data.as_ptr()];
            let sma_options = [5.0]; // short period
            let sma_start_index = unsafe { ti_sma_start(sma_options.as_ptr()) };
            let sma_output_len = medprice_data.len() - (sma_start_index as usize);
            let mut c_short_sma = vec![0.0; sma_output_len];
            let mut sma_outputs_c = vec![c_short_sma.as_mut_ptr()];

            let ret = unsafe {
                ti_sma(
                    medprice_data.len() as i32,
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

        println!("✓ All AO_MEDPRICE short SMA optional output database tests passed!");
    }

    #[test]
    fn test_ao_medprice_long_sma_optional_output_vs_c_tulip_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(&stock_data);

            println!(
                "Testing AO_MEDPRICE long SMA optional output with database stock {}",
                stock_symbol
            );

            // First calculate medprice from H/L
            let medprice_inputs = [high.as_slice(), low.as_slice()];
            let (medprice_outputs, _) = medprice_indicator(&medprice_inputs, &[], None)
                .expect("Medprice calculation failed");
            let medprice_data = &medprice_outputs[0];

            // Get Rust AO_MEDPRICE with long SMA optional output enabled
            let inputs_rust = [medprice_data.as_slice()];
            let (rust_outputs, _) = indicator(&inputs_rust, &[], Some(&[false, true]))
                .expect("Rust AO_MEDPRICE indicator with long SMA optional output failed");

            let rust_long_sma = &rust_outputs[2]; // long_sma is at index 2

            if rust_long_sma.is_empty() {
                panic!(
                    "Rust long SMA optional output is empty for stock {}",
                    stock_symbol
                );
            }

            // Get C Tulip SMA output on medprice
            let sma_inputs_c = vec![medprice_data.as_ptr()];
            let sma_options = [34.0]; // long period
            let sma_start_index = unsafe { ti_sma_start(sma_options.as_ptr()) };
            let sma_output_len = medprice_data.len() - (sma_start_index as usize);
            let mut c_long_sma = vec![0.0; sma_output_len];
            let mut sma_outputs_c = vec![c_long_sma.as_mut_ptr()];

            let ret = unsafe {
                ti_sma(
                    medprice_data.len() as i32,
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

        println!("✓ All AO_MEDPRICE long SMA optional output database tests passed!");
    }
}
