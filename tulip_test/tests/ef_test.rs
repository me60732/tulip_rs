#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::ef::{indicator as rust_ef, min_data, TIndicatorState};
    use tulip_rs::indicators::kama::indicator as rust_kama;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [10.0], [14.0], [20.0]];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    fn expand_close() -> Vec<f64> {
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..3 {
            close_vec.extend_from_slice(&CLOSE);
        }
        close_vec
    }

    /// Verify that the standalone EF indicator produces values identical to the
    /// optional `ef` output of the KAMA indicator (KAMA calls ef::calc internally).
    #[test]
    fn test_ef_vs_kama_optional() {
        let close = expand_close();
        let inputs = [close.as_slice()];

        for options in OPTIONS_LIST {
            // Run EF directly
            let (ef_outputs, _) =
                rust_ef(&inputs, &options, None).expect("Rust EF indicator failed");
            let ef_result = &ef_outputs[0];

            // Run KAMA with optional EF output enabled (outputs[1] = ef line)
            let (kama_outputs, _) = rust_kama(&inputs, &options, Some(&[true]))
                .expect("Rust KAMA indicator (with EF optional) failed");
            let kama_ef_result = &kama_outputs[1];

            assert_eq!(
                ef_result.len(),
                kama_ef_result.len(),
                "Output length mismatch with options {:?}: EF={}, KAMA_EF={}",
                options,
                ef_result.len(),
                kama_ef_result.len()
            );

            for (i, (&ef_val, &kama_ef_val)) in
                ef_result.iter().zip(kama_ef_result.iter()).enumerate()
            {
                if ef_val.is_nan() {
                    panic!("EF has NaN at index {} with options {:?}", i, options);
                }
                if !approx_eq!(f64, ef_val, kama_ef_val, epsilon = 1e-12) {
                    panic!(
                        "Mismatch at index {} with options {:?}: EF = {}, KAMA_EF = {}",
                        i, options, ef_val, kama_ef_val
                    );
                }
            }
        }
    }

    /// Same cross-validation against KAMA's optional EF output, run over real
    /// database stocks.
    #[test]
    fn test_ef_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Run EF directly
                let (ef_outputs, _) =
                    rust_ef(&inputs, &options, None).expect("Rust EF indicator failed");
                let ef_result = &ef_outputs[0];

                // Run KAMA with optional EF output
                let (kama_outputs, _) = rust_kama(&inputs, &options, Some(&[true]))
                    .expect("Rust KAMA indicator (with EF optional) failed");
                let kama_ef_result = &kama_outputs[1];

                assert_eq!(
                    ef_result.len(),
                    kama_ef_result.len(),
                    "Output length mismatch for stock {} with options {:?}: EF={}, KAMA_EF={}",
                    stock_symbol,
                    options,
                    ef_result.len(),
                    kama_ef_result.len()
                );

                for (i, (&ef_val, &kama_ef_val)) in
                    ef_result.iter().zip(kama_ef_result.iter()).enumerate()
                {
                    if ef_val.is_nan() {
                        panic!(
                            "EF has NaN at index {} for stock {} with options {:?}",
                            i, stock_symbol, options
                        );
                    }
                    if !approx_eq!(f64, ef_val, kama_ef_val, epsilon = 1e-12) {
                        println!(
                            "EF Line: {:?}, \n\nKAMA_EF Line: {:?}",
                            ef_result, kama_ef_result
                        );
                        panic!(
                            "Mismatch at index {} for stock {} with options {:?}: EF = {}, KAMA_EF = {}",
                            i, stock_symbol, options, ef_val, kama_ef_val
                        );
                    }
                }
            }
        }
    }

    /// Verify that chunked state processing produces the same output as a
    /// single full-dataset run of the EF indicator.
    #[test]
    fn test_ef_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Full run
                let (full_outputs, _) =
                    rust_ef(&inputs, &options, None).expect("EF full run failed");

                // Chunked run
                let mut batch_output: Vec<f64> = Vec::new();
                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                let chunk_inputs = [&close[..min_data_val]];
                let (first_outputs, mut state) =
                    rust_ef(&chunk_inputs, &options, None).expect("EF first chunk failed");
                batch_output.extend_from_slice(&first_outputs[0]);

                let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);
                for close_chunk in close_chunks.by_ref() {
                    let chunk_inputs = [close_chunk];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("EF batch_indicator failed");
                    batch_output.extend_from_slice(&chunk_outputs[0]);
                }

                let remainder = close_chunks.remainder();
                if !remainder.is_empty() {
                    let chunk_inputs = [remainder];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("EF batch_indicator (remainder) failed");
                    batch_output.extend_from_slice(&chunk_outputs[0]);
                }

                assert_eq!(
                    full_outputs[0].len(),
                    batch_output.len(),
                    "Output length mismatch for stock {} with options {:?}: full={}, batch={}",
                    stock_symbol,
                    options,
                    full_outputs[0].len(),
                    batch_output.len()
                );

                for (i, (&full_val, &batch_val)) in
                    full_outputs[0].iter().zip(batch_output.iter()).enumerate()
                {
                    if !approx_eq!(f64, full_val, batch_val, epsilon = 1e-12) {
                        panic!(
                            "State mismatch at index {} for stock {} with options {:?}: full = {}, batch = {}",
                            i, stock_symbol, options, full_val, batch_val
                        );
                    }
                }
            }
        }
    }

    /// Verify that the SIMD by-assets EF variant produces identical output to
    /// the scalar EF indicator for each of the first four database stocks.
    #[test]
    fn test_ef_simd_vs_regular_database() {
        use tulip_rs::indicators::ef::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, data)| (symbol.clone(), get_close_array(data)))
            .collect();

        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            let (simd_results, _) = indicator_by_assets::<4>(&inputs, &options, None)
                .expect("SIMD by assets EF failed");

            for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
                let stock_inputs = [stock_close.as_slice()];
                let (regular_results, _) =
                    rust_ef(&stock_inputs, &options, None).expect("Regular EF failed");

                let simd_result = &simd_results[stock_idx][0];
                let regular_result = &regular_results[0];

                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "Length mismatch for stock {} with options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result.len(),
                    regular_result.len()
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by assets EF has NaN at index {} for stock {} with options {:?}",
                            i, stock_symbol, options
                        );
                    }
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        panic!(
                            "Mismatch at index {} for stock {} with options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }

                println!(
                    "✓ SIMD by assets vs Regular passed for stock {} with options {:?}",
                    stock_symbol, options
                );
            }
        }

        println!("✓ All SIMD by assets vs Regular EF database tests passed!");
    }

    /// Verify that the SIMD by-options EF variant produces identical output to
    /// scalar EF for each option set, across all database stocks.
    #[test]
    fn test_ef_simd_by_options_vs_regular_database() {
        use tulip_rs::indicators::ef::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by options EF failed");

            for (idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    rust_ef(&inputs, options, None).expect("Regular EF failed");

                let simd_result = &simd_results[idx][0];
                let regular_result = &regular_results[0];

                assert_eq!(
                    simd_result.len(),
                    regular_result.len(),
                    "Length mismatch for stock {} options {:?}: SIMD={}, Regular={}",
                    stock_symbol,
                    options,
                    simd_result.len(),
                    regular_result.len()
                );

                for (i, (&simd_val, &regular_val)) in
                    simd_result.iter().zip(regular_result.iter()).enumerate()
                {
                    if simd_val.is_nan() {
                        panic!(
                            "SIMD by options EF has NaN at index {} for stock {}: SIMD = {}, Options = {:?}",
                            i, stock_symbol, simd_val, options
                        );
                    }
                    if !approx_eq!(f64, simd_val, regular_val, epsilon = 1e-12) {
                        panic!(
                            "Mismatch at index {} for stock {} options {:?}: SIMD = {}, Regular = {}",
                            i, stock_symbol, options, simd_val, regular_val
                        );
                    }
                }
            }

            println!(
                "✓ SIMD by options vs Regular passed for stock {}",
                stock_symbol
            );
        }

        println!("✓ All SIMD by options vs Regular EF database tests passed!");
    }
}
