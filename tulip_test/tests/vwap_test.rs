#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::vwap::{indicator as rust_vwap, min_data, TIndicatorState};
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
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    const VOLUME: [f64; 15] = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ];

    // VWAP takes no options.
    const OPTIONS: [f64; 0] = [];

    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        let mut volume_vec = VOLUME.to_vec();
        for _ in 0..3 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
            volume_vec.extend_from_slice(&VOLUME);
        }
        (high_vec, low_vec, close_vec, volume_vec)
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

    // -------------------------------------------------------------------------
    // Basic correctness: all outputs must be finite and monotonically
    // non-decreasing in the denominator direction (vwap must be finite).
    // -------------------------------------------------------------------------
    #[test]
    fn test_vwap_indicator() {
        let (high, low, close, volume) = expand_inputs();
        let inputs = [
            high.as_slice(),
            low.as_slice(),
            close.as_slice(),
            volume.as_slice(),
        ];

        let (outputs, _) = rust_vwap(&inputs, &OPTIONS, None).expect("Rust VWAP indicator failed");

        for (i, &val) in outputs[0].iter().enumerate() {
            if val.is_nan() {
                panic!("VWAP has NaN at index {}", i);
            }
            if val.is_infinite() {
                panic!("VWAP has Inf at index {}: val={}", i, val);
            }
        }

        println!("✓ VWAP indicator ok: {} output values", outputs[0].len());
    }

    // -------------------------------------------------------------------------
    // Optional-output test:
    //   The `typprice` optional output produced by VWAP must exactly match
    //   the output of the standalone `typprice` indicator on the same H/L/C.
    // -------------------------------------------------------------------------
    #[test]
    fn test_vwap_optional_outputs() {
        use tulip_rs::indicators::typprice::indicator as rust_typprice;

        let (high, low, close, volume) = expand_inputs();
        let inputs = [
            high.as_slice(),
            low.as_slice(),
            close.as_slice(),
            volume.as_slice(),
        ];

        // Run VWAP with typprice optional output enabled.
        let (vwap_outputs, _) =
            rust_vwap(&inputs, &OPTIONS, Some(&[true])).expect("VWAP with optional outputs failed");

        let vwap_tp = &vwap_outputs[1];

        // Standalone typprice on the same H/L/C.
        let typprice_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let (tp_outputs, _) =
            rust_typprice(&typprice_inputs, &[], None).expect("Standalone typprice failed");
        let standalone_tp = &tp_outputs[0];

        assert_eq!(
            vwap_tp.len(),
            standalone_tp.len(),
            "typprice length mismatch: vwap_tp.len()={}, standalone_tp.len()={}",
            vwap_tp.len(),
            standalone_tp.len()
        );

        for (i, (&vwap_val, &tp_val)) in vwap_tp.iter().zip(standalone_tp.iter()).enumerate() {
            assert!(
                vwap_val.is_finite(),
                "VWAP typprice optional output is not finite at index {}: {}",
                i,
                vwap_val
            );
            if !approx_eq!(f64, vwap_val, tp_val, epsilon = 1e-12) {
                panic!(
                    "typprice mismatch at index {}: vwap={}, standalone={}",
                    i, vwap_val, tp_val
                );
            }
        }

        println!(
            "✓ VWAP typprice optional output matches standalone typprice: len={}",
            vwap_tp.len()
        );
    }

    // -------------------------------------------------------------------------
    // Database test: VWAP must produce all-finite output on real OHLCV data.
    // -------------------------------------------------------------------------
    #[test]
    fn test_vwap_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            let (outputs, _) =
                rust_vwap(&inputs, &OPTIONS, None).expect("Rust VWAP indicator failed");

            for (i, &val) in outputs[0].iter().enumerate() {
                if val.is_nan() || val.is_infinite() {
                    panic!(
                        "VWAP has NaN/Inf at index {}: val={}, stock={}",
                        i, val, stock_symbol
                    );
                }
            }

            println!("✓ VWAP database test passed for stock {}", stock_symbol);
        }
    }

    // -------------------------------------------------------------------------
    // Database optional-outputs test: typprice must match standalone indicator
    // on every stock in the database.
    // -------------------------------------------------------------------------
    #[test]
    fn test_vwap_database_optional_outputs() {
        use tulip_rs::indicators::typprice::indicator as rust_typprice;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            // VWAP with typprice optional output.
            let (vwap_outputs, _) = rust_vwap(&inputs, &OPTIONS, Some(&[true]))
                .expect("VWAP with optional outputs failed");
            let vwap_tp = &vwap_outputs[1];

            // Standalone typprice.
            let typprice_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (tp_outputs, _) =
                rust_typprice(&typprice_inputs, &[], None).expect("Standalone typprice failed");
            let standalone_tp = &tp_outputs[0];

            assert_eq!(
                vwap_tp.len(),
                standalone_tp.len(),
                "typprice length mismatch: stock={}, vwap_tp={}, standalone={}",
                stock_symbol,
                vwap_tp.len(),
                standalone_tp.len()
            );

            for (i, (&vwap_val, &tp_val)) in vwap_tp.iter().zip(standalone_tp.iter()).enumerate() {
                if !vwap_val.is_finite() {
                    panic!(
                        "VWAP typprice not finite at index {}: stock={}",
                        i, stock_symbol
                    );
                }
                if !approx_eq!(f64, vwap_val, tp_val, epsilon = 1e-12) {
                    panic!(
                        "typprice mismatch at index {}: vwap={}, standalone={}, stock={}",
                        i, vwap_val, tp_val, stock_symbol
                    );
                }
            }

            println!(
                "✓ VWAP database optional outputs test passed for stock {}",
                stock_symbol
            );
        }

        println!("✓ All VWAP database optional output tests passed!");
    }

    // -------------------------------------------------------------------------
    // State-continuation test: chunked batch_indicator output must be
    // bit-for-bit identical to a single full-dataset indicator() call.
    // -------------------------------------------------------------------------
    #[test]
    fn test_vwap_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close, volume) = get_hlcv_arrays(stock_data);
            let inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];

            // Full output in one shot.
            let (full_outputs, _) =
                rust_vwap(&inputs, &OPTIONS, None).expect("VWAP full run failed");

            let min_data_val = min_data(&OPTIONS).max(CHUNK_SIZE);
            let mut batch_full_output: Vec<f64> = Vec::new();

            if high.len() <= min_data_val {
                let (outputs, _) =
                    rust_vwap(&inputs, &OPTIONS, None).expect("VWAP indicator failed");
                batch_full_output.extend_from_slice(&outputs[0]);
            } else {
                // First chunk — establishes the state.
                let chunk_inputs = [
                    &high[..min_data_val],
                    &low[..min_data_val],
                    &close[..min_data_val],
                    &volume[..min_data_val],
                ];
                let (first_outputs, mut state) =
                    rust_vwap(&chunk_inputs, &OPTIONS, None).expect("VWAP first chunk failed");
                batch_full_output.extend_from_slice(&first_outputs[0]);

                // Remaining full-size chunks.
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
                    let chunk_inputs = [high_chunk, low_chunk, close_chunk, volume_chunk];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VWAP batch_indicator failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }

                // Remainder (final partial chunk).
                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                let volume_rem = volume_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_inputs = [high_rem, low_rem, close_rem, volume_rem];
                    let chunk_outputs = state
                        .batch_indicator(&chunk_inputs, None)
                        .expect("VWAP batch_indicator remainder failed");
                    batch_full_output.extend_from_slice(&chunk_outputs[0]);
                }
            }

            assert_eq!(
                full_outputs[0].len(),
                batch_full_output.len(),
                "Output length mismatch: stock={}, full={}, batch={}",
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
                    "VWAP state mismatch at index {}: full={}, batch={}, stock={}",
                    i, full_val, batch_val, stock_symbol
                );
            }

            println!(
                "✓ VWAP database state test passed for stock {}",
                stock_symbol
            );
        }
    }

    // =========================================================================
    // SIMD by-assets: outputs match scalar VWAP (database)
    // =========================================================================

    #[test]
    #[cfg(feature = "simd_assets")]
    fn test_vwap_simd_by_assets_vs_regular_database() {
        use tulip_rs::indicators::vwap::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Collect the first 4 stocks into owned vecs so we can borrow slices.
        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (h, l, c, v) = get_hlcv_arrays(eod);
                (symbol.clone(), h, l, c, v)
            })
            .collect();

        let asset0: [&[f64]; 4] = [
            &stock_data[0].1,
            &stock_data[0].2,
            &stock_data[0].3,
            &stock_data[0].4,
        ];
        let asset1: [&[f64]; 4] = [
            &stock_data[1].1,
            &stock_data[1].2,
            &stock_data[1].3,
            &stock_data[1].4,
        ];
        let asset2: [&[f64]; 4] = [
            &stock_data[2].1,
            &stock_data[2].2,
            &stock_data[2].3,
            &stock_data[2].4,
        ];
        let asset3: [&[f64]; 4] = [
            &stock_data[3].1,
            &stock_data[3].2,
            &stock_data[3].3,
            &stock_data[3].4,
        ];
        let inputs_4: [&[&[f64]; 4]; 4] = [&asset0, &asset1, &asset2, &asset3];

        let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &OPTIONS, None)
            .expect("SIMD by-assets VWAP failed");

        for (asset_idx, (stock_symbol, high, low, close, volume)) in stock_data.iter().enumerate() {
            let scalar_inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];
            let (scalar_outputs, _) =
                rust_vwap(&scalar_inputs, &OPTIONS, None).expect("Rust VWAP failed");

            let simd_vwap = &simd_results[asset_idx][0];
            let scalar_vwap = &scalar_outputs[0];

            assert_eq!(
                simd_vwap.len(),
                scalar_vwap.len(),
                "vwap length mismatch: stock={stock_symbol}"
            );
            for (i, (&sv, &rv)) in simd_vwap.iter().zip(scalar_vwap.iter()).enumerate() {
                assert_eq!(
                    sv, rv,
                    "vwap mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}"
                );
            }

            println!("✓ SIMD by-assets vs scalar VWAP ok: stock={stock_symbol}");
        }
    }

    // =========================================================================
    // SIMD by-assets: first FIRST_CHUNK bars via SIMD, rest via batch_indicator,
    // final output must match a single full-dataset scalar indicator() call.
    // =========================================================================

    #[test]
    #[cfg(feature = "simd_assets")]
    fn test_vwap_simd_by_assets_state_continuity() {
        use tulip_rs::indicators::vwap::indicator_by_assets;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (h, l, c, v) = get_hlcv_arrays(eod);
                (symbol.clone(), h, l, c, v)
            })
            .collect();

        let asset0: [&[f64]; 4] = [
            &stock_data[0].1[..FIRST_CHUNK],
            &stock_data[0].2[..FIRST_CHUNK],
            &stock_data[0].3[..FIRST_CHUNK],
            &stock_data[0].4[..FIRST_CHUNK],
        ];
        let asset1: [&[f64]; 4] = [
            &stock_data[1].1[..FIRST_CHUNK],
            &stock_data[1].2[..FIRST_CHUNK],
            &stock_data[1].3[..FIRST_CHUNK],
            &stock_data[1].4[..FIRST_CHUNK],
        ];
        let asset2: [&[f64]; 4] = [
            &stock_data[2].1[..FIRST_CHUNK],
            &stock_data[2].2[..FIRST_CHUNK],
            &stock_data[2].3[..FIRST_CHUNK],
            &stock_data[2].4[..FIRST_CHUNK],
        ];
        let asset3: [&[f64]; 4] = [
            &stock_data[3].1[..FIRST_CHUNK],
            &stock_data[3].2[..FIRST_CHUNK],
            &stock_data[3].3[..FIRST_CHUNK],
            &stock_data[3].4[..FIRST_CHUNK],
        ];
        let inputs_4: [&[&[f64]; 4]; 4] = [&asset0, &asset1, &asset2, &asset3];

        let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_4, &OPTIONS, None)
            .expect("SIMD by-assets VWAP failed on first chunk");

        for (asset_idx, (stock_symbol, high, low, close, volume)) in stock_data.iter().enumerate() {
            // Start from SIMD first-chunk output, then continue with batch_indicator.
            let mut batch_vwap = simd_first[asset_idx][0].clone();

            let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
            let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
            let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
            let mut volume_chunks = volume[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

            for (((hc, lc), cc), vc) in high_chunks
                .by_ref()
                .zip(low_chunks.by_ref())
                .zip(close_chunks.by_ref())
                .zip(volume_chunks.by_ref())
            {
                let chunk_outputs = states[asset_idx]
                    .batch_indicator(&[hc, lc, cc, vc], None)
                    .expect("batch_indicator failed");
                batch_vwap.extend_from_slice(&chunk_outputs[0]);
            }

            let high_rem = high_chunks.remainder();
            let low_rem = low_chunks.remainder();
            let close_rem = close_chunks.remainder();
            let volume_rem = volume_chunks.remainder();
            if !high_rem.is_empty() {
                let chunk_outputs = states[asset_idx]
                    .batch_indicator(&[high_rem, low_rem, close_rem, volume_rem], None)
                    .expect("batch_indicator failed on remainder");
                batch_vwap.extend_from_slice(&chunk_outputs[0]);
            }

            // Full scalar run for comparison.
            let scalar_inputs = [
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                volume.as_slice(),
            ];
            let (scalar_outputs, _) =
                rust_vwap(&scalar_inputs, &OPTIONS, None).expect("Rust VWAP failed");

            assert_eq!(
                batch_vwap.len(),
                scalar_outputs[0].len(),
                "vwap length mismatch: stock={stock_symbol}"
            );
            for (i, (&bv, &rv)) in batch_vwap.iter().zip(scalar_outputs[0].iter()).enumerate() {
                assert_eq!(
                    bv, rv,
                    "vwap mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}"
                );
            }

            println!("✓ SIMD by-assets state continuity ok: stock={stock_symbol}");
        }
    }
}
