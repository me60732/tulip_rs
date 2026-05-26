#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::marketfi::{indicator as rust_marketfi, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_marketfi, ti_marketfi_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    //const OPTIONS: [f64; 0] = [];
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
    fn test_marketfi_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, volume) = expand_inputs();

        // Prepare inputs for the C implementation
        let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), volume.as_ptr()];

        // Determine the offset required by the C MarketFI function
        let start_index = unsafe { ti_marketfi_start(std::ptr::null()) };
        assert!(
            start_index >= 0,
            "ti_marketfi_start returned a negative index"
        );
        let output_len_c = high.len() - (start_index as usize);

        // Run the C implementation
        let mut marketfi_output_vec_c = vec![0.0_f64; output_len_c];
        let marketfi_ptr: *mut f64 = marketfi_output_vec_c.as_mut_ptr();
        let mut outputs_c: Vec<*mut f64> = vec![marketfi_ptr];
        let ret = unsafe {
            ti_marketfi(
                high.len() as i32,
                inputs_c.as_ptr(),
                std::ptr::null(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_marketfi returned error code {}", ret);

        // Run the Rust implementation
        let inputs_rust = [high.as_slice(), low.as_slice(), volume.as_slice()];
        let (outputs, _) =
            rust_marketfi(&inputs_rust, &[], None).expect("Rust MARKETFI indicator failed");

        let output_len_rust = outputs[0].len();

        // Compare the outputs in reverse for the length of the Rust outputs
        for (i, (&c_val, &rust_val)) in marketfi_output_vec_c
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
                    "Rust MARKETFI has NaN at index {}: Rust = {}",
                    index, rust_val
                );
            }

            // Fail test if Rust has infinity
            if rust_val.is_infinite() {
                panic!(
                    "Rust MARKETFI has infinity at index {}: Rust = {}",
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
                    index, marketfi_output_vec_c, outputs[0]
                );
                panic!(
                    "Mismatch at index {}: C = {}, Rust = {}",
                    index, c_val, rust_val
                );
            }
        }
    }

    #[test]
    fn test_marketfi_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(&stock_data);

            // C implementation
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), volume.as_ptr()];

            let start_index = unsafe { ti_marketfi_start(std::ptr::null()) };
            assert!(
                start_index >= 0,
                "ti_marketfi_start returned a negative index"
            );
            let output_len_c = high.len() - (start_index as usize);

            let mut marketfi_output_vec_c = vec![0.0_f64; output_len_c];
            let marketfi_ptr: *mut f64 = marketfi_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![marketfi_ptr];
            let ret = unsafe {
                ti_marketfi(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_marketfi returned error code {}", ret);

            // Rust implementation
            let inputs_rust = [high.as_slice(), low.as_slice(), volume.as_slice()];
            let (outputs, _) =
                rust_marketfi(&inputs_rust, &[], None).expect("Rust MarketFI indicator failed");

            let output_len_rust = outputs[0].len();

            // Compare results
            for (i, (&c_val, &rust_val)) in marketfi_output_vec_c
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
                        "Rust MARKETFI has NaN at index {}: Rust = {},  Stock: {}",
                        index, rust_val, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust MARKETFI has infinity at index {}: Rust = {}",
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
                        index, marketfi_output_vec_c, outputs[0], stock_symbol
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
    fn test_marketfi_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, volume) = get_hlv_arrays(&stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice(), volume.as_slice()];

            // Get full output
            let (full_outputs, _) = rust_marketfi(&inputs_rust, &[], None)
                .expect("Failed to run MARKETFI indicator on full data");

            // Process in batches
            let mut batch_full_output = Vec::new();

            let min_data_val = min_data(&[]).max(CHUNK_SIZE);

            // First chunk - convert to Vec<&Vec<f64>>
            let high_vec = high[..min_data_val].to_vec();
            let low_vec = low[..min_data_val].to_vec();
            let volume_vec = volume[..min_data_val].to_vec();
            let chunk_inputs = [
                high_vec.as_slice(),
                low_vec.as_slice(),
                volume_vec.as_slice(),
            ];

            let (first_outputs, mut state) = rust_marketfi(&chunk_inputs, &[], None)
                .expect("Failed to run MARKETFI indicator on first chunk");
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
                    .expect("MARKETFI batch indicator failed");
                batch_full_output.extend_from_slice(&chunk_outputs[0]);
            }

            // Process remainder if any
            let high_rem = high_chunks.remainder();
            let low_rem = low_chunks.remainder();
            let volume_rem = volume_chunks.remainder();
            if !high_rem.is_empty() {
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
                    .expect("MARKETFI batch indicator failed");
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
                    "Mismatch in MARKETFI output at index {}: full = {}, batch = {}, Stock: {}",
                    i, full_val, batch_val, stock_symbol
                );
            }
        }
    }

    #[test]
    fn test_marketfi_simd_vs_regular_database() {
        use tulip_rs::indicators::marketfi::indicator_by_assets;

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
            &[&stock_data[0].1, &stock_data[0].2, &stock_data[0].3], // high, low, volume
            &[&stock_data[1].1, &stock_data[1].2, &stock_data[1].3], // high, low, volume
            &[&stock_data[2].1, &stock_data[2].2, &stock_data[2].3], // high, low, volume
            &[&stock_data[3].1, &stock_data[3].2, &stock_data[3].3], // high, low, volume
        ];

        // Get SIMD by assets result
        let (simd_results, _) = indicator_by_assets::<4>(&inputs, &[], None)
            .expect("SIMD by assets MARKETFI indicator failed");

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
                rust_marketfi(&stock_inputs, &[], None).expect("Regular MARKETFI indicator failed");

            let simd_result = &simd_results[stock_idx][0];
            let regular_result = &regular_results[0];

            // Compare output lengths
            assert_eq!(
                simd_result.len(),
                regular_result.len(),
                "Output length mismatch for stock {} with options []: SIMD={}, Regular={}",
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
                        "SIMD by assets MARKETFI has NaN at index {} for stock {}: SIMD = {}",
                        i, stock_symbol, simd_val
                    );
                }

                if simd_val.is_infinite() {
                    panic!(
                        "SIMD by assets MARKETFI has infinity at index {} for stock {}: SIMD = {}",
                        i, stock_symbol, simd_val
                    );
                }

                // Compare values with appropriate epsilon for MARKETFI
                if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                    println!(
                        "SIMD: {:?}\n\nRegular: {:?}",
                        &simd_result[..20.min(simd_result.len())],
                        &regular_result[..20.min(regular_result.len())]
                    );
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

        println!("✓ All SIMD by assets vs Regular MARKETFI database tests passed!");
    }
}
