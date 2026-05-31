#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::bop::{indicator, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_bop, ti_bop_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const OPEN: [f64; 15] = [
        81.85, 81.20, 81.55, 82.91, 83.10, 83.41, 82.71, 82.70, 84.20, 84.25, 84.03, 85.45, 86.18,
        88.00, 87.60,
    ];
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

    const OPTIONS: [f64; 0] = [];

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut open_vec = OPEN.to_vec();
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..3 {
            open_vec.extend_from_slice(&OPEN);
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
        }
        (open_vec, high_vec, low_vec, close_vec)
    }

    #[test]
    fn test_bop_indicator() {
        // Use the same input data as in the benchmarks
        let (open, high, low, close) = expand_inputs();

        // Prepare inputs for the C implementation
        let inputs_c: Vec<*const f64> =
            vec![open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr()];

        // Determine the offset required by the C BOP function
        let start_index = unsafe { ti_bop_start(OPTIONS.as_ptr()) };
        assert!(start_index >= 0, "ti_bop_start returned a negative index");
        let output_len_c = open.len() - (start_index as usize);

        // Run the C implementation
        let mut bop_output_vec_c = vec![0.0_f64; output_len_c];
        let bop_ptr: *mut f64 = bop_output_vec_c.as_mut_ptr();
        let mut outputs_c: Vec<*mut f64> = vec![bop_ptr];
        let ret = unsafe {
            ti_bop(
                open.len() as i32,
                inputs_c.as_ptr(),
                OPTIONS.as_ptr(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_bop returned error code {}", ret);

        // Run the Rust implementation
        let inputs_rust = [
            open.as_slice(),
            high.as_slice(),
            low.as_slice(),
            close.as_slice(),
        ];
        let (outputs, _) =
            indicator(&inputs_rust, &OPTIONS, None).expect("Rust BOP indicator failed");

        let output_len_rust = outputs[0].len();

        // Compare the outputs in reverse for the length of the Rust outputs
        for (i, (&c_val, &rust_val)) in bop_output_vec_c
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
                    "Rust BOP has NaN at index {}: Rust = {}, Options = {:?}",
                    index, rust_val, OPTIONS
                );
            }

            // Fail test if Rust has infinity
            if rust_val.is_infinite() {
                panic!(
                    "Rust BOP has infinity at index {}: Rust = {}",
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
                    "Test failed at index {}: \nC = {:?}, \nRust = {:?}, Options = {:?}",
                    index, bop_output_vec_c, outputs[0], OPTIONS
                );
                panic!(
                    "Mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                    index, c_val, rust_val, OPTIONS
                );
            }
        }
    }

    #[test]
    fn test_bop_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (open, high, low, close) = get_ohlc_arrays(stock_data);

            // run c code
            let inputs_c: Vec<*const f64> =
                vec![open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr()];

            // Determine the offset required by the C BOP function
            let start_index = unsafe { ti_bop_start(std::ptr::null()) };
            assert!(start_index >= 0, "ti_bop_start returned a negative index");
            let output_len_c = open.len() - (start_index as usize);

            // Run the C implementation
            let mut bop_output_vec_c = vec![0.0_f64; output_len_c];
            let bop_ptr: *mut f64 = bop_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![bop_ptr];
            let ret = unsafe {
                ti_bop(
                    open.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_bop returned error code {}", ret);

            let inputs_rust = [
                open.as_slice(),
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
            ];
            let (outputs, _) =
                indicator(&inputs_rust, &OPTIONS, None).expect("Rust BOP indicator failed");

            let output_len_rust = outputs[0].len();

            for (i, (&c_val, &rust_val)) in bop_output_vec_c
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
                        "Rust BOP has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                        index, rust_val, OPTIONS, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust BOP has infinity at index {}: Rust = {}",
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
                        index, bop_output_vec_c, outputs[0], stock_symbol
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
    fn test_bop_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (open, high, low, close) = get_ohlc_arrays(stock_data);
            let inputs_rust = [
                open.as_slice(),
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
            ];
            let options: [f64; 0] = [];

            // Get full output
            let (full_outputs, _) = indicator(&inputs_rust, &options, None)
                .expect("Failed to run BOP indicator on full data");

            // Process in batches
            let mut batch_full_output = Vec::new();

            let min_data_val = min_data(&options).max(CHUNK_SIZE);

            if open.len() <= min_data_val {
                // If data is too small, just run full calculation
                let (outputs, _) =
                    indicator(&inputs_rust, &options, None).expect("Failed to run BOP indicator");
                batch_full_output.extend_from_slice(&outputs[0]);
            } else {
                // First chunk - convert to Vec<&Vec<f64>>
                let open_vec = open[..min_data_val].to_vec();
                let high_vec = high[..min_data_val].to_vec();
                let low_vec = low[..min_data_val].to_vec();
                let close_vec = close[..min_data_val].to_vec();
                let chunk_inputs = [
                    open_vec.as_slice(),
                    high_vec.as_slice(),
                    low_vec.as_slice(),
                    close_vec.as_slice(),
                ];

                let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                    .expect("Failed to run BOP indicator on first chunk");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Process remaining data in chunks using state
                let mut open_chunks = open[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                for (((open_chunk, high_chunk), low_chunk), close_chunk) in open_chunks
                    .by_ref()
                    .zip(high_chunks.by_ref())
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let open_vec = open_chunk.to_vec();
                    let high_vec = high_chunk.to_vec();
                    let low_vec = low_chunk.to_vec();
                    let close_vec = close_chunk.to_vec();
                    let chunk_inputs = [
                        open_vec.as_slice(),
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                    ];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("BOP batch indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Process remainder if any
                let open_rem = open_chunks.remainder();
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();

                if !open_rem.is_empty() {
                    let open_vec = open_rem.to_vec();
                    let high_vec = high_rem.to_vec();
                    let low_vec = low_rem.to_vec();
                    let close_vec = close_rem.to_vec();
                    let chunk_inputs = [
                        open_vec.as_slice(),
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                    ];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("BOP batch indicator failed");
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

    fn get_ohlc_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let open: Vec<f64> = stock_data.iter().map(|d| d.open).collect();
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (open, high, low, close)
    }

    #[test]
    fn test_bop_simd_vs_regular_database() {
        use tulip_rs::indicators::bop::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Get first 4 stocks' data
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| {
                let (open, high, low, close) = get_ohlc_arrays(data);
                (symbol.clone(), open, high, low, close)
            })
            .collect();

        // Prepare inputs in the format expected by indicator_by_assets
        let inputs: [&[&[f64]; 4]; 4] = [
            &[
                &stock_data[0].1, // open
                &stock_data[0].2, // high
                &stock_data[0].3, // low
                &stock_data[0].4, // close
            ],
            &[
                &stock_data[1].1, // open
                &stock_data[1].2, // high
                &stock_data[1].3, // low
                &stock_data[1].4, // close
            ],
            &[
                &stock_data[2].1, // open
                &stock_data[2].2, // high
                &stock_data[2].3, // low
                &stock_data[2].4, // close
            ],
            &[
                &stock_data[3].1, // open
                &stock_data[3].2, // high
                &stock_data[3].3, // low
                &stock_data[3].4, // close
            ],
        ];

        // Test without optional outputs (BOP doesn't have optional outputs)
        {
            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &OPTIONS, None)
                .expect("SIMD by assets BOP indicator failed");

            // Compare each SIMD result with regular indicator for each stock
            for (stock_idx, (stock_symbol, stock_open, stock_high, stock_low, stock_close)) in
                stock_data.iter().enumerate()
            {
                // Get regular indicator result for this stock
                let stock_inputs = [
                    stock_open.as_slice(),
                    stock_high.as_slice(),
                    stock_low.as_slice(),
                    stock_close.as_slice(),
                ];
                let (regular_results, _) =
                    indicator(&stock_inputs, &OPTIONS, None).expect("Regular BOP indicator failed");

                let simd_result = &simd_results[stock_idx][0];
                let regular_result = &regular_results[0];

                // Compare output lengths
                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "Output length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
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
                            "SIMD by assets BOP has NaN at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, OPTIONS, simd_val
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD by assets BOP has infinity at index {} for stock {} with options {:?}: SIMD = {}",
                            i, stock_symbol, OPTIONS, simd_val
                        );
                    }

                    // Compare values with appropriate epsilon for BOP
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        panic!(
                            "Mismatch at index {} for stock {} with options {:?}: SIMD by assets = {}, Regular = {}",
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

        println!("✓ All SIMD by assets vs Regular BOP database tests passed!");
    }
}
