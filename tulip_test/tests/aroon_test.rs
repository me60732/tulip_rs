#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::aroon::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_aroon, ti_aroon_start};
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

    //const OPTIONS_LIST: [[f64; 1]; 6] = [[5.0], [14.0], [25.0], [35.0], [50.0], [100.0]]; // reckon there is an issue with the remainder in SIMD
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
        for _ in 0..15 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
        }
        (high_vec, low_vec)
    }

    #[test]
    fn test_aroon_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

            // Determine the offset required by the C AROON function
            let start_index = unsafe { ti_aroon_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_aroon_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut aroon_down_output_vec_c = vec![0.0_f64; output_len_c];
            let mut aroon_up_output_vec_c = vec![0.0_f64; output_len_c];
            let aroon_down_ptr: *mut f64 = aroon_down_output_vec_c.as_mut_ptr();
            let aroon_up_ptr: *mut f64 = aroon_up_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![aroon_down_ptr, aroon_up_ptr];
            let ret = unsafe {
                ti_aroon(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_aroon returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice()];
            let (outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Rust AROON indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the AROON_DOWN outputs in reverse
            for (i, (&c_val, &rust_val)) in aroon_down_output_vec_c
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
                        "Rust AROON_DOWN has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust AROON has infinity at index {}: Rust = {}",
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
                    approx_eq!(f64, c_val, rust_val, epsilon = 1e-12),
                    "Mismatch in AROON_DOWN at index {}: C = {}, Rust = {} for options {:?}, \nC AROON_DOWN results: \n{:?}, \nRust AROON_DOWN results: \n{:?}",
                    index,
                    c_val,
                    rust_val,
                    options,
                    aroon_down_output_vec_c,
                    outputs[0]
                );
            }

            // Compare the AROON_UP outputs in reverse
            for (i, (&c_val, &rust_val)) in aroon_up_output_vec_c
                .iter()
                .rev()
                .take(output_len_rust)
                .zip(outputs[1].iter().rev())
                .enumerate()
            {
                let index = output_len_rust - i - 1;

                // Fail test if Rust has NaN
                if rust_val.is_nan() {
                    panic!(
                        "Rust AROON_UP has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust AROON has infinity at index {}: Rust = {}",
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
                    approx_eq!(f64, c_val, rust_val, epsilon = 1e-12),
                    "Mismatch in AROON_UP at index {}: C = {}, Rust = {} for options {:?}, \nC AROON_UP results: \n{:?}, \nRust AROON_UP results: \n{:?}",
                    index,
                    c_val,
                    rust_val,
                    options,
                    aroon_up_output_vec_c,
                    outputs[1]
                );
            }
        }
    }

    #[test]
    fn test_aroon_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            /*if stock_symbol == "BHP_ASX" {
                println!("const HIGH: [f64; 6705] = {:?}\n\nconst LOW: [f64; 6705] = {:?}", high, low);
            }*/
            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr()];

                let start_index = unsafe { ti_aroon_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_aroon_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut aroon_down_output_vec_c = vec![0.0_f64; output_len_c];
                let mut aroon_up_output_vec_c = vec![0.0_f64; output_len_c];
                let aroon_down_ptr: *mut f64 = aroon_down_output_vec_c.as_mut_ptr();
                let aroon_up_ptr: *mut f64 = aroon_up_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![aroon_down_ptr, aroon_up_ptr];
                let ret = unsafe {
                    ti_aroon(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_aroon returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice()];
                let (outputs, _) =
                    indicator(&inputs_rust, &options, None).expect("Rust AROON indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare AROON_DOWN results
                for (i, (&c_val, &rust_val)) in aroon_down_output_vec_c
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
                            "Rust AROON_DOWN has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust AROON has infinity at index {}: Rust = {}",
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
                            "AROON_DOWN test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, aroon_down_output_vec_c, outputs[0], options, stock_symbol
                        );
                        panic!(
                            "AROON_DOWN mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }

                // Compare AROON_UP results
                for (i, (&c_val, &rust_val)) in aroon_up_output_vec_c
                    .iter()
                    .rev()
                    .take(output_len_rust)
                    .zip(outputs[1].iter().rev())
                    .enumerate()
                {
                    let index = output_len_rust - i - 1;

                    // Fail test if Rust has NaN
                    if rust_val.is_nan() {
                        panic!(
                            "Rust AROON_UP has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust AROON has infinity at index {}: Rust = {}",
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
                            "AROON_UP test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, aroon_up_output_vec_c, outputs[1], options, stock_symbol
                        );
                        panic!(
                            "AROON_UP mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_aroon_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice()];

            for options in OPTIONS_LIST {
                // Get full output
                let (full_outputs, _) = indicator(&inputs_rust, &options, None)
                    .expect("Failed to run AROON indicator on full data");

                // Process in batches
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) = indicator(&inputs_rust, &options, None)
                        .expect("Failed to run AROON indicator");
                    for output_idx in 0..outputs.len() {
                        batch_full_outputs[output_idx].extend_from_slice(&outputs[output_idx]);
                    }
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let high_vec = high[..min_data_val].to_vec();
                    let low_vec = low[..min_data_val].to_vec();
                    let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];

                    let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("Failed to run AROON indicator on first chunk");
                    for output_idx in 0..first_outputs.len() {
                        batch_full_outputs[output_idx]
                            .extend_from_slice(&first_outputs[output_idx]);
                    }

                    // Process remaining data in chunks using state
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for (high_chunk, low_chunk) in high_chunks.by_ref().zip(low_chunks.by_ref()) {
                        let high_vec = high_chunk.to_vec();
                        let low_vec = low_chunk.to_vec();
                        let chunk_inputs = [high_vec.as_slice(), low_vec.as_slice()];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("AROON batch indicator failed");
                        for output_idx in 0..chunk_outputs.len() {
                            batch_full_outputs[output_idx]
                                .extend_from_slice(&chunk_outputs[output_idx]);
                        }
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
                            .expect("AROON batch indicator failed");
                        for output_idx in 0..chunk_outputs.len() {
                            batch_full_outputs[output_idx]
                                .extend_from_slice(&chunk_outputs[output_idx]);
                        }
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
                        assert_eq!(
                            full_val, batch_val,
                            "Output {} mismatch at index {} for stock {} with options {:?}: full={}, batch={}",
                            output_idx, i, stock_symbol, options, full_val, batch_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_aroon_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::aroon::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' high and low data
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
            &[&stock_data[0].1, &stock_data[0].2],
            &[&stock_data[1].1, &stock_data[1].2],
            &[&stock_data[2].1, &stock_data[2].2],
            &[&stock_data[3].1, &stock_data[3].2],
        ];

        for options in OPTIONS_LIST {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets AROON indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_high, stock_low)) in stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [stock_high.as_slice(), stock_low.as_slice()];
                let (regular_results, _) = indicator(&stock_inputs, &options, None)
                    .expect("Regular AROON indicator failed");

                let simd_aroon_down = &simd_results[stock_idx][0];
                let simd_aroon_up = &simd_results[stock_idx][1];
                let regular_aroon_down = &regular_results[0];
                let regular_aroon_up = &regular_results[1];

                // Compare output lengths
                assert_eq!(
                    simd_aroon_down.len(),
                    regular_aroon_down.len(),
                    "Aroon Down output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_aroon_down.len(),
                    regular_aroon_down.len()
                );

                assert_eq!(
                    simd_aroon_up.len(),
                    regular_aroon_up.len(),
                    "Aroon Up output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_aroon_up.len(),
                    regular_aroon_up.len()
                );

                // Compare Aroon Down values
                for (i, (&simd_val, &regular_val)) in simd_aroon_down
                    .iter()
                    .zip(regular_aroon_down.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by assets AROON Down has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets AROON Down has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with high precision
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        /*let start = if i < 10 { 0 } else { i - 10 };
                        println!(
                            "simd aroon down results: {:?} \n\nRegular aroon down Results: {:?} \n\n",
                            &simd_aroon_down[start..(i+10).min(simd_aroon_down.len())],//[..10.min(simd_aroon_down.len())],
                            &regular_aroon_down[start..(i+10).min(simd_aroon_down.len())]
                        );*/
                        panic!(
                            "Aroon Down mismatch at index {} for stock {} options {:?}: SIMD by assets = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Aroon Up values
                for (i, (&simd_val, &regular_val)) in simd_aroon_up
                    .iter()
                    .zip(regular_aroon_up.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by assets AROON Up has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets AROON Up has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with high precision
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        let start = i.saturating_sub(10);
                        println!(
                            "simd aroon up results: {:?} \n\nRegular aroon up Results: {:?} \n\n",
                            &simd_aroon_up[start..(i + 10).min(simd_aroon_down.len())],
                            &regular_aroon_up[start..(i + 10).min(simd_aroon_down.len())]
                        );
                        panic!(
                            "Aroon Up mismatch at index {} for stock {} options {:?}: SIMD by assets = {}, Regular = {}",
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

        println!("✓ All SIMD by assets vs Regular AROON database tests passed!");
    }

    #[test]
    fn test_aroon_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::aroon::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low) = get_hl_arrays(stock_data);
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
                    .expect("SIMD AROON 4-wide first failed");

            // Process second 4 options with 4-wide SIMD
            let options_4_second = [
                &OPTIONS_LIST[4],
                &OPTIONS_LIST[5],
                &OPTIONS_LIST[6],
                &OPTIONS_LIST[7],
            ];
            let (simd_results_4_second, _) =
                indicator_by_options::<4>(&inputs, &options_4_second, None)
                    .expect("SIMD AROON 4-wide second failed");

            // Combine all SIMD results
            let mut all_simd_results = simd_results_4_first;
            all_simd_results.extend(simd_results_4_second);

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    indicator(&inputs, options, None).expect("Regular AROON indicator failed");

                let simd_result = &all_simd_results[idx];
                let regular_result = &regular_results;

                // Compare output lengths for both Aroon Down and Aroon Up
                assert_eq!(
                    simd_result[0].len(),
                    regular_result[0].len(),
                    "Aroon Down output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[0].len(),
                    regular_result[0].len()
                );

                assert_eq!(
                    simd_result[1].len(),
                    regular_result[1].len(),
                    "Aroon Up output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result[1].len(),
                    regular_result[1].len()
                );

                // Compare Aroon Down values
                for (i, (&simd_val, &regular_val)) in simd_result[0]
                    .iter()
                    .zip(regular_result[0].iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Aroon Down has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Aroon Down has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if (simd_val - regular_val).abs() > 1e-10 {
                        panic!(
                            "Aroon Down mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Aroon Up values
                for (i, (&simd_val, &regular_val)) in simd_result[1]
                    .iter()
                    .zip(regular_result[1].iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Aroon Up has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Aroon Up has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if (simd_val - regular_val).abs() > 1e-10 {
                        panic!(
                            "Aroon Up mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular AROON database tests passed!");
    }

    //add test code here
}
