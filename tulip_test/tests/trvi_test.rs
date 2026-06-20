#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::trvi::{
        indicator as rust_trvi, indicator_by_assets, indicator_by_options, min_data,
        TIndicatorState,
    };
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

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [14.0], [20.0], [30.0]];

    // -------------------------------------------------------------------------
    // Compare TRVI (uses TR) against CVI (uses H-L) on real database data.
    // On data with overnight gaps TR != H-L, so the outputs will differ —
    // this test only verifies that both indicators produce the same output
    // length and all-finite values.
    // -------------------------------------------------------------------------
    #[test]
    fn test_trvi_vs_cvi() {
        use tulip_rs::indicators::cvi::indicator as rust_cvi;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let n = high.len();

            for options in OPTIONS_LIST {
                if n < min_data(&options) {
                    continue;
                }

                let trvi_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let cvi_inputs = [high.as_slice(), low.as_slice()];

                let (trvi_outputs, _) =
                    rust_trvi(&trvi_inputs, &options, None).expect("TRVI failed");
                let (cvi_outputs, _) = rust_cvi(&cvi_inputs, &options, None).expect("CVI failed");

                let trvi_out = &trvi_outputs[0];
                let cvi_out = &cvi_outputs[0];

                assert_eq!(
                    trvi_out.len(),
                    cvi_out.len(),
                    "Output length mismatch: stock={}, options={:?}",
                    stock_symbol,
                    options
                );

                for (i, (&tv, &cv)) in trvi_out.iter().zip(cvi_out.iter()).enumerate() {
                    assert!(
                        tv.is_finite(),
                        "TRVI non-finite at index {}: {}, stock={}, options={:?}",
                        i,
                        tv,
                        stock_symbol,
                        options
                    );
                    assert!(
                        cv.is_finite(),
                        "CVI non-finite at index {}: {}, stock={}, options={:?}",
                        i,
                        cv,
                        stock_symbol,
                        options
                    );
                    /*let diff = (tv - cv).abs();
                    if diff > 1e-10 {
                        let start = i.saturating_sub(3);
                        let end = (i + 4).min(trvi_out.len());
                        println!("  [{stock_symbol}] options={options:?} index {i}: trvi={tv:.10}, cvi={cv:.10}, diff={diff:.2e}");
                        println!("    trvi[{start}..{end}]: {:?}", &trvi_out[start..end]);
                        println!("     cvi[{start}..{end}]: {:?}", &cvi_out[start..end]);
                    }*/
                }
            }

            println!(
                "✓ TRVI vs CVI database test passed for stock {}",
                stock_symbol
            );
        }
    }

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

    fn get_hlc_arrays(
        stock_data: &[tulip_test::database::EodData],
    ) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }

    #[test]
    fn test_trvi_indicator() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let (outputs, _) =
                rust_trvi(&inputs, &options, None).expect("Rust TRVI indicator failed");

            for (i, &val) in outputs[0].iter().enumerate() {
                if val.is_nan() {
                    panic!("Rust TRVI has NaN at index {}: options={:?}", i, options);
                }
                if val.is_infinite() {
                    panic!(
                        "Rust TRVI has infinity at index {}: val={}, options={:?}",
                        i, val, options
                    );
                }
            }

            println!(
                "✓ TRVI indicator ok: {} output values, options={:?}",
                outputs[0].len(),
                options
            );
        }
    }

    // -------------------------------------------------------------------------
    // Optional-output test (the core test requested):
    //   1. TR optional output length matches standalone rust_tr output length.
    //   2. TR optional output values match standalone rust_tr values.
    //   3. Feeding the standalone rust_tr results into rust_ema produces an EMA
    //      output whose length and values match the TRVI EMA optional output.
    // -------------------------------------------------------------------------
    #[test]
    fn test_trvi_optional_outputs() {
        use tulip_rs::indicators::ema::indicator as rust_ema;
        use tulip_rs::indicators::tr::indicator as rust_tr;

        let (high, low, close) = expand_inputs();
        let n = high.len();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            if n < min_data(&options) {
                continue;
            }

            // Run TRVI with both optional outputs enabled.
            let (trvi_outputs, _) = rust_trvi(&inputs, &options, Some(&[true, true]))
                .expect("TRVI with optional outputs failed");

            let trvi_tr = &trvi_outputs[1];
            let trvi_ema = &trvi_outputs[2];

            // ── Part 1: TR optional output vs standalone rust_tr ──────────────

            // Standalone TR (no options).
            let (tr_outputs, _) = rust_tr(&inputs, &[], None).expect("Rust TR indicator failed");
            let standalone_tr = &tr_outputs[0];

            assert_eq!(
                trvi_tr.len(),
                standalone_tr.len(),
                "TR length mismatch: options={:?}, trvi_tr.len()={}, standalone_tr.len()={}",
                options,
                trvi_tr.len(),
                standalone_tr.len()
            );

            for (i, (&trvi_val, &tr_val)) in trvi_tr.iter().zip(standalone_tr.iter()).enumerate() {
                assert!(
                    trvi_val.is_finite(),
                    "TRVI TR optional output is not finite at index {}: {}, options={:?}",
                    i,
                    trvi_val,
                    options
                );
                if !approx_eq!(f64, trvi_val, tr_val, epsilon = 1e-12) {
                    panic!(
                        "TR value mismatch at index {}: trvi={}, standalone={}, options={:?}",
                        i, trvi_val, tr_val, options
                    );
                }
            }
            println!(
                "✓ TR optional output matches standalone Rust TR: len={}, options={:?}",
                trvi_tr.len(),
                options
            );

            // ── Part 2: EMA optional output vs rust_ema(standalone TR) ────────

            // Feed standalone TR into rust_ema using the same period.
            let (ema_outputs, _) = rust_ema(&[standalone_tr.as_slice()], &options, None)
                .expect("Rust EMA(TR) indicator failed");
            let ema_of_tr = &ema_outputs[0];

            assert_eq!(
                trvi_ema.len(),
                ema_of_tr.len(),
                "EMA length mismatch: options={:?}, trvi_ema.len()={}, ema_of_tr.len()={}",
                options,
                trvi_ema.len(),
                ema_of_tr.len()
            );

            for (i, (&trvi_val, &ema_val)) in trvi_ema.iter().zip(ema_of_tr.iter()).enumerate() {
                assert!(
                    trvi_val.is_finite(),
                    "TRVI EMA optional output is not finite at index {}: {}, options={:?}",
                    i,
                    trvi_val,
                    options
                );
                if !approx_eq!(f64, trvi_val, ema_val, epsilon = 1e-12) {
                    let start = if i > 10 { i - 10 } else { 0 };
                    let end = if trvi_tr.len() - 10 >= i {
                        i + 10
                    } else {
                        trvi_tr.len()
                    };
                    println!(
                        "\ntrvi_ema: {:?}\n\nEMA TR: {:?}",
                        &trvi_ema[start..end],
                        &ema_of_tr[start..end]
                    );
                    panic!(
                        "EMA value mismatch at index {}: trvi={}, ema_of_tr={}, options={:?}",
                        i, trvi_val, ema_val, options
                    );
                }
            }
            println!(
                "✓ EMA optional output matches rust_ema(standalone TR): len={}, options={:?}",
                trvi_ema.len(),
                options
            );
        }

        println!("✓ All TRVI optional output tests passed!");
    }

    #[test]
    fn test_trvi_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                if high.len() < min_data(&options) {
                    continue;
                }

                let (outputs, _) =
                    rust_trvi(&inputs, &options, None).expect("Rust TRVI indicator failed");

                for (i, &val) in outputs[0].iter().enumerate() {
                    if val.is_nan() || val.is_infinite() {
                        panic!(
                            "Rust TRVI has NaN/Inf at index {}: val={}, options={:?}, stock={}",
                            i, val, options, stock_symbol
                        );
                    }
                }
            }

            println!("✓ TRVI database test passed for stock {}", stock_symbol);
        }
    }

    #[test]
    fn test_trvi_database_optional_outputs() {
        use tulip_rs::indicators::ema::indicator as rust_ema;
        use tulip_rs::indicators::tr::indicator as rust_tr;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let n = high.len();
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                if n < min_data(&options) {
                    continue;
                }

                // TRVI with both optional outputs.
                let (trvi_outputs, _) = rust_trvi(&inputs, &options, Some(&[true, true]))
                    .expect("TRVI with optional outputs failed");

                let trvi_tr = &trvi_outputs[1];
                let trvi_ema = &trvi_outputs[2];

                // Standalone TR.
                let (tr_outputs, _) =
                    rust_tr(&inputs, &[], None).expect("Rust TR indicator failed");
                let standalone_tr = &tr_outputs[0];

                // TR length check.
                assert_eq!(
                    trvi_tr.len(),
                    standalone_tr.len(),
                    "TR length mismatch: stock={}, options={:?}, trvi={}, standalone={}",
                    stock_symbol,
                    options,
                    trvi_tr.len(),
                    standalone_tr.len()
                );

                // TR value comparison — lengths are guaranteed equal by the assert above.
                for (i, (&trvi_val, &tr_val)) in
                    trvi_tr.iter().zip(standalone_tr.iter()).enumerate()
                {
                    if !trvi_val.is_finite() {
                        panic!(
                            "TRVI TR not finite at index {}: stock={}, options={:?}",
                            i, stock_symbol, options
                        );
                    }
                    if !approx_eq!(f64, trvi_val, tr_val, epsilon = 1e-12) {
                        let start = if i > 10 { i - 10 } else { 0 };
                        let end = if trvi_tr.len() - 10 >= i {
                            i + 10
                        } else {
                            trvi_tr.len()
                        };
                        println!(
                            "\ntrvi_tr: {:?}\n\nTR: {:?}",
                            &trvi_tr[start..end],
                            &standalone_tr[start..end]
                        );
                        panic!(
                            "TR mismatch at index {}: trvi={}, standalone={}, stock={}, options={:?}",
                            i, trvi_val, tr_val, stock_symbol, options
                        );
                    }
                }

                // EMA: feed standalone TR into rust_ema.
                let (ema_outputs, _) = rust_ema(&[standalone_tr.as_slice()], &options, None)
                    .expect("Rust EMA(TR) indicator failed");
                let ema_of_tr = &ema_outputs[0];

                // EMA length check.
                assert_eq!(
                    trvi_ema.len(),
                    ema_of_tr.len(),
                    "EMA length mismatch: stock={}, options={:?}, trvi={}, ema_of_tr={}",
                    stock_symbol,
                    options,
                    trvi_ema.len(),
                    ema_of_tr.len()
                );

                // EMA value comparison — lengths are guaranteed equal by the assert above.
                for (i, (&trvi_val, &ema_val)) in trvi_ema.iter().zip(ema_of_tr.iter()).enumerate()
                {
                    if !trvi_val.is_finite() {
                        panic!(
                            "TRVI EMA not finite at index {}: stock={}, options={:?}",
                            i, stock_symbol, options
                        );
                    }
                    if !approx_eq!(f64, trvi_val, ema_val, epsilon = 1e-12) {
                        panic!(
                            "EMA mismatch at index {}: trvi={}, ema_of_tr={}, stock={}, options={:?}",
                            i, trvi_val, ema_val, stock_symbol, options
                        );
                    }
                }
            }

            println!(
                "✓ TRVI database optional outputs test passed for stock {}",
                stock_symbol
            );
        }

        println!("✓ All TRVI database optional output tests passed!");
    }

    #[test]
    fn test_trvi_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs_rust = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                if high.len() < min_data(&options) {
                    continue;
                }

                // Full output in one shot.
                let (full_outputs, _) = rust_trvi(&inputs_rust, &options, None)
                    .expect("Rust TRVI indicator failed on full data");

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                let mut batch_full_output: Vec<f64> = Vec::new();

                if high.len() <= min_data_val {
                    let (outputs, _) = rust_trvi(&inputs_rust, &options, None)
                        .expect("Failed to run TRVI indicator");
                    batch_full_output.extend_from_slice(&outputs[0]);
                } else {
                    // First chunk.
                    let chunk_inputs = [
                        &high[..min_data_val],
                        &low[..min_data_val],
                        &close[..min_data_val],
                    ];
                    let (first_outputs, mut state) = rust_trvi(&chunk_inputs, &options, None)
                        .expect("Failed to run TRVI indicator on first chunk");
                    batch_full_output.extend_from_slice(&first_outputs[0]);

                    // Remaining chunks.
                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for ((high_chunk, low_chunk), close_chunk) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                    {
                        let chunk_inputs = [high_chunk, low_chunk, close_chunk];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("TRVI batch indicator failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }

                    // Remainder.
                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();
                    if !high_rem.is_empty() {
                        let chunk_inputs = [high_rem, low_rem, close_rem];
                        let chunk_outputs = state
                            .batch_indicator(&chunk_inputs, None)
                            .expect("TRVI batch indicator remainder failed");
                        batch_full_output.extend_from_slice(&chunk_outputs[0]);
                    }
                }

                // Compare full vs batched.
                assert_eq!(
                    full_outputs[0].len(),
                    batch_full_output.len(),
                    "Output length mismatch: stock={}, options={:?}, full={}, batch={}",
                    stock_symbol,
                    options,
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
                        "TRVI state mismatch at index {}: full={}, batch={}, stock={}, options={:?}",
                        i, full_val, batch_val, stock_symbol, options
                    );
                }
            }

            println!(
                "✓ TRVI database state test passed for stock {}",
                stock_symbol
            );
        }
    }

    // =========================================================================
    // SIMD by-assets: outputs match scalar TRVI (database)
    // =========================================================================

    #[test]
    fn test_trvi_simd_by_assets_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_hlc_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 3] = [&stock_data[0].1, &stock_data[0].2, &stock_data[0].3];
            let asset1: [&[f64]; 3] = [&stock_data[1].1, &stock_data[1].2, &stock_data[1].3];
            let asset2: [&[f64]; 3] = [&stock_data[2].1, &stock_data[2].2, &stock_data[2].3];
            let asset3: [&[f64]; 3] = [&stock_data[3].1, &stock_data[3].2, &stock_data[3].3];
            let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by-assets TRVI failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) =
                    rust_trvi(&scalar_inputs, &options, None).expect("Rust TRVI failed");

                let simd_trvi = &simd_results[asset_idx][0];
                let scalar_trvi = &scalar_outputs[0];

                assert_eq!(
                    simd_trvi.len(),
                    scalar_trvi.len(),
                    "trvi length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_trvi.iter().zip(scalar_trvi.iter()).enumerate() {
                    assert_eq!(sv, rv,
                        "trvi mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }

            println!("✓ SIMD by-assets vs scalar TRVI ok for options={options:?}");
        }
    }

    // =========================================================================
    // SIMD by-options: outputs match scalar TRVI (database)
    // =========================================================================

    #[test]
    fn test_trvi_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by-options TRVI failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                if high.len() < min_data(options) {
                    continue;
                }
                let (scalar_outputs, _) =
                    rust_trvi(&inputs, options, None).expect("Rust TRVI failed");

                let simd_trvi = &simd_results[opt_idx][0];
                let scalar_trvi = &scalar_outputs[0];

                assert_eq!(
                    simd_trvi.len(),
                    scalar_trvi.len(),
                    "trvi length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_trvi.iter().zip(scalar_trvi.iter()).enumerate() {
                    assert_eq!(sv, rv,
                        "trvi mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }

            println!("✓ SIMD by-options vs scalar TRVI ok for stock={stock_symbol}");
        }
    }

    // =========================================================================
    // SIMD by-assets: first 1000 bars via SIMD, rest via batch_indicator
    // =========================================================================

    #[test]
    fn test_trvi_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_hlc_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 3] = [
                &stock_data[0].1[..FIRST_CHUNK],
                &stock_data[0].2[..FIRST_CHUNK],
                &stock_data[0].3[..FIRST_CHUNK],
            ];
            let asset1: [&[f64]; 3] = [
                &stock_data[1].1[..FIRST_CHUNK],
                &stock_data[1].2[..FIRST_CHUNK],
                &stock_data[1].3[..FIRST_CHUNK],
            ];
            let asset2: [&[f64]; 3] = [
                &stock_data[2].1[..FIRST_CHUNK],
                &stock_data[2].2[..FIRST_CHUNK],
                &stock_data[2].3[..FIRST_CHUNK],
            ];
            let asset3: [&[f64]; 3] = [
                &stock_data[3].1[..FIRST_CHUNK],
                &stock_data[3].2[..FIRST_CHUNK],
                &stock_data[3].3[..FIRST_CHUNK],
            ];
            let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by-assets failed on first chunk");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let mut batch_trvi = simd_first[asset_idx][0].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("batch_indicator failed");
                    batch_trvi.extend_from_slice(&chunk_outputs[0]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_trvi.extend_from_slice(&chunk_outputs[0]);
                }

                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) =
                    rust_trvi(&scalar_inputs, &options, None).expect("Rust TRVI failed");

                assert_eq!(
                    batch_trvi.len(),
                    scalar_outputs[0].len(),
                    "trvi length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_trvi.iter().zip(scalar_outputs[0].iter()).enumerate() {
                    assert_eq!(bv, rv,
                        "trvi mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }

            println!("✓ SIMD by-assets state continuity ok for options={options:?}");
        }
    }

    // =========================================================================
    // SIMD by-options: first 1000 bars via SIMD, rest via batch_indicator
    // =========================================================================

    #[test]
    fn test_trvi_simd_by_options_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_hlc_arrays(stock_data);

            let first_inputs = [
                &high[..FIRST_CHUNK],
                &low[..FIRST_CHUNK],
                &close[..FIRST_CHUNK],
            ];
            let (simd_first, mut states) =
                indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD by-options failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch_trvi = simd_first[opt_idx][0].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("batch_indicator failed");
                    batch_trvi.extend_from_slice(&chunk_outputs[0]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_trvi.extend_from_slice(&chunk_outputs[0]);
                }

                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) =
                    rust_trvi(&scalar_inputs, options, None).expect("Rust TRVI failed");

                assert_eq!(
                    batch_trvi.len(),
                    scalar_outputs[0].len(),
                    "trvi length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_trvi.iter().zip(scalar_outputs[0].iter()).enumerate() {
                    assert_eq!(bv, rv,
                        "trvi mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}");
                }
            }

            println!("✓ SIMD by-options state continuity ok for stock={stock_symbol}");
        }
    }

    // =========================================================================
    // SIMD by-assets optional outputs: trvi, tr, ema all match scalar
    // =========================================================================

    #[test]
    fn test_trvi_simd_by_assets_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_hlc_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 3] = [&stock_data[0].1, &stock_data[0].2, &stock_data[0].3];
            let asset1: [&[f64]; 3] = [&stock_data[1].1, &stock_data[1].2, &stock_data[1].3];
            let asset2: [&[f64]; 3] = [&stock_data[2].1, &stock_data[2].2, &stock_data[2].3];
            let asset3: [&[f64]; 3] = [&stock_data[3].1, &stock_data[3].2, &stock_data[3].3];
            let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_results, _) =
                indicator_by_assets::<4>(&inputs_4, &options, Some(&[true, true]))
                    .expect("SIMD by-assets TRVI with optional outputs failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) = rust_trvi(&scalar_inputs, &options, Some(&[true, true]))
                    .expect("Scalar TRVI with optional outputs failed");

                // trvi (0), tr (1), ema (2)
                for out_idx in 0..3 {
                    let simd_out = &simd_results[asset_idx][out_idx];
                    let scalar_out = &scalar_outputs[out_idx];
                    assert_eq!(
                        simd_out.len(), scalar_out.len(),
                        "output[{out_idx}] length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out).enumerate() {
                        assert_eq!(
                            sv, rv,
                            "output[{out_idx}] mismatch at index {i}: simd={sv}, scalar={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }

                println!(
                    "✓ SIMD by-assets optional outputs match scalar: stock={stock_symbol}, options={options:?}"
                );
            }
        }

        println!("✓ All SIMD by-assets TRVI optional output tests passed!");
    }

    // =========================================================================
    // SIMD by-options optional outputs: trvi, tr, ema all match scalar
    // =========================================================================

    #[test]
    fn test_trvi_simd_by_options_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (_stock_symbol, stock_data) in data.iter().take(4) {
            let (high, low, close) = get_hlc_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let (simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(&[true, true]))
                    .expect("SIMD by-options TRVI with optional outputs failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_outputs, _) = rust_trvi(&inputs, options, Some(&[true, true]))
                    .expect("Scalar TRVI with optional outputs failed");

                // trvi (0), tr (1), ema (2)
                for out_idx in 0..3 {
                    let simd_out = &simd_results[opt_idx][out_idx];
                    let scalar_out = &scalar_outputs[out_idx];
                    assert_eq!(
                        simd_out.len(),
                        scalar_out.len(),
                        "output[{out_idx}] length mismatch: opt_idx={opt_idx}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out).enumerate() {
                        assert_eq!(
                            sv, rv,
                            "output[{out_idx}] mismatch at index {i}: simd={sv}, scalar={rv}, \
                             opt_idx={opt_idx}, options={options:?}"
                        );
                    }
                }

                println!(
                    "✓ SIMD by-options optional outputs match scalar: opt_idx={opt_idx}, options={options:?}"
                );
            }
        }

        println!("\u{2713} All SIMD by-options TRVI optional output tests passed!");
    }

    }
