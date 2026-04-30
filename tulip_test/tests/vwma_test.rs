#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::vwma::{indicator as rust_vwma, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_vwma, ti_vwma_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-10;
    const CHUNK_SIZE: usize = 100;
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
    fn expand_inputs() -> (Vec<f64>, Vec<f64>) {
        let mut close_vec = CLOSE.to_vec();
        let mut volume_vec = VOLUME.to_vec();
        for _ in 0..3 {
            close_vec.extend_from_slice(&CLOSE);
            volume_vec.extend_from_slice(&VOLUME);
        }
        (close_vec, volume_vec)
    }

    #[test]
    fn test_vwma_indicator() {
        // Use the same input data as in the benchmarks
        let (close, volume) = expand_inputs();

        for options in OPTIONS_LIST {
            // Prepare inputs for the C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr(), volume.as_ptr()];

            // Determine the offset required by the C VWMA function
            let start_index = unsafe { ti_vwma_start(options.as_ptr()) };
            assert!(start_index >= 0, "ti_vwma_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut vwma_output_vec_c = vec![0.0_f64; output_len_c];
            let vwma_ptr: *mut f64 = vwma_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![vwma_ptr];
            let ret = unsafe {
                ti_vwma(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    options.as_ptr(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_vwma returned error code {}", ret);

            // Run the Rust implementation
            let inputs_rust = [close.as_slice(), volume.as_slice()];
            let (outputs, _) =
                rust_vwma(&inputs_rust, &options, None).expect("Rust VWMA indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare the outputs in reverse for the length of the Rust outputs
            for (i, (&c_val, &rust_val)) in vwma_output_vec_c
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
                        "Rust VWMA has NaN at index {}: Rust = {}, Options = {:?}",
                        index, rust_val, options
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust VWMA has infinity at index {}: Rust = {}, Options = {:?}",
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
                    // Adjust epsilon if needed
                    println!(
                        "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                        index, vwma_output_vec_c, outputs[0], options
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
    fn test_vwma_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (close, volume) = get_close_volume_arrays(&stock_data);

            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![close.as_ptr(), volume.as_ptr()];

                let start_index = unsafe { ti_vwma_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_vwma_start returned a negative index");
                let output_len_c = close.len() - (start_index as usize);

                let mut output_vec_c = vec![0.0_f64; output_len_c];
                let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
                let ret = unsafe {
                    ti_vwma(
                        close.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_vwma returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [close.as_slice(), volume.as_slice()];
                let (outputs, _) =
                    rust_vwma(&inputs_rust, &options, None).expect("Rust VWMA indicator failed");

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
                            "Rust VWMA has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust VWMA has infinity at index {}: Rust = {}, Options = {:?}",
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
    fn test_vwma_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (close, volume) = get_close_volume_arrays(&stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [close.as_slice(), volume.as_slice()];

                // Get full output from processing all data at once
                let (full_outputs, _) =
                    rust_vwma(&inputs_rust, &options, None).expect("Rust VWMA indicator failed");

                // Process data in batches and accumulate outputs
                let mut batch_full_output = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                // First chunk - convert to Vec<&Vec<f64>>
                let close_vec = close[..min_data_val].to_vec();
                let volume_vec = volume[..min_data_val].to_vec();
                let chunk_inputs = [close_vec.as_slice(), volume_vec.as_slice()];

                let (first_outputs, mut state) =
                    rust_vwma(&chunk_inputs, &options, None).expect("Rust VWMA indicator failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut volume_chunks = volume[min_data_val..].chunks_exact(CHUNK_SIZE);

                for (close_chunk, volume_chunk) in close_chunks.by_ref().zip(volume_chunks.by_ref())
                {
                    let close_vec = close_chunk.to_vec();
                    let volume_vec = volume_chunk.to_vec();
                    let chunk_inputs = [close_vec.as_slice(), volume_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VWMA batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Handle remainder
                let close_rem = close_chunks.remainder();
                let volume_rem = volume_chunks.remainder();
                if !close_rem.is_empty() && !volume_rem.is_empty() {
                    let close_vec = close_rem.to_vec();
                    let volume_vec = volume_rem.to_vec();
                    let chunk_inputs = [close_vec.as_slice(), volume_vec.as_slice()];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VWMA batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Compare outputs
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

    fn get_close_volume_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>) {
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
        (close, volume)
    }

    #[test]
    fn test_vwma_simd_by_assets() {
        use tulip_rs::indicators::vwma::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks for SIMD testing
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (close, volume) = get_close_volume_arrays(data);
                (symbol.clone(), close, volume)
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
            // Check min data requirement
            let min_data_required = min_data(&options);
            let min_stock_len = stock_data
                .iter()
                .map(|(_, c, _)| c.len())
                .min()
                .unwrap_or(0);

            if min_stock_len < min_data_required {
                println!(
                    "Skipping VWMA SIMD test with options {:?} - insufficient data (need: {}, have: {})",
                    options, min_data_required, min_stock_len
                );
                continue;
            }

            // Run SIMD by assets implementation
            let (simd_outputs, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets VWMA indicator failed");

            // Compare with individual Rust implementations
            for i in 0..4 {
                let individual_inputs = [stock_data[i].1.as_slice(), stock_data[i].2.as_slice()];
                let (individual_outputs, _) = rust_vwma(&individual_inputs, &options, None)
                    .expect("Individual Rust VWMA indicator failed");

                // Compare outputs
                assert_eq!(
                    simd_outputs[i][0].len(),
                    individual_outputs[0].len(),
                    "Output lengths don't match for stock {} with options {:?}",
                    stock_data[i].0,
                    options
                );

                for (j, (&simd_val, &individual_val)) in simd_outputs[i][0]
                    .iter()
                    .zip(individual_outputs[0].iter())
                    .enumerate()
                {
                    // Check for NaN or infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD VWMA has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            j, stock_data[i].0, options, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD VWMA has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            j, stock_data[i].0, options, simd_val
                        );
                    }

                    if !approx_eq!(f64, simd_val, individual_val, epsilon = EPSILON) {
                        panic!(
                            "SIMD vs Individual mismatch at index {} for stock {} with options {:?}: SIMD = {}, Individual = {}",
                            j, stock_data[i].0, options, simd_val, individual_val
                        );
                    }
                }

                println!(
                    "✓ SIMD by assets vs Regular test passed for stock {} with options {:?}",
                    stock_data[i].0, options
                );
            }
        }

        println!("✓ All SIMD by assets vs Regular VWMA database tests passed!");
    }

    // SIMD-by-options test for VWMA (compare SIMD to regular for all options)
    #[test]
    fn test_vwma_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::vwma::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (close, volume) = get_close_volume_arrays(&stock_data);
            let inputs = [close.as_slice(), volume.as_slice()];

            // Process first 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD VWMA 4-wide failed");

            // Process remaining 2 options with 2-wide SIMD
            let options_2 = [&OPTIONS_LIST[4], &OPTIONS_LIST[5]];
            let (simd_results_2, _) = indicator_by_options::<2>(&inputs, &options_2, None)
                .expect("SIMD VWMA 2-wide failed");

            // Combine SIMD results
            let mut all_simd_results = Vec::new();
            for i in 0..4 {
                all_simd_results.push(simd_results_4[i].clone());
            }
            for i in 0..2 {
                all_simd_results.push(simd_results_2[i].clone());
            }

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_vwma(&inputs, options, None).expect("Regular VWMA indicator failed");

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
                            "SIMD VWMA has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD VWMA has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with high precision
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }
    }
}
