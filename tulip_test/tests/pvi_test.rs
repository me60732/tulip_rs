#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::pvi::indicator_by_assets;
    use tulip_rs::indicators::pvi::{indicator as rust_pvi, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_pvi, ti_pvi_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-8;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    const VOLUME: [f64; 15] = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];

    const CHUNK_SIZE: usize = 100;

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
    fn test_pvi_indicator() {
        // Use the same input data as in the benchmarks
        let (close, volume) = expand_inputs();

        // Prepare inputs for the C implementation
        let inputs_c: Vec<*const f64> = vec![close.as_ptr(), volume.as_ptr()];

        // Determine the offset required by the C PVI function
        let start_index = unsafe { ti_pvi_start(std::ptr::null()) };
        assert!(start_index >= 0, "ti_pvi_start returned a negative index");
        let output_len_c = close.len() - (start_index as usize);

        // Run the C implementation
        let mut pvi_output_vec_c = vec![0.0_f64; output_len_c];
        let pvi_ptr: *mut f64 = pvi_output_vec_c.as_mut_ptr();
        let mut outputs_c: Vec<*mut f64> = vec![pvi_ptr];
        let ret = unsafe {
            ti_pvi(
                close.len() as i32,
                inputs_c.as_ptr(),
                std::ptr::null(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_pvi returned error code {}", ret);

        // Run the Rust implementation
        let inputs_rust = [close.as_slice(), volume.as_slice()];
        let (outputs, _) = rust_pvi(&inputs_rust, &[], None).expect("Rust PVI indicator failed");

        let output_len_rust = outputs[0].len();

        // Compare the outputs in reverse for the length of the Rust outputs
        for (i, (&c_val, &rust_val)) in pvi_output_vec_c
            .iter()
            .rev()
            .take(output_len_rust)
            .zip(outputs[0].iter().rev())
            .enumerate()
        {
            let index = output_len_rust - i - 1;

            // Fail test if Rust has NaN
            if rust_val.is_nan() {
                panic!("Rust PVI has NaN at index {}: Rust = {}", index, rust_val);
            }

            // Fail test if Rust has infinity
            if rust_val.is_infinite() {
                panic!(
                    "Rust PVI has infinity at index {}: Rust = {}",
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
                    index, pvi_output_vec_c, outputs[0]
                );
                panic!(
                    "Mismatch at index {}: C = {}, Rust = {}",
                    index, c_val, rust_val
                );
            }
        }
    }

    #[test]
    fn test_pvi_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (close, volume) = get_cv_arrays(stock_data);

            // C implementation
            let inputs_c: Vec<*const f64> = vec![close.as_ptr(), volume.as_ptr()];

            let start_index = unsafe { ti_pvi_start(std::ptr::null()) };
            assert!(start_index >= 0, "ti_pvi_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            let mut pvi_output_vec_c = vec![0.0_f64; output_len_c];
            let pvi_ptr: *mut f64 = pvi_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![pvi_ptr];
            let ret = unsafe {
                ti_pvi(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_pvi returned error code {}", ret);

            // Rust implementation
            let inputs_rust = [close.as_slice(), volume.as_slice()];
            let (outputs, _) =
                rust_pvi(&inputs_rust, &[], None).expect("Rust PVI indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare results
            for (i, (&c_val, &rust_val)) in pvi_output_vec_c
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
                        "Rust PVI has NaN at index {}: Rust = {}, Stock: {}",
                        index, rust_val, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust PVI has infinity at index {}: Rust = {}",
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
                        index, pvi_output_vec_c, outputs[0], stock_symbol
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
    fn test_pvi_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (close, volume) = get_cv_arrays(stock_data);
            let inputs_rust = [close.as_slice(), volume.as_slice()];

            // Get full output
            let (full_outputs, _) =
                rust_pvi(&inputs_rust, &[], None).expect("PVI indicator should work on full data");

            // Process in batches
            let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

            let min_data_val = min_data(&[]).max(CHUNK_SIZE);

            // Process first chunk to get initial state
            let first_chunk_size = min_data_val.min(close.len());
            let first_close = close[..first_chunk_size].to_vec();
            let first_volume = volume[..first_chunk_size].to_vec();
            let first_inputs = [first_close.as_slice(), first_volume.as_slice()];

            let (outputs, mut state) = rust_pvi(&first_inputs, &[], None)
                .expect("PVI indicator should work on first chunk");

            for output_idx in 0..outputs.len() {
                batch_full_outputs[output_idx].extend_from_slice(&outputs[output_idx]);
            }

            let mut processed = first_chunk_size;

            // Process subsequent chunks using state.batch_indicator
            while processed < close.len() {
                let end = (processed + CHUNK_SIZE).min(close.len());

                let chunk_close = close[processed..end].to_vec();
                let chunk_volume = volume[processed..end].to_vec();
                let chunk_inputs = [chunk_close.as_slice(), chunk_volume.as_slice()];

                let chunk_outputs = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("PVI batch indicator failed");

                for output_idx in 0..chunk_outputs.len() {
                    batch_full_outputs[output_idx].extend_from_slice(&chunk_outputs[output_idx]);
                }

                processed = end;
            }

            // Compare all outputs
            for output_idx in 0..full_outputs.len() {
                assert_eq!(
                    full_outputs[output_idx].len(),
                    batch_full_outputs[output_idx].len(),
                    "Output length mismatch for stock {}, output {}",
                    stock_symbol,
                    output_idx
                );

                for (i, (&full_val, &batch_val)) in full_outputs[output_idx]
                    .iter()
                    .zip(batch_full_outputs[output_idx].iter())
                    .enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "State handover test failed for stock {}, output {}, index {}: full = {}, batch = {}",
                        stock_symbol, output_idx, i, full_val, batch_val
                    );
                }
            }
        }
    }

    #[test]
    fn test_pvi_simd_by_assets() {
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
            .expect("SIMD by assets PVI indicator failed");

        // Compare with individual Rust implementations
        for i in 0..4 {
            let individual_inputs = [stock_data[i].1.as_slice(), stock_data[i].2.as_slice()];
            let (individual_outputs, _) = rust_pvi(&individual_inputs, &[], None)
                .expect("Individual Rust PVI indicator failed");

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
                        "SIMD PVI has NaN at index {}: SIMD = {}, Stock: {}",
                        j, simd_val, stock_data[i].0
                    );
                }

                if simd_val.is_infinite() {
                    panic!(
                        "SIMD PVI has infinity at index {}: SIMD = {}, Stock: {}",
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
