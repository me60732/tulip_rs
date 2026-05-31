#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::di::indicator_by_assets;
    use tulip_rs::indicators::di::indicator_by_options;
    use tulip_rs::indicators::di::{indicator as rust_di, min_data, TIndicatorState};
    //use tulip_test::c_bindings::{ti_di, ti_di_start};
    use tulip_test::c_bindings::{ti_atr, ti_atr_start, ti_tr, ti_tr_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILON: f64 = 1e-1;
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

    const FULL_PLUS_DI: [f64; 15] = [
        30.79558431671124,
        25.666243654822452,
        33.23390603457425,
        37.76944255790437,
        34.526729019750405,
        38.618792397835215,
        42.1958208009572,
        40.32934631419584,
        49.82090299322741,
        42.28294357902672,
        18.22944009455337,
        15.87732184128535,
        24.387587418025024,
        25.710615254589907,
        30.077131946179456,
    ];
    const FULL_MINUS_DI: [f64; 15] = [
        9.8972211648269,
        20.542512690355505,
        13.73422420193033,
        11.678075146610086,
        9.764735614010249,
        6.827001954175812,
        5.5086962672547255,
        4.415922172035797,
        3.603045788287192,
        5.872805781923429,
        56.76422745314365,
        56.14950608500865,
        44.76967135318073,
        41.31611434971924,
        36.68971949814254,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [30.0]];

    fn get_hlc_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }

    #[test]
    fn test_di_indicator() {
        let mut high_vector = HIGH.to_vec();
        let mut low_vector = LOW.to_vec();
        let mut close_vector = CLOSE.to_vec();
        for _ in 0..20 {
            high_vector.extend_from_slice(&HIGH);
            low_vector.extend_from_slice(&LOW);
            close_vector.extend_from_slice(&CLOSE);
        }
        let inputs = [
            high_vector.as_slice(),  // High prices
            low_vector.as_slice(),   // Low prices
            close_vector.as_slice(), // Close prices
        ];

        let options = [5.0]; // Example period

        // Run the Rust implementation
        let (outputs, _) = rust_di(&inputs, &options, None).expect("Rust DI indicator failed");

        let plus_di_rust = &outputs[0];
        let minus_di_rust = &outputs[1];

        // Compare the outputs
        for (i, (&plus_di, &minus_di)) in FULL_PLUS_DI.iter().zip(FULL_MINUS_DI.iter()).enumerate()
        {
            // Fail test if Rust has NaN
            if plus_di_rust[i].is_nan() {
                panic!(
                    "Rust DI PLUS has NaN at index {}: Rust = {}, Options = {:?}",
                    i, plus_di_rust[i], options
                );
            }

            if minus_di_rust[i].is_nan() {
                panic!(
                    "Rust DI MINUS has NaN at index {}: Rust = {}, Options = {:?}",
                    i, minus_di_rust[i], options
                );
            }

            // Fail test if Rust has infinity
            if plus_di_rust[i].is_infinite() {
                panic!(
                    "Rust DI PLUS has infinity at index {}: Rust = {}",
                    i, plus_di_rust[i]
                );
            }

            if minus_di_rust[i].is_infinite() {
                panic!(
                    "Rust DI MINUS has infinity at index {}: Rust = {}",
                    i, minus_di_rust[i]
                );
            }

            // Skip if only C has NaN (C bug)
            if (plus_di.is_nan() && !plus_di_rust[i].is_nan())
                || (minus_di.is_nan() && !minus_di_rust[i].is_nan())
            {
                continue;
            }

            // Skip if only C has infinity (C bug)
            if (plus_di.is_infinite() && !plus_di_rust[i].is_infinite())
                || (minus_di.is_infinite() && !minus_di_rust[i].is_infinite())
            {
                continue;
            }

            if !approx_eq!(f64, plus_di, plus_di_rust[i], epsilon = EPSILON)
                || !approx_eq!(f64, minus_di, minus_di_rust[i], epsilon = EPSILON)
            {
                println!(
                    "Test failed at index {}: \nFull Plus DI = {:?}, \nRust Plus DI = {:?}, \nFull Minus DI = {:?}, \nRust Minus DI = {:?}",
                    i, FULL_PLUS_DI, plus_di_rust, FULL_MINUS_DI, minus_di_rust
                );
                panic!(
                    "Mismatch at index {}: Full Plus DI = {}, Rust Plus DI = {}, Full Minus DI = {}, Rust Minus DI = {}",
                    i, plus_di, plus_di_rust[i], minus_di, minus_di_rust[i]
                );
            }
        }
    }

    /*#[test]
    fn test_di_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(&stock_data);
            for ((&h, &l), &c) in high.iter().zip(low.iter()).zip(close.iter()) {
                if h.is_nan() || l.is_nan() || c.is_nan() {
                    panic!(
                        "Rust inputs contain NaN at high: {:?}, low: {:?}, close: {:?}",
                        h, l, c
                    );
                }
            }
            for options in OPTIONS_LIST {
                // C implementation
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];

                let start_index = unsafe { ti_di_start(options.as_ptr()) };
                assert!(start_index >= 0, "ti_di_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut plus_di_output_vec_c = vec![0.0_f64; output_len_c];
                let mut minus_di_output_vec_c = vec![0.0_f64; output_len_c];
                let plus_di_ptr: *mut f64 = plus_di_output_vec_c.as_mut_ptr();
                let minus_di_ptr: *mut f64 = minus_di_output_vec_c.as_mut_ptr();
                let mut outputs_c: Vec<*mut f64> = vec![plus_di_ptr, minus_di_ptr];
                let ret = unsafe {
                    ti_di(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    )
                };
                assert_eq!(ret, 0, "ti_di returned error code {}", ret);

                // Rust implementation
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (outputs, _) =
                    rust_di(&inputs_rust, &options, None).expect("Rust DI indicator failed");

                let output_len_rust = outputs[0].len();

                // Compare PLUS_DI results
                for (i, (&c_val, &rust_val)) in plus_di_output_vec_c
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
                            "Rust DI PLUS has NaN at index {}: Rust = {}, Options = {:?}, Stock: {}",
                            index, rust_val, options, stock_symbol
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust DI has infinity at index {}: Rust = {}",
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
                            "DI+ test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, plus_di_output_vec_c, outputs[0], options, stock_symbol
                        );
                        panic!(
                            "DI PLUS mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }

                // Compare MINUS_DI results
                for (i, (&c_val, &rust_val)) in minus_di_output_vec_c
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
                            "Rust DI has NaN at index {}: Rust = {}, Options = {:?}",
                            index, rust_val, options
                        );
                    }

                    // Fail test if Rust has infinity
                    if rust_val.is_infinite() {
                        panic!(
                            "Rust DI has infinity at index {}: Rust = {}",
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
                            "DI- test failed at index {}: \nC = {:?}, \n\nRust = {:?}, Options = {:?}, Stock: {}",
                            index, minus_di_output_vec_c, outputs[1], options, stock_symbol
                        );
                        panic!(
                            "DI MINUS mismatch at index {}: C = {}, Rust = {}, Options = {:?}",
                            index, c_val, rust_val, options
                        );
                    }
                }
            }
        }
    }*/

    #[test]
    fn test_di_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            for options in OPTIONS_LIST {
                let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

                // Get full output
                let (full_outputs, _) =
                    rust_di(&inputs_rust, &options, None).expect("Rust DI indicator failed");

                // Process in batches
                let mut batch_full_outputs = vec![Vec::new(); full_outputs.len()];

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    // If data is too small, just run full calculation
                    let (outputs, _) =
                        rust_di(&inputs_rust, &options, None).expect("Failed to run DI indicator");
                    for (output_idx, output) in outputs.iter().enumerate() {
                        batch_full_outputs[output_idx].extend_from_slice(output);
                    }
                } else {
                    // First chunk - convert to Vec<&Vec<f64>>
                    let high_vec = high[..min_data_val].to_vec();
                    let low_vec = low[..min_data_val].to_vec();
                    let close_vec = close[..min_data_val].to_vec();
                    let chunk_inputs = [
                        high_vec.as_slice(),
                        low_vec.as_slice(),
                        close_vec.as_slice(),
                    ];

                    let (first_outputs, mut state) = rust_di(&chunk_inputs, &options, None)
                        .expect("Failed to run DI indicator on first chunk");
                    for (output_idx, output) in first_outputs.iter().enumerate() {
                        batch_full_outputs[output_idx].extend_from_slice(output);
                    }

                    // Process remaining data in chunks using state
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for ((high_chunk, low_chunk), close_chunk) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                    {
                        let high_vec = high_chunk.to_vec();
                        let low_vec = low_chunk.to_vec();
                        let close_vec = close_chunk.to_vec();
                        let chunk_inputs = [
                            high_vec.as_slice(),
                            low_vec.as_slice(),
                            close_vec.as_slice(),
                        ];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("DI batch indicator failed");
                        for (output_idx, output) in chunk_outputs.iter().enumerate() {
                            batch_full_outputs[output_idx].extend_from_slice(output);
                        }
                    }

                    // Process remainder if any
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();

                    if !high_rem.is_empty() && !low_rem.is_empty() && !close_rem.is_empty() {
                        let high_vec = high_rem.to_vec();
                        let low_vec = low_rem.to_vec();
                        let close_vec = close_rem.to_vec();
                        let chunk_inputs = [
                            high_vec.as_slice(),
                            low_vec.as_slice(),
                            close_vec.as_slice(),
                        ];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("DI batch indicator failed");
                        for (output_idx, output) in chunk_outputs.iter().enumerate() {
                            batch_full_outputs[output_idx].extend_from_slice(output);
                        }
                    }
                }

                // Compare outputs (plus_di and minus_di)
                for output_idx in 0..2 {
                    for (i, (&full_val, &batch_val)) in full_outputs[output_idx]
                        .iter()
                        .zip(batch_full_outputs[output_idx].iter())
                        .enumerate()
                    {
                        assert_eq!(
                            full_val, batch_val,
                            "DI output {} mismatch at index {}: full = {}, batch = {}, options = {:?}, stock = {}",
                            output_idx, i, full_val, batch_val, options, stock_symbol
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_di_atr_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let high = HIGH.to_vec();
        let low = LOW.to_vec();
        let close = CLOSE.to_vec();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let options = [5.0]; // period = 5
        let optional_outputs = Some([true, false].as_slice()); // Request ATR output

        // Get Rust DI output with ATR optional output
        let result = rust_di(&inputs, &options, optional_outputs).unwrap();
        let rust_atr = &result.0[2]; // atr is at index 2

        // Fail fast if Rust output is empty
        if rust_atr.is_empty() {
            panic!("Rust DI ATR optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip ATR output for comparison
        let c_inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
        let c_options = [5.0];
        let c_start_index = unsafe { ti_atr_start(c_options.as_ptr()) } as usize;
        let c_output_len = high.len() - c_start_index;
        let mut c_atr = vec![0.0; c_output_len];
        let mut c_outputs = vec![c_atr.as_mut_ptr()];

        let ret = unsafe {
            ti_atr(
                high.len() as i32,
                c_inputs.as_ptr(),
                c_options.as_ptr(),
                c_outputs.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_atr returned error code {}", ret);

        // Compare ATR outputs from the end backwards (reverse order comparison)
        println!("Comparing DI ATR optional output vs C Tulip ATR:");
        println!(
            "Rust ATR length: {}, C ATR length: {}",
            rust_atr.len(),
            c_atr.len()
        );

        for (i, (rust_val, c_val)) in rust_atr.iter().rev().zip(c_atr.iter().rev()).enumerate() {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust ATR output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            // Skip comparison if C output is NaN/infinite (assume C bug)
            if !c_val.is_finite() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > EPSILON {
                panic!(
                    "DI ATR optional output mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ DI ATR optional output matches C Tulip ATR output");
    }

    #[test]
    fn test_di_tr_optional_output_vs_c_tulip() {
        const EPSILON: f64 = 1e-12;

        let high = HIGH.to_vec();
        let low = LOW.to_vec();
        let close = CLOSE.to_vec();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let options = [5.0]; // period = 5
        let optional_outputs = Some([false, true].as_slice()); // Request TR output

        // Get Rust DI output with TR optional output
        let result = rust_di(&inputs, &options, optional_outputs).unwrap();
        let rust_tr = &result.0[3]; // tr is at index 3

        // Fail fast if Rust output is empty
        if rust_tr.is_empty() {
            panic!("Rust DI TR optional output is empty - this indicates an indicator bug");
        }

        // Get C Tulip TR output for comparison
        let c_inputs: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
        let c_options = [1.0]; // TR doesn't use period, but needs a value
        let c_start_index = unsafe { ti_tr_start(c_options.as_ptr()) } as usize;
        let c_output_len = high.len() - c_start_index;
        let mut c_tr = vec![0.0; c_output_len];
        let mut c_outputs = vec![c_tr.as_mut_ptr()];

        let ret = unsafe {
            ti_tr(
                high.len() as i32,
                c_inputs.as_ptr(),
                c_options.as_ptr(),
                c_outputs.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_tr returned error code {}", ret);

        // Compare TR outputs from the end backwards (reverse order comparison)
        println!("Comparing DI TR optional output vs C Tulip TR:");
        println!(
            "Rust TR length: {}, C TR length: {}",
            rust_tr.len(),
            c_tr.len()
        );

        for (i, (rust_val, c_val)) in rust_tr.iter().rev().zip(c_tr.iter().rev()).enumerate() {
            // Check for NaN/infinity in Rust output (should not happen)
            if !rust_val.is_finite() {
                panic!(
                    "Rust TR output contains NaN/infinity at position {}: {}",
                    i, rust_val
                );
            }

            // Skip comparison if C output is NaN/infinite (assume C bug)
            if !c_val.is_finite() {
                println!(
                    "Skipping comparison at position {} - C output is NaN/infinite: {}",
                    i, c_val
                );
                continue;
            }

            let diff = (rust_val - c_val).abs();
            if diff > EPSILON {
                panic!(
                    "DI TR optional output mismatch at reverse position {}: Rust = {:.12}, C = {:.12}, diff = {:.2e}",
                    i, rust_val, c_val, diff
                );
            }
        }

        println!("✓ DI TR optional output matches C Tulip TR output");
    }

    #[test]
    fn test_di_database_optional_atr() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 20 {
                continue;
            }

            let (high, low, close) = get_hlc_arrays(stock_data);

            for &options in &OPTIONS_LIST {
                // Get DI with ATR optional output
                let optional_outputs = Some(&[true, false][..]);
                let (di_result, _) = tulip_rs::indicators::di::indicator(
                    &[&high, &low, &close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_atr = &di_result[2];

                // Calculate expected ATR using C Tulip ti_atr
                let atr_options = [options[0]]; // period
                let start_index = unsafe { ti_atr_start(atr_options.as_ptr()) };
                assert!(start_index >= 0, "ti_atr_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut c_atr_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_atr_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_atr(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        atr_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_atr failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_atr.len().min(c_atr_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_atr.len() - 1 - i;
                    let c_idx = c_atr_output.len() - 1 - i;

                    let rust_val = rust_atr[rust_idx];
                    let c_val = c_atr_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust ATR output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "DI ATR optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
                        rust_idx,
                        options,
                        rust_val,
                        c_val,
                        (rust_val - c_val).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn test_di_database_optional_tr() {
        const EPSILON: f64 = 1e-12;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (_stock_symbol, stock_data) in data {
            if stock_data.len() < 20 {
                continue;
            }

            let (high, low, close) = get_hlc_arrays(stock_data);

            for &options in &OPTIONS_LIST {
                // Get DI with TR optional output
                let optional_outputs = Some(&[false, true][..]);
                let (di_result, _) = tulip_rs::indicators::di::indicator(
                    &[&high, &low, &close],
                    &[options[0]],
                    optional_outputs,
                )
                .unwrap();

                let rust_tr = &di_result[3];

                // Calculate expected TR using C Tulip ti_tr
                let tr_options = [1.0]; // TR doesn't use period, but needs a value
                let start_index = unsafe { ti_tr_start(tr_options.as_ptr()) };
                assert!(start_index >= 0, "ti_tr_start returned a negative index");
                let output_len_c = high.len() - (start_index as usize);

                let mut c_tr_output = vec![0.0; output_len_c];
                let inputs_c: Vec<*const f64> = vec![high.as_ptr(), low.as_ptr(), close.as_ptr()];
                let mut outputs_c: Vec<*mut f64> = vec![c_tr_output.as_mut_ptr()];

                unsafe {
                    let ret = ti_tr(
                        high.len() as i32,
                        inputs_c.as_ptr(),
                        tr_options.as_ptr(),
                        outputs_c.as_mut_ptr(),
                    );
                    assert_eq!(ret, 0, "ti_tr failed");
                }

                // Compare from most recent values backwards
                let compare_len = rust_tr.len().min(c_tr_output.len());
                for i in 0..compare_len {
                    let rust_idx = rust_tr.len() - 1 - i;
                    let c_idx = c_tr_output.len() - 1 - i;

                    let rust_val = rust_tr[rust_idx];
                    let c_val = c_tr_output[c_idx];

                    if rust_val.is_nan() || rust_val.is_infinite() {
                        panic!(
                            "Rust TR output is NaN or infinite at index {}: {}",
                            rust_idx, rust_val
                        );
                    }

                    if c_val.is_nan() || c_val.is_infinite() {
                        continue; // Skip comparison if C output is invalid
                    }

                    assert!(
                        approx_eq!(f64, rust_val, c_val, epsilon = EPSILON),
                        "DI TR optional output mismatch at index {} (options {:?}): rust={}, c={}, diff={}",
                        rust_idx,
                        options,
                        rust_val,
                        c_val,
                        (rust_val - c_val).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn test_di_simd_vs_regular_database() {
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

        // Test each period
        for options in &OPTIONS_LIST {
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

            // Get SIMD by assets result
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, options, None)
                .expect("SIMD by assets DI indicator failed");

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
                let (regular_outputs, _) = rust_di(&stock_inputs, options, None).unwrap_or_else(|_| panic!("Regular DI failed for {} with period {}",
                    stock_symbol, options[0]));

                // Compare SIMD result with regular result for both plus_di and minus_di
                for output_idx in 0..2 {
                    let output_name = if output_idx == 0 {
                        "plus_di"
                    } else {
                        "minus_di"
                    };

                    assert_eq!(
                        regular_outputs[output_idx].len(),
                        simd_results[stock_idx][output_idx].len(),
                        "{} output length mismatch for stock {} with period {}: regular = {}, simd = {}",
                        output_name,
                        stock_symbol,
                        options[0],
                        regular_outputs[output_idx].len(),
                        simd_results[stock_idx][output_idx].len()
                    );

                    for (i, (&regular_val, &simd_val)) in regular_outputs[output_idx]
                        .iter()
                        .zip(simd_results[stock_idx][output_idx].iter())
                        .enumerate()
                    {
                        if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                            panic!(
                                "{} mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                                output_name, i, stock_symbol, options[0], regular_val, simd_val
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn test_di_simd_vs_regular_database_optional_atr() {
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

        // Test each period with optional ATR output
        for options in &OPTIONS_LIST {
            let optional_outputs = Some([true, false].as_slice()); // Enable ATR only

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

            // Get SIMD by assets result with optional ATR output
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, options, optional_outputs)
                .expect("SIMD by assets DI indicator with optional ATR failed");

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
                let (regular_outputs, _) = rust_di(&stock_inputs, options, optional_outputs)
                    .unwrap_or_else(|_| panic!("Regular DI with optional ATR failed for {} with period {}",
                        stock_symbol, options[0]));

                // Compare number of outputs (should be 3: plus_di, minus_di, atr)
                assert_eq!(
                    regular_outputs.len(),
                    simd_results[stock_idx].len(),
                    "Number of outputs mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs.len(),
                    simd_results[stock_idx].len()
                );

                // Compare plus_di output (index 0)
                assert_eq!(
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len(),
                    "Plus DI output length mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[0]
                    .iter()
                    .zip(simd_results[stock_idx][0].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "Plus DI mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }

                // Compare minus_di output (index 1)
                assert_eq!(
                    regular_outputs[1].len(),
                    simd_results[stock_idx][1].len(),
                    "Minus DI output length mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs[1].len(),
                    simd_results[stock_idx][1].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[1]
                    .iter()
                    .zip(simd_results[stock_idx][1].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "Minus DI mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }

                // Compare ATR output (index 2)
                assert_eq!(
                    regular_outputs[2].len(),
                    simd_results[stock_idx][2].len(),
                    "ATR output length mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs[2].len(),
                    simd_results[stock_idx][2].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[2]
                    .iter()
                    .zip(simd_results[stock_idx][2].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "ATR mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_di_simd_vs_regular_database_optional_tr() {
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

        // Test each period with optional TR output
        for options in &OPTIONS_LIST {
            let optional_outputs = Some([false, true].as_slice()); // Enable TR only

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

            // Get SIMD by assets result with optional TR output
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, options, optional_outputs)
                .expect("SIMD by assets DI indicator with optional TR failed");

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
                let (regular_outputs, _) = rust_di(&stock_inputs, options, optional_outputs)
                    .unwrap_or_else(|_| panic!("Regular DI with optional TR failed for {} with period {}",
                        stock_symbol, options[0]));

                // Compare number of outputs (should be 3: plus_di, minus_di, tr)
                assert_eq!(
                    regular_outputs.len(),
                    simd_results[stock_idx].len(),
                    "Number of outputs mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs.len(),
                    simd_results[stock_idx].len()
                );

                // Compare plus_di output (index 0)
                assert_eq!(
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len(),
                    "Plus DI output length mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs[0].len(),
                    simd_results[stock_idx][0].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[0]
                    .iter()
                    .zip(simd_results[stock_idx][0].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "Plus DI mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }

                // Compare minus_di output (index 1)
                assert_eq!(
                    regular_outputs[1].len(),
                    simd_results[stock_idx][1].len(),
                    "Minus DI output length mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs[1].len(),
                    simd_results[stock_idx][1].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[1]
                    .iter()
                    .zip(simd_results[stock_idx][1].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "Minus DI mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }

                // Compare TR output (index 2)
                assert_eq!(
                    regular_outputs[2].len(),
                    simd_results[stock_idx][2].len(),
                    "TR output length mismatch for stock {} with period {}: regular = {}, simd = {}",
                    stock_symbol,
                    options[0],
                    regular_outputs[2].len(),
                    simd_results[stock_idx][2].len()
                );

                for (i, (&regular_val, &simd_val)) in regular_outputs[2]
                    .iter()
                    .zip(simd_results[stock_idx][2].iter())
                    .enumerate()
                {
                    if !approx_eq!(f64, regular_val, simd_val, epsilon = EPSILON) {
                        panic!(
                            "TR mismatch at index {} for stock {} with period {}: regular = {}, simd = {}",
                            i, stock_symbol, options[0], regular_val, simd_val
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_di_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD DI 4-wide failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result
                let (regular_results, _) =
                    rust_di(&inputs, options, None).expect("Regular DI indicator failed");

                let simd_plus_di_result = &all_simd_results[idx][0];
                let regular_plus_di_result = &regular_results[0];

                let simd_minus_di_result = &all_simd_results[idx][1];
                let regular_minus_di_result = &regular_results[1];

                // Compare Plus DI output lengths
                assert_eq!(
                    simd_plus_di_result.len(),
                    regular_plus_di_result.len(),
                    "Plus DI output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_plus_di_result.len(),
                    regular_plus_di_result.len()
                );

                // Compare Minus DI output lengths
                assert_eq!(
                    simd_minus_di_result.len(),
                    regular_minus_di_result.len(),
                    "Minus DI output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_minus_di_result.len(),
                    regular_minus_di_result.len()
                );

                // Compare Plus DI values
                for (i, (&simd_val, &regular_val)) in simd_plus_di_result
                    .iter()
                    .zip(regular_plus_di_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Plus DI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Plus DI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Plus DI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Minus DI values
                for (i, (&simd_val, &regular_val)) in simd_minus_di_result
                    .iter()
                    .zip(regular_minus_di_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Minus DI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Minus DI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Minus DI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!("✓ All SIMD by options vs Regular DI database tests passed!");
    }

    #[test]
    fn test_di_simd_by_options_vs_regular_database_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            // Test with ATR and TR optional outputs
            let optional_outputs = Some(&[true, true][..]);

            // Process all 4 options with 4-wide SIMD
            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results_4, _) =
                indicator_by_options::<4>(&inputs, &options_4, optional_outputs)
                    .expect("SIMD DI 4-wide with optional outputs failed");

            // Use SIMD results directly
            let all_simd_results = simd_results_4;

            // Compare each SIMD result with regular indicator
            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                // Get regular indicator result with optional outputs
                let (regular_results, _) = rust_di(&inputs, options, optional_outputs)
                    .expect("Regular DI indicator with optional outputs failed");

                let simd_plus_di_result = &all_simd_results[idx][0];
                let regular_plus_di_result = &regular_results[0];

                let simd_minus_di_result = &all_simd_results[idx][1];
                let regular_minus_di_result = &regular_results[1];

                let simd_atr_result = &all_simd_results[idx][2];
                let regular_atr_result = &regular_results[2];

                let simd_tr_result = &all_simd_results[idx][3];
                let regular_tr_result = &regular_results[3];

                // Compare Plus DI output lengths
                assert_eq!(
                    simd_plus_di_result.len(),
                    regular_plus_di_result.len(),
                    "Plus DI output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_plus_di_result.len(),
                    regular_plus_di_result.len()
                );

                // Compare Minus DI output lengths
                assert_eq!(
                    simd_minus_di_result.len(),
                    regular_minus_di_result.len(),
                    "Minus DI output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_minus_di_result.len(),
                    regular_minus_di_result.len()
                );

                // Compare ATR output lengths
                assert_eq!(
                    simd_atr_result.len(),
                    regular_atr_result.len(),
                    "ATR output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_atr_result.len(),
                    regular_atr_result.len()
                );

                // Compare TR output lengths
                assert_eq!(
                    simd_tr_result.len(),
                    regular_tr_result.len(),
                    "TR output length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_tr_result.len(),
                    regular_tr_result.len()
                );

                // Compare Plus DI values
                for (i, (&simd_val, &regular_val)) in simd_plus_di_result
                    .iter()
                    .zip(regular_plus_di_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Plus DI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Plus DI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Plus DI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare Minus DI values
                for (i, (&simd_val, &regular_val)) in simd_minus_di_result
                    .iter()
                    .zip(regular_minus_di_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD Minus DI has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD Minus DI has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = EPSILON) {
                        panic!(
                            "Minus DI mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare ATR values
                const ATR_EPSILON: f64 = 1e-12;
                for (i, (&simd_val, &regular_val)) in simd_atr_result
                    .iter()
                    .zip(regular_atr_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD ATR has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD ATR has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = ATR_EPSILON) {
                        panic!(
                            "ATR mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                // Compare TR values
                const TR_EPSILON: f64 = 1e-12;
                for (i, (&simd_val, &regular_val)) in simd_tr_result
                    .iter()
                    .zip(regular_tr_result.iter())
                    .enumerate()
                {
                    // Check for NaN/infinity in SIMD result
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD TR has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    if simd_val.is_infinite() {
                        panic!(
                            "SIMD TR has infinity at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }

                    // Compare values with tolerance
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = TR_EPSILON) {
                        panic!(
                            "TR mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }
        }

        println!(
            "✓ All SIMD by options vs Regular DI database tests with optional outputs passed!"
        );
    }
}
