#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::msw::{
        indicator as new_msw, indicator_by_assets, indicator_by_options, min_data, TIndicatorState,
    };
    use tulip_test::c_bindings::{ti_msw, ti_msw_start};
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const FIRST_CHUNK: usize = 1000;
    /// SIMD trig (simd_atan/simd_sin) vs scalar — small rounding differences are expected.
    const EPSILON_SIMD: f64 = 1e-3;
    /// Fine epsilon for streaming vs full-run comparison (both use the same SDFT).
    const EPSILON_STREAM: f64 = 1e-10;
    /// Looser epsilon for comparison against the C reference (different float ops).
    const EPSILON_C: f64 = 1e-3;

    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    // Standard periods — same as msw_test.rs.
    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [10.0], [14.0], [20.0]];

    // Extended list: periods 9 and 25 were incorrect in the old implementation due
    // to the simd_remainder_dispatch! base-case bug (remainder = 1 element after
    // N=8 chunks). new_msw fixes this; these should now agree with the C reference.
    const OPTIONS_LIST_BUG_PERIODS: [[f64; 1]; 3] = [[9.0], [17.0], [25.0]];

    fn expand_close(reps: usize) -> Vec<f64> {
        let mut v = CLOSE.to_vec();
        for _ in 0..reps {
            v.extend_from_slice(&CLOSE);
        }
        v
    }

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // ── Helper: compare new_msw vs C reference ────────────────────────────────

    fn compare_vs_c(close: &[f64], options: &[f64; 1], label: &str) {
        // C reference
        let inputs_c: Vec<*const f64> = vec![close.as_ptr()];
        let start_index = unsafe { ti_msw_start(options.as_ptr()) };
        assert!(start_index >= 0);
        let output_len_c = close.len() - start_index as usize;

        let mut sine_c = vec![0.0f64; output_len_c];
        let mut lead_c = vec![0.0f64; output_len_c];
        let mut out_ptrs: Vec<*mut f64> = vec![sine_c.as_mut_ptr(), lead_c.as_mut_ptr()];
        let ret = unsafe {
            ti_msw(
                close.len() as i32,
                inputs_c.as_ptr(),
                options.as_ptr(),
                out_ptrs.as_mut_ptr(),
            )
        };
        assert_eq!(ret, 0, "ti_msw returned error {ret}");

        // new_msw
        let inputs = [close];
        let (rust_out, _) = new_msw(&inputs, options, None).expect("new_msw failed");
        let n = rust_out[0].len();

        for (name, c_vals, rust_vals) in [
            ("sine", &sine_c, &rust_out[0]),
            ("lead", &lead_c, &rust_out[1]),
        ] {
            for (i, (&c_val, &rust_val)) in c_vals
                .iter()
                .rev()
                .take(n)
                .zip(rust_vals.iter().rev())
                .enumerate()
            {
                let idx = n - i - 1;
                assert!(
                    !rust_val.is_nan(),
                    "{label} {name}[{idx}] is NaN (options={options:?})"
                );
                assert!(!rust_val.is_infinite(), "{label} {name}[{idx}] is infinite");
                if c_val.is_nan() || c_val.is_infinite() {
                    continue; // skip known C edge-case artefacts
                }
                assert!(
                    approx_eq!(f64, c_val, rust_val, epsilon = EPSILON_C),
                    "{label} {name}[{idx}]: C={c_val}, new_msw={rust_val}, options={options:?}"
                );
            }
        }
    }

    // ── Helper: compare streaming vs full run ─────────────────────────────────

    fn compare_streaming_vs_full(close: &[f64], options: &[f64; 1], label: &str) {
        let inputs = [close];
        let (full_out, _) = new_msw(&inputs, options, None).expect("new_msw full run failed");

        // First chunk — capped at close.len() so short test data doesn't panic.
        let first_len = min_data(options).max(CHUNK_SIZE).min(close.len());
        let (first_out, mut state) =
            new_msw(&[&close[..first_len]], options, None).expect("new_msw first chunk failed");

        let mut batch_out = vec![first_out[0].clone(), first_out[1].clone()];

        // Remaining chunks
        let mut remaining = close[first_len..].chunks_exact(CHUNK_SIZE);
        for chunk in remaining.by_ref() {
            let chunk_out = state
                .batch_indicator(&[chunk], None)
                .expect("batch_indicator failed");
            batch_out[0].extend_from_slice(&chunk_out[0]);
            batch_out[1].extend_from_slice(&chunk_out[1]);
        }
        let rem = remaining.remainder();
        if !rem.is_empty() {
            let chunk_out = state
                .batch_indicator(&[rem], None)
                .expect("batch_indicator remainder failed");
            batch_out[0].extend_from_slice(&chunk_out[0]);
            batch_out[1].extend_from_slice(&chunk_out[1]);
        }

        for (name, full_vals, batch_vals) in [
            ("sine", &full_out[0], &batch_out[0]),
            ("lead", &full_out[1], &batch_out[1]),
        ] {
            assert_eq!(
                full_vals.len(),
                batch_vals.len(),
                "{label} {name} length mismatch: full={}, batch={} (options={options:?})",
                full_vals.len(),
                batch_vals.len(),
            );
            for (i, (&fv, &bv)) in full_vals.iter().zip(batch_vals.iter()).enumerate() {
                assert!(
                    approx_eq!(f64, fv, bv, epsilon = EPSILON_STREAM),
                    "{label} {name}[{i}]: full={fv}, batch={bv}, options={options:?}"
                );
            }
        }
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// Correctness against C reference — standard periods, synthetic data.
    #[test]
    fn test_new_msw_vs_c_sample() {
        let close = expand_close(3);
        for options in OPTIONS_LIST {
            compare_vs_c(&close, &options, "sample");
        }
    }

    /// Correctness for the periods that were buggy in the old implementation
    /// (simd_remainder_dispatch! base-case error). new_msw should now match C.
    #[test]
    fn test_new_msw_vs_c_previously_bugged_periods() {
        let close = expand_close(3);
        for options in OPTIONS_LIST_BUG_PERIODS {
            compare_vs_c(&close, &options, "bug-period");
        }
    }

    /// Correctness against C reference — full database stocks, standard periods.
    #[test]
    fn test_new_msw_database_vs_c() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            for options in OPTIONS_LIST {
                compare_vs_c(&close, &options, symbol);
            }
        }
        println!("✓ new_msw vs C: all database stocks passed");
    }

    /// Correctness against C — database stocks with previously-bugged periods.
    #[test]
    fn test_new_msw_database_vs_c_bug_periods() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            for options in OPTIONS_LIST_BUG_PERIODS {
                compare_vs_c(&close, &options, symbol);
            }
        }
        println!("✓ new_msw vs C: bug-period database tests passed");
    }

    /// Streaming continuity — batch_indicator must produce output identical to a
    /// single full indicator() call (same SDFT path, so epsilon is very tight).
    #[test]
    fn test_new_msw_streaming_sample() {
        let close = expand_close(15); // 15 reps = 240 bars — well above CHUNK_SIZE
        for options in OPTIONS_LIST {
            compare_streaming_vs_full(&close, &options, "sample");
        }
    }

    /// Streaming continuity on full database stocks.
    #[test]
    fn test_new_msw_streaming_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            for options in OPTIONS_LIST {
                compare_streaming_vs_full(&close, &options, symbol);
            }
        }
        println!("✓ new_msw streaming: all database stocks passed");
    }

    /// SDFT numerical-stability test — processes a long synthetic series in
    /// streaming mode and compares the result to a single full indicator() call.
    ///
    /// The re-anchor interval is 50 000 bars; this test exercises ~7 re-anchors
    /// to confirm they do not introduce discontinuities.
    #[test]
    fn test_new_msw_sdft_long_run_stability() {
        // Generate ~360 000 bars by tiling the CLOSE sample.
        const REPS: usize = 24_000; // 24 000 × 15 = 360 000 bars
        let close = expand_close(REPS);

        for options in [[20.0f64], [50.0f64]] {
            let inputs = [close.as_slice()];

            // Reference: single full run (SDFT from bar 0).
            let (full_out, _) = new_msw(&inputs, &options, None).expect("full run failed");

            // Streaming: first min_data bars, then CHUNK_SIZE at a time.
            let first_len = min_data(&options).max(CHUNK_SIZE);
            let (first_out, mut state) =
                new_msw(&[&close[..first_len]], &options, None).expect("first chunk failed");

            let mut batch_sine = first_out[0].clone();
            let mut batch_lead = first_out[1].clone();

            let mut chunks = close[first_len..].chunks_exact(CHUNK_SIZE);
            for chunk in chunks.by_ref() {
                let out = state.batch_indicator(&[chunk], None).unwrap();
                batch_sine.extend_from_slice(&out[0]);
                batch_lead.extend_from_slice(&out[1]);
            }
            let rem = chunks.remainder();
            if !rem.is_empty() {
                let out = state.batch_indicator(&[rem], None).unwrap();
                batch_sine.extend_from_slice(&out[0]);
                batch_lead.extend_from_slice(&out[1]);
            }

            // Compare — tight epsilon since both code paths use identical SDFT.
            assert_eq!(full_out[0].len(), batch_sine.len(), "sine length mismatch");
            for (i, (&fv, &bv)) in full_out[0].iter().zip(batch_sine.iter()).enumerate() {
                assert!(
                    approx_eq!(f64, fv, bv, epsilon = EPSILON_STREAM),
                    "sine[{i}] drift after long run: full={fv}, batch={bv}, options={options:?}"
                );
            }
            for (i, (&fv, &bv)) in full_out[1].iter().zip(batch_lead.iter()).enumerate() {
                assert!(
                    approx_eq!(f64, fv, bv, epsilon = EPSILON_STREAM),
                    "lead[{i}] drift after long run: full={fv}, batch={bv}, options={options:?}"
                );
            }

            println!(
                "✓ SDFT stability: {} bars, period={:.0}, max drift < {EPSILON_STREAM}",
                close.len(),
                options[0]
            );
        }
    }

    // ── SIMD by_assets vs scalar ──────────────────────────────────────────────

    /// SIMD by_assets output must match scalar indicator output within EPSILON_SIMD
    /// for each asset. Uses the first 4 database stocks and OPTIONS_LIST[0] (period=5).
    #[test]
    fn test_msw_simd_by_assets_vs_scalar() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| (sym.clone(), get_close_array(eod)))
            .collect();

        let inputs_4: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        for options in OPTIONS_LIST {
            let (simd_results, _) =
                indicator_by_assets::<4>(&inputs_4, &options, None).expect("SIMD by_assets failed");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let (scalar_out, _) =
                    new_msw(&[close.as_slice()], &options, None).expect("scalar failed");

                for (name, simd_line, scalar_line) in [
                    ("sine", &simd_results[asset_idx][0], &scalar_out[0]),
                    ("lead", &simd_results[asset_idx][1], &scalar_out[1]),
                ] {
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{name} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        assert!(!sv.is_nan(), "SIMD {name}[{i}] NaN: stock={stock_symbol}");
                        assert!(
                            !sv.is_infinite(),
                            "SIMD {name}[{i}] Inf: stock={stock_symbol}"
                        );
                        assert!(
                            approx_eq!(f64, sv, rv, epsilon = EPSILON_SIMD),
                            "{name}[{i}]: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
                println!("✓ SIMD by_assets vs scalar passed: {stock_symbol} options={options:?}");
            }
        }
        println!("✓ All SIMD by_assets vs scalar MSW tests passed!");
    }

    // ── SIMD by_assets state continuity ──────────────────────────────────────

    /// SIMD by_assets first chunk + scalar batch_indicator remainder must match
    /// the full scalar run within EPSILON_SIMD.
    #[test]
    fn test_msw_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| (sym.clone(), get_close_array(eod)))
            .collect();

        for options in OPTIONS_LIST {
            let inputs_first: [&[&[f64]; 1]; 4] = [
                &[&stock_data[0].1[..FIRST_CHUNK]],
                &[&stock_data[1].1[..FIRST_CHUNK]],
                &[&stock_data[2].1[..FIRST_CHUNK]],
                &[&stock_data[3].1[..FIRST_CHUNK]],
            ];

            let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_first, &options, None)
                .expect("SIMD by_assets first chunk failed");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let mut batch_sine = simd_first[asset_idx][0].clone();
                let mut batch_lead = simd_first[asset_idx][1].clone();

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = states[asset_idx]
                        .batch_indicator(&[chunk], None)
                        .expect("batch_indicator failed");
                    batch_sine.extend_from_slice(&out[0]);
                    batch_lead.extend_from_slice(&out[1]);
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = states[asset_idx]
                        .batch_indicator(&[rem], None)
                        .expect("batch_indicator remainder failed");
                    batch_sine.extend_from_slice(&out[0]);
                    batch_lead.extend_from_slice(&out[1]);
                }

                let (scalar_out, _) =
                    new_msw(&[close.as_slice()], &options, None).expect("scalar failed");

                for (name, batch_vals, scalar_vals) in [
                    ("sine", &batch_sine, &scalar_out[0]),
                    ("lead", &batch_lead, &scalar_out[1]),
                ] {
                    assert_eq!(
                        batch_vals.len(),
                        scalar_vals.len(),
                        "{name} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&bv, &rv)) in batch_vals.iter().zip(scalar_vals.iter()).enumerate() {
                        assert!(!bv.is_nan(), "{name}[{i}] NaN: stock={stock_symbol}");
                        assert!(!bv.is_infinite(), "{name}[{i}] Inf: stock={stock_symbol}");
                        assert!(
                            approx_eq!(f64, bv, rv, epsilon = EPSILON_SIMD),
                            "{name}[{i}]: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
                println!("✓ SIMD by_assets state continuity: {stock_symbol} options={options:?}");
            }
        }
        println!("✓ All SIMD by_assets state continuity MSW tests passed!");
    }

    // ── SIMD by_options vs scalar ─────────────────────────────────────────────

    /// SIMD by_options runs all 4 option periods simultaneously; each lane's output
    /// must match the scalar indicator run for that period.
    #[test]
    fn test_msw_simd_by_options_vs_scalar() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4: [&[f64; 1]; 4] = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by_options failed");

            for (lane, &options) in options_4.iter().enumerate() {
                let (scalar_out, _) = new_msw(&inputs, options, None).expect("scalar failed");

                for (name, simd_line, scalar_line) in [
                    ("sine", &simd_results[lane][0], &scalar_out[0]),
                    ("lead", &simd_results[lane][1], &scalar_out[1]),
                ] {
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{name} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        assert!(!sv.is_nan(), "SIMD {name}[{i}] NaN: stock={stock_symbol}");
                        assert!(
                            !sv.is_infinite(),
                            "SIMD {name}[{i}] Inf: stock={stock_symbol}"
                        );
                        assert!(
                            approx_eq!(f64, sv, rv, epsilon = EPSILON_SIMD),
                            "{name}[{i}]: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
            println!("✓ SIMD by_options vs scalar passed: {stock_symbol}");
        }
        println!("✓ All SIMD by_options vs scalar MSW tests passed!");
    }

    // ── SIMD by_options state continuity ─────────────────────────────────────

    /// SIMD by_options first chunk + scalar batch_indicator remainder must match
    /// the full scalar run for each option lane within EPSILON_SIMD.
    #[test]
    fn test_msw_simd_by_options_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4: [&[f64; 1]; 4] = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            let (simd_first, mut states) =
                indicator_by_options::<4>(&[&close[..FIRST_CHUNK]], &options_4, None)
                    .expect("SIMD by_options first chunk failed");

            for (lane, &options) in options_4.iter().enumerate() {
                let mut batch_sine = simd_first[lane][0].clone();
                let mut batch_lead = simd_first[lane][1].clone();

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = states[lane]
                        .batch_indicator(&[chunk], None)
                        .expect("batch_indicator failed");
                    batch_sine.extend_from_slice(&out[0]);
                    batch_lead.extend_from_slice(&out[1]);
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = states[lane]
                        .batch_indicator(&[rem], None)
                        .expect("batch_indicator remainder failed");
                    batch_sine.extend_from_slice(&out[0]);
                    batch_lead.extend_from_slice(&out[1]);
                }

                let (scalar_out, _) =
                    new_msw(&[close.as_slice()], options, None).expect("scalar failed");

                for (name, batch_vals, scalar_vals) in [
                    ("sine", &batch_sine, &scalar_out[0]),
                    ("lead", &batch_lead, &scalar_out[1]),
                ] {
                    assert_eq!(
                        batch_vals.len(),
                        scalar_vals.len(),
                        "{name} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&bv, &rv)) in batch_vals.iter().zip(scalar_vals.iter()).enumerate() {
                        assert!(!bv.is_nan(), "{name}[{i}] NaN: stock={stock_symbol}");
                        assert!(!bv.is_infinite(), "{name}[{i}] Inf: stock={stock_symbol}");
                        assert!(
                            approx_eq!(f64, bv, rv, epsilon = EPSILON_SIMD),
                            "{name}[{i}]: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
            println!("✓ SIMD by_options state continuity: {stock_symbol}");
        }
        println!("✓ All SIMD by_options state continuity MSW tests passed!");
    }

    }
