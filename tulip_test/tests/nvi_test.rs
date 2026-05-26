#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::nvi::indicator_by_assets;
    use tulip_rs::indicators::nvi::{indicator as rust_nvi, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_nvi, ti_nvi_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILON: f64 = 1e-10;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    const VOLUME: [f64; 15] = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];

    fn get_cv_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>) {
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        let volume: Vec<f64> = stock_data.iter().map(|d| d.volume).collect();
        (close, volume)
    }

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
    fn test_nvi_indicator() {
        // Use the same input data as in the benchmarks
        let (close, volume) = expand_inputs();

        // Prepare inputs for the C implementation
        let inputs_c: Vec<*const f64> = vec![close.as_ptr(), volume.as_ptr()];

        // Determine the offset required by the C NVI function
        let start_index = unsafe { ti_nvi_start(std::ptr::null()) };
        assert!(start_index >= 0, "ti_nvi_start returned a negative index");
        let output_len_c = close.len() - (start_index as usize);

        // Run the C implementation
        let mut nvi_output_vec_c = vec![0.0_f64; output_len_c];
        let nvi_ptr: *mut f64 = nvi_output_vec_c.as_mut_ptr();
        let mut outputs_c: Vec<*mut f64> = vec![nvi_ptr];
        let ret = unsafe {
            ti_nvi(
                close.len() as i32,
                inputs_c.as_ptr(),
                std::ptr::null(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_nvi returned error code {}", ret);

        // Run the Rust implementation
        let inputs_rust = [close.as_slice(), volume.as_slice()];
        let (outputs, _) = rust_nvi(&inputs_rust, &[], None).expect("Rust NVI indicator failed");

        let output_len_rust = outputs[0].len();

        // Compare the outputs in reverse for the length of the Rust outputs
        for (i, (&c_val, &rust_val)) in nvi_output_vec_c
            .iter()
            .rev()
            .take(output_len_rust)
            .zip(outputs[0].iter().rev())
            .enumerate()
        {
            let index = output_len_rust - i - 1;

            // Fail test if Rust has NaN
            if rust_val.is_nan() {
                panic!("Rust NVI has NaN at index {}: Rust = {}", index, rust_val);
            }

            // Fail test if Rust has infinity
            if rust_val.is_infinite() {
                panic!(
                    "Rust NVI has infinity at index {}: Rust = {}",
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
                    "Test failed at index {}: \nC = {:?}, \nRust = {:?}",
                    index, nvi_output_vec_c, outputs[0]
                );
                panic!(
                    "Mismatch at index {}: C = {}, Rust = {}",
                    index, c_val, rust_val
                );
            }
        }
    }

    #[test]
    fn test_nvi_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (close, volume) = get_cv_arrays(&stock_data);

            // C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr(), volume.as_ptr()];

            let start_index = unsafe { ti_nvi_start(std::ptr::null()) };
            assert!(start_index >= 0, "ti_nvi_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut nvi_output_vec_c = vec![0.0_f64; output_len_c];
            let nvi_ptr: *mut f64 = nvi_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![nvi_ptr];
            let ret = unsafe {
                ti_nvi(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_nvi returned error code {}", ret);

            // Rust implementation
            let inputs_rust = [close.as_slice(), volume.as_slice()];
            let (outputs, _) =
                rust_nvi(&inputs_rust, &[], None).expect("Rust NVI indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare results
            for (i, (&c_val, &rust_val)) in nvi_output_vec_c
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
                        "Rust NVI has NaN at index {}: Rust = {}, Stock: {}",
                        index, rust_val, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust NVI has infinity at index {}: Rust = {}",
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
                        index, nvi_output_vec_c, outputs[0], stock_symbol
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
    fn test_nvi_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (close, volume) = get_cv_arrays(&stock_data);
            let inputs_rust = [close.as_slice(), volume.as_slice()];

            // Get full output
            let (full_outputs, _) = rust_nvi(&inputs_rust, &[], None)
                .expect("Failed to run NVI indicator on full data");

            // Process in batches
            let mut batch_full_output = Vec::new();

            let min_data_val = min_data(&[]).max(CHUNK_SIZE);

            // First chunk - convert to Vec<&Vec<f64>>
            let close_vec = close[..min_data_val].to_vec();
            let volume_vec = volume[..min_data_val].to_vec();
            let chunk_inputs = [close_vec.as_slice(), volume_vec.as_slice()];

            let (first_outputs, mut state) = rust_nvi(&chunk_inputs, &[], None)
                .expect("Failed to run NVI indicator on first chunk");
            batch_full_output.extend_from_slice(&first_outputs[0]);

            // Process remaining data in chunks using state
            let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);
            let mut volume_chunks = volume[min_data_val..].chunks_exact(CHUNK_SIZE);

            for (close_chunk, volume_chunk) in close_chunks.by_ref().zip(volume_chunks.by_ref()) {
                let close_vec = close_chunk.to_vec();
                let volume_vec = volume_chunk.to_vec();
                let chunk_inputs = [close_vec.as_slice(), volume_vec.as_slice()];
                let chunk_outputs = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("NVI batch indicator failed");
                batch_full_output.extend_from_slice(&chunk_outputs[0]);
            }

            // Process remainder if any
            let close_rem = close_chunks.remainder();
            let volume_rem = volume_chunks.remainder();
            if !close_rem.is_empty() {
                let close_vec = close_rem.to_vec();
                let volume_vec = volume_rem.to_vec();
                let chunk_inputs = [close_vec.as_slice(), volume_vec.as_slice()];
                let chunk_outputs = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("NVI batch indicator failed");
                batch_full_output.extend_from_slice(&chunk_outputs[0]);
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
                    "Mismatch in NVI output at index {}: full = {}, batch = {}, Stock: {}",
                    i, full_val, batch_val, stock_symbol
                );
            }
        }
    }

    #[test]
    fn test_nvi_simd_by_assets() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks for SIMD testing
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let close = data.iter().map(|d| d.close).collect();
                let volume = data.iter().map(|d| d.volume).collect();
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

        // Run SIMD by assets implementation
        let (simd_outputs, _) = indicator_by_assets::<4>(&inputs, &[], None)
            .expect("SIMD by assets NVI indicator failed");

        // Compare with individual Rust implementations
        for i in 0..4 {
            let individual_inputs = [stock_data[i].1.as_slice(), stock_data[i].2.as_slice()];
            let (individual_outputs, _) = rust_nvi(&individual_inputs, &[], None)
                .expect("Individual Rust NVI indicator failed");

            // Compare outputs
            assert_eq!(
                simd_outputs[i][0].len(),
                individual_outputs[0].len(),
                "Output lengths don't match for stock {}",
                stock_data[i].0
            );

            for (j, (&simd_val, &individual_val)) in simd_outputs[i][0]
                .iter()
                .zip(individual_outputs[0].iter())
                .enumerate()
            {
                // Check for NaN or infinity in SIMD result
                if simd_val.is_nan() {
                    panic!(
                        "SIMD NVI has NaN at index {}: SIMD = {}, Stock: {}",
                        j, simd_val, stock_data[i].0
                    );
                }

                if simd_val.is_infinite() {
                    panic!(
                        "SIMD NVI has infinity at index {}: SIMD = {}, Stock: {}",
                        j, simd_val, stock_data[i].0
                    );
                }

                if !approx_eq!(f64, simd_val, individual_val, epsilon = EPSILON) {
                    panic!(
                        "SIMD vs Individual mismatch at index {} for stock {}: SIMD = {}, Individual = {}",
                        j, stock_data[i].0, simd_val, individual_val
                    );
                }
            }
        }
    }
}
