#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::ad::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_ad, ti_ad_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};
    const EPSILON: f64 = 1e-2;
    const CHUNK_SIZE: usize = 100;
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
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

    // Options are empty for AD

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the benchmark enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut close_vec = CLOSE.to_vec();
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut volume_vec = VOLUME.to_vec();
        for _ in 0..200 {
            close_vec.extend_from_slice(&CLOSE);
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            volume_vec.extend_from_slice(&VOLUME);
        }
        (high_vec, low_vec, close_vec, volume_vec)
    }

    #[test]
    fn test_ad_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close, volume) = expand_inputs();
        let options = [];

        // Prepare inputs for the C implementation
        let inputs_c: Vec<*const f64> =
            vec![high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr()];

        // Determine the offset required by the C AD function
        let start_index = unsafe { ti_ad_start(options.as_ptr()) };
        assert!(start_index >= 0, "ti_ad_start returned a negative index");
        let output_len = close.len() - (start_index as usize);

        // Run the C implementation
        let mut output_vec_c = vec![0.0_f64; output_len];
        let output_ptr: *mut f64 = output_vec_c.as_mut_ptr();
        let mut outputs_c: Vec<*mut f64> = vec![output_ptr];
        let ret = unsafe {
            ti_ad(
                close.len() as i32,
                inputs_c.as_ptr(),
                options.as_ptr(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_ad returned error code {}", ret);

        // Run the Rust implementation
        let inputs_rust = [
            high.as_slice(),
            low.as_slice(),
            close.as_slice(),
            volume.as_slice(),
        ];
        let (outputs, _) =
            indicator(&inputs_rust, &options, None).expect("Rust AD indicator failed");

        // Compare the outputs
        for (i, (&c_val, &rust_val)) in output_vec_c.iter().zip(outputs[0].iter()).enumerate() {
            // Fail test if Rust has NaN
            if rust_val.is_nan() {
                panic!(
                    "Rust AD has NaN at index {}: Rust = {}, Options = {:?}",
                    i, rust_val, options
                );
            }

            // Fail test if Rust has infinity
            if rust_val.is_infinite() {
                panic!(
                    "Rust AD has infinity at index {}: Rust = {}, Options = {:?}",
                    i, rust_val, options
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
                i,
                c_val,
                rust_val
            );
        }
    }

    #[test]
    fn test_ad_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);

            let options = [];
            // run c code
            let inputs_c: Vec<*const f64> =
                vec![high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr()];

            // Determine the offset required by the C AD function
            let start_index = unsafe { ti_ad_start(std::ptr::null()) };
            assert!(start_index >= 0, "ti_ad_start returned a negative index");
            let output_len_c = close.len() - (start_index as usize);

            // Run the C implementation
            let mut ad_output_vec_c = vec![0.0_f64; output_len_c];
            let ad_ptr: *mut f64 = ad_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![ad_ptr];
            let ret = unsafe {
                ti_ad(
                    close.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_ad returned error code {}", ret);

            let inputs_rust = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];
            let (outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Rust AD indicator failed");

            let output_len_rust = outputs[0].len();

            for (i, (&c_val, &rust_val)) in ad_output_vec_c
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
                        "Rust AD has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                        index, rust_val, options, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust AD has infinity at index {}: Rust = {}, Options = {:?}, Stock: {}",
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
                    /*println!(
                        "Test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Stock: {}",
                        index, ad_output_vec_c, outputs[0], stock_symbol
                    );*/
                    panic!(
                        "Mismatch at index {}: C = {}, Rust = {}, Stock: {}",
                        index, c_val, rust_val, stock_symbol
                    );
                }
            }
        }
    }

    #[test]
    fn test_ad_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let inputs_rust = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];
            let options = [];

            // Get full output from processing all data at once
            let (full_outputs, _) =
                indicator(&inputs_rust, &options, None).expect("Rust AD indicator failed");

            // Process data in batches and accumulate outputs
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

            let (first_outputs, mut state) =
                indicator(&chunk_inputs, &options, None).expect("Rust AD indicator failed");
            batch_full_output.extend_from_slice(&first_outputs[0]);

            // Process remaining data in chunks
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
                    .expect("Rust AD batch indicator failed");
                batch_full_output.extend_from_slice(&chunk_outputs[0]);
            }

            // Handle remainder
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
                    .expect("Rust AD batch indicator failed");
                batch_full_output.extend_from_slice(&chunk_outputs[0]);
            }

            // Compare full output with batch output
            assert_eq!(
                full_outputs[0].len(),
                batch_full_output.len(),
                "Output lengths don't match for stock: {}",
                stock_symbol
            );

            for (i, (&full_val, &batch_val)) in full_outputs[0]
                .iter()
                .zip(batch_full_output.iter())
                .enumerate()
            {
                assert_eq!(
                    full_val, batch_val,
                    "State handover mismatch at index {} for stock {}: full = {}, batch = {}",
                    i, stock_symbol, full_val, batch_val
                );
            }
        }
    }
    #[test]
    fn test_ad_simd_vs_regular_database() {
        use tulip_rs::indicators::ad::indicator_by_assets;

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

        let options = [];

        // Get SIMD by assets result
        let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
            .expect("SIMD by assets AD indicator failed");

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
                indicator(&stock_inputs, &options, None).expect("Regular AD indicator failed");

            let simd_result = &simd_results[stock_idx][0];
            let regular_result = &regular_results[0];

            // Compare output lengths
            assert_eq!(
                simd_result.len(),
                regular_result.len(),
                "Output length mismatch for stock {}: SIMD={}, Regular={}",
                stock_symbol,
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
                        "SIMD by assets AD has NaN at index {} for stock {}: SIMD = {}",
                        i, stock_symbol, simd_val
                    );
                }

                if simd_val.is_infinite() {
                    panic!(
                        "SIMD by assets AD has infinity at index {} for stock {}: SIMD = {}",
                        i, stock_symbol, simd_val
                    );
                }

                // Compare values with appropriate epsilon for AD
                if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                    panic!(
                        "Mismatch at index {} for stock {}: SIMD by assets = {}, Regular = {}",
                        i, stock_symbol, simd_val, regular_val
                    );
                }
            }

            println!(
                "✓ SIMD by assets vs Regular test passed for stock {}",
                stock_symbol
            );
        }

        println!("✓ All SIMD by assets vs Regular AD database tests passed!");
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
}
