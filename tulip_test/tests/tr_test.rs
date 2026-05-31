#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::tr::{indicator as rust_tr, min_data, TIndicatorState};
    use tulip_test::c_bindings::{ti_tr, ti_tr_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    //#[cfg(feature = "nightly")]
    use tulip_rs::indicators::tr::indicator_by_assets as rust_tr_simd;

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

    /// Expand the sample input data by repeating it.
    /// Adjust the number of repetitions to give the test enough work.
    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
        }
        (high_vec, low_vec, close_vec)
    }

    #[test]
    fn test_tr_indicator() {
        // Use the same input data as in the benchmarks
        let (high, low, close) = expand_inputs();

        // Prepare inputs for the C implementation
        let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

        // Determine the offset required by the C TR function
        let start_index = unsafe { ti_tr_start(std::ptr::null()) };
        assert!(start_index >= 0, "ti_tr_start returned a negative index");
        let output_len_c = high.len() - (start_index as usize);

        // Run the C implementation
        let mut tr_output_vec_c = vec![0.0_f64; output_len_c];
        let tr_ptr: *mut f64 = tr_output_vec_c.as_mut_ptr();
        let mut outputs_c: Vec<*mut f64> = vec![tr_ptr];
        let ret = unsafe {
            ti_tr(
                high.len() as i32,
                inputs_c.as_ptr(),
                std::ptr::null(),
                outputs_c.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_tr returned error code {}", ret);

        // Run the Rust implementation
        let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
        let (outputs, _) = rust_tr(&inputs_rust, &[], None).expect("Rust TR indicator failed");

        let output_len_rust = outputs[0].len();

        // Compare the outputs in reverse for the length of the Rust outputs
        for (i, (&c_val, &rust_val)) in tr_output_vec_c
            .iter()
            .rev()
            .take(output_len_rust)
            .zip(outputs[0].iter().rev())
            .enumerate()
        {
            let index = output_len_rust - i - 1;

            // Fail test if Rust has NaN
            if rust_val.is_nan() {
                panic!("Rust TR has NaN at index {}: Rust = {}", index, rust_val);
            }

            // Fail test if Rust has infinity
            if rust_val.is_infinite() {
                panic!(
                    "Rust TR has infinity at index {}: Rust = {}",
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
                // Adjust epsilon if needed
                println!(
                    "Test failed at index {}: \nC = {:?}, \nRust = {:?}",
                    index, tr_output_vec_c, outputs[0]
                );
                panic!(
                    "Mismatch at index {}: C = {}, Rust = {}",
                    index, c_val, rust_val
                );
            }
        }
    }

    #[test]
    fn test_tr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            // run c code
            let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

            // Determine the offset required by the C TR function
            let start_index = unsafe { ti_tr_start(std::ptr::null()) };
            assert!(start_index >= 0, "ti_tr_start returned a negative index");
            let output_len_c = high.len() - (start_index as usize);

            // Run the C implementation
            let mut tr_output_vec_c = vec![0.0_f64; output_len_c];
            let tr_ptr: *mut f64 = tr_output_vec_c.as_mut_ptr();
            let mut outputs_c: Vec<*mut f64> = vec![tr_ptr];
            let ret = unsafe {
                ti_tr(
                    high.len() as i32,
                    inputs_c.as_ptr(),
                    std::ptr::null(),
                    outputs_c.as_mut_ptr(),
                )
            };
            assert_eq!(ret, 0, "ti_tr returned error code {}", ret);

            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (outputs, _) = rust_tr(&inputs_rust, &[], None).expect("Rust TR indicator failed");

            let output_len_rust = outputs[0].len();

            for (i, (&c_val, &rust_val)) in tr_output_vec_c
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
                        "Rust TR has NaN at index {}: Rust = {}, Stock: {}",
                        index, rust_val, stock_symbol
                    );
                }

                // Fail test if Rust has infinity
                if rust_val.is_infinite() {
                    panic!(
                        "Rust TR has infinity at index {}: Rust = {}",
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
                        index, tr_output_vec_c, outputs[0], stock_symbol
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
    fn test_tr_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let options = [];
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Get full output from processing all data at once
            let (full_outputs, _) =
                rust_tr(&inputs_rust, &options, None).expect("Rust TR indicator failed");

            // Process data in batches and accumulate outputs
            let mut batch_full_output = Vec::new();

            let min_data_val = min_data(&options).max(CHUNK_SIZE);

            // First chunk - convert to Vec<&Vec<f64>>
            let high_vec = high[..min_data_val].to_vec();
            let low_vec = low[..min_data_val].to_vec();
            let close_vec = close[..min_data_val].to_vec();
            let chunk_inputs = [
                high_vec.as_slice(),
                low_vec.as_slice(),
                close_vec.as_slice(),
            ];

            let (first_outputs, mut state) =
                rust_tr(&chunk_inputs, &options, None).expect("Rust TR indicator failed");
            batch_full_output.extend_from_slice(&first_outputs[0]);

            // Process remaining data in chunks
            let remaining_len = high.len() - min_data_val;
            let mut chunks_processed = 0;
            while chunks_processed * CHUNK_SIZE < remaining_len {
                let start_idx = min_data_val + chunks_processed * CHUNK_SIZE;
                let end_idx = (start_idx + CHUNK_SIZE).min(high.len());

                let high_vec = high[start_idx..end_idx].to_vec();
                let low_vec = low[start_idx..end_idx].to_vec();
                let close_vec = close[start_idx..end_idx].to_vec();
                let chunk_inputs = [
                    high_vec.as_slice(),
                    low_vec.as_slice(),
                    close_vec.as_slice(),
                ];

                let chunk_outputs = state
                    .batch_indicator(&chunk_inputs, None)
                    .expect("TR batch indicator failed");
                batch_full_output.extend_from_slice(&chunk_outputs[0]);
                chunks_processed += 1;
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

    //#[cfg(feature = "nightly")]
    #[test]
    fn test_tr_simd_vs_regular_database() {
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

        let options = [];

        // Get SIMD by assets result
        let (simd_results, _) =
            rust_tr_simd::<4>(&inputs, &options, None).expect("SIMD by assets TR indicator failed");

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
            let (regular_outputs, _) = rust_tr(&stock_inputs, &options, None)
                .unwrap_or_else(|_| panic!("Regular TR failed for {}", stock_symbol));

            // Compare SIMD result with regular result
            assert_eq!(
                regular_outputs[0].len(),
                simd_results[stock_idx][0].len(),
                "Output length mismatch for stock {}: regular = {}, simd = {}",
                stock_symbol,
                regular_outputs[0].len(),
                simd_results[stock_idx][0].len()
            );

            for (i, (&regular_val, &simd_val)) in regular_outputs[0]
                .iter()
                .zip(simd_results[stock_idx][0].iter())
                .enumerate()
            {
                if !approx_eq!(f64, regular_val, simd_val, epsilon = 1e-12) {
                    panic!(
                        "TR mismatch at index {} for stock {}: regular = {}, simd = {}",
                        i, stock_symbol, regular_val, simd_val
                    );
                }
            }
        }
    }

    fn get_hlc_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }
}
