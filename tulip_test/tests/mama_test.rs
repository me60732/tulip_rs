#[cfg(test)]
mod tests {
    use tulip_rs::indicator_types::TIndicatorState;
    use tulip_rs::indicators::homodynediscriminator::indicator as hd_indicator;
    use tulip_rs::indicators::mama::{
        indicator as mama_indicator, indicator_by_assets, indicator_by_options, min_data,
        output_length,
    };
    use tulip_rs::types::IndicatorError;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const FIRST_CHUNK: usize = 1000;

    /// Ehlers defaults plus three alternative parameter sets.
    const OPTIONS_LIST: [[f64; 2]; 4] = [[0.5, 0.05], [0.4, 0.04], [0.3, 0.03], [0.2, 0.02]];

    /// Indices into the outputs vec: [mama, fama, dc_period, alpha]
    const OUTPUT_LABELS: [&str; 4] = ["mama", "fama", "dc_period", "alpha"];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Basic sanity checks — no database needed.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_min_data_and_output_length() {
        // Warmup is fixed at 23 bars regardless of options.
        for opts in &OPTIONS_LIST {
            assert_eq!(min_data(opts), 23, "min_data mismatch for options={opts:?}");
            assert_eq!(
                output_length(23, opts),
                1,
                "output_length(23) mismatch for options={opts:?}"
            );
            assert_eq!(
                output_length(100, opts),
                78,
                "output_length(100) mismatch for options={opts:?}"
            );
            assert_eq!(
                output_length(1000, opts),
                978,
                "output_length(1000) mismatch for options={opts:?}"
            );
        }
    }

    #[test]
    fn test_mama_not_enough_data() {
        let close: Vec<f64> = (0..22).map(|i| 100.0 + i as f64).collect();
        for opts in &OPTIONS_LIST {
            let result = mama_indicator(&[close.as_slice()], opts, None);
            assert!(
                matches!(result, Err(IndicatorError::NotEnoughData)),
                "Expected NotEnoughData for {} bars (need 23), options={opts:?}",
                close.len()
            );
        }
    }

    #[test]
    fn test_mama_invalid_options() {
        let close: Vec<f64> = (0..100).map(|i| 100.0 + i as f64).collect();
        let inputs = [close.as_slice()];
        assert!(matches!(
            mama_indicator(&inputs, &[0.0, 0.05], None),
            Err(IndicatorError::InvalidOptions)
        ));
        assert!(matches!(
            mama_indicator(&inputs, &[1.1, 0.05], None),
            Err(IndicatorError::InvalidOptions)
        ));
        assert!(matches!(
            mama_indicator(&inputs, &[0.5, 0.0], None),
            Err(IndicatorError::InvalidOptions)
        ));
        assert!(matches!(
            mama_indicator(&inputs, &[0.5, 0.5], None),
            Err(IndicatorError::InvalidOptions)
        ));
        assert!(matches!(
            mama_indicator(&inputs, &[0.5, 0.6], None),
            Err(IndicatorError::InvalidOptions)
        ));
        // All standard option sets must be accepted.
        for opts in &OPTIONS_LIST {
            assert!(
                mama_indicator(&inputs, opts, None).is_ok(),
                "Expected Ok for options={opts:?}"
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Seeding: first output must equal first valid bar's price exactly.
    // mama[0] = fama[0] = real[min_data - 1] regardless of alpha because the
    // seed is set to price before calling calc_unchecked.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_first_output_equals_seed_price() {
        let close: Vec<f64> = (0..100).map(|i| 50.0 + (i as f64) * 0.1).collect();
        for opts in &OPTIONS_LIST {
            let (outputs, _) =
                mama_indicator(&[close.as_slice()], opts, None).expect("indicator failed");
            let seed = close[min_data(opts) - 1];
            assert_eq!(
                outputs[0][0], seed,
                "mama[0] should equal close[min_data-1], options={opts:?}"
            );
            assert_eq!(
                outputs[1][0], seed,
                "fama[0] should equal close[min_data-1], options={opts:?}"
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // output_length formula must match the actual returned slice length.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_output_length_matches() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            for options in OPTIONS_LIST {
                let (out, _) =
                    mama_indicator(&[close.as_slice()], &options, None).expect("indicator failed");
                let expected = output_length(close.len(), &options);
                assert_eq!(out[0].len(), expected, "mama length: stock={stock_symbol}");
                assert_eq!(out[1].len(), expected, "fama length: stock={stock_symbol}");
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Output bounds: mama and fama finite; alpha in [slow, fast].
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_output_bounds() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            for options in OPTIONS_LIST {
                let (out, _) = mama_indicator(&[close.as_slice()], &options, Some(&[true, true]))
                    .expect("indicator failed");
                let (fast, slow) = (options[0], options[1]);
                for (i, ((&m, &f), &a)) in out[0]
                    .iter()
                    .zip(out[1].iter())
                    .zip(out[3].iter())
                    .enumerate()
                {
                    assert!(m.is_finite(), "mama NaN/Inf at {i}: stock={stock_symbol}");
                    assert!(f.is_finite(), "fama NaN/Inf at {i}: stock={stock_symbol}");
                    assert!(
                        a >= slow && a <= fast,
                        "alpha={a} outside [{slow},{fast}] at {i}: stock={stock_symbol}"
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Optional outputs: populated only when requested, correct length, correct
    // values when compared to individual optional runs.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_optional_outputs() {
        let close: Vec<f64> = (0..200).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();
        let inputs = [close.as_slice()];

        for opts in &OPTIONS_LIST {
            let expected_len = output_length(close.len(), opts);

            let (none, _) = mama_indicator(&inputs, opts, None).unwrap();
            assert_eq!(none[0].len(), expected_len, "mama len, options={opts:?}");
            assert_eq!(none[1].len(), expected_len, "fama len, options={opts:?}");
            assert!(
                none[2].is_empty(),
                "dc_period should be empty, options={opts:?}"
            );
            assert!(
                none[3].is_empty(),
                "alpha should be empty, options={opts:?}"
            );

            let (dc_only, _) = mama_indicator(&inputs, opts, Some(&[true, false])).unwrap();
            assert_eq!(
                dc_only[2].len(),
                expected_len,
                "dc_period len, options={opts:?}"
            );
            assert!(
                dc_only[3].is_empty(),
                "alpha should be empty, options={opts:?}"
            );

            let (alpha_only, _) = mama_indicator(&inputs, opts, Some(&[false, true])).unwrap();
            assert!(
                alpha_only[2].is_empty(),
                "dc_period should be empty, options={opts:?}"
            );
            assert_eq!(
                alpha_only[3].len(),
                expected_len,
                "alpha len, options={opts:?}"
            );

            let (both, _) = mama_indicator(&inputs, opts, Some(&[true, true])).unwrap();
            assert_eq!(
                both[2].len(),
                expected_len,
                "dc_period (both) len, options={opts:?}"
            );
            assert_eq!(
                both[3].len(),
                expected_len,
                "alpha (both) len, options={opts:?}"
            );

            // Values must be consistent across all request modes.
            for (i, (&dc_a, &dc_b)) in dc_only[2].iter().zip(both[2].iter()).enumerate() {
                assert_eq!(
                    dc_a, dc_b,
                    "dc_period inconsistency at {i}, options={opts:?}"
                );
            }
            for (i, (&a_a, &a_b)) in alpha_only[3].iter().zip(both[3].iter()).enumerate() {
                assert_eq!(a_a, a_b, "alpha inconsistency at {i}, options={opts:?}");
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // dc_period identity: MAMA's embedded HD must produce bit-identical output
    // to running homodynediscriminator::indicator() on the same data.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_dc_period_matches_standalone_hd() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let (hd_out, _) = hd_indicator(&[close.as_slice()], &[], None).expect("HD failed");
            // dc_period is driven purely by the HD pipeline — options do not affect it.
            for opts in &OPTIONS_LIST {
                let (mama_out, _) = mama_indicator(&[close.as_slice()], opts, Some(&[true, false]))
                    .expect("MAMA failed");
                assert_eq!(
                    mama_out[2].len(),
                    hd_out[0].len(),
                    "dc_period length mismatch: stock={stock_symbol}, options={opts:?}"
                );
                for (i, (&mv, &hv)) in mama_out[2].iter().zip(hd_out[0].iter()).enumerate() {
                    assert_eq!(
                        mv, hv,
                        "dc_period mismatch at {i}: mama={mv}, hd={hv}, \
                         stock={stock_symbol}, options={opts:?}"
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Scalar state continuity: indicator() first chunk + batch_indicator()
    // remainder must be bit-exact to a full single-call run for all four
    // outputs (mama, fama, dc_period, alpha).
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let (ref_out, _) =
                    mama_indicator(&[close.as_slice()], &options, Some(&[true, true]))
                        .expect("reference run failed");

                let (first_out, mut state) =
                    mama_indicator(&[&close[..FIRST_CHUNK]], &options, Some(&[true, true]))
                        .expect("seed run failed");

                let mut batch = [
                    first_out[0].clone(),
                    first_out[1].clone(),
                    first_out[2].clone(),
                    first_out[3].clone(),
                ];

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = state
                        .batch_indicator(&[chunk], Some(&[true, true]))
                        .expect("batch_indicator failed");
                    for k in 0..4 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = state
                        .batch_indicator(&[rem], Some(&[true, true]))
                        .expect("batch_indicator failed on remainder");
                    for k in 0..4 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }

                for k in 0..4 {
                    let label = OUTPUT_LABELS[k];
                    assert_eq!(
                        batch[k].len(),
                        ref_out[k].len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&bv, &rv)) in batch[k].iter().zip(ref_out[k].iter()).enumerate() {
                        if bv.is_nan() {
                            panic!(
                                "{label} has NaN at {i}: stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if bv.is_infinite() {
                            panic!(
                                "{label} has Inf at {i}: stock={stock_symbol}, options={options:?}"
                            );
                        }
                        assert_eq!(
                            bv, rv,
                            "{label} mismatch at {i}: batch={bv}, ref={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets: N=4 assets processed in a single SIMD pass must match
    // the scalar run for all four outputs within 1e-4.
    //
    // The HD pipeline uses simd_atan (Cephes polynomial) which differs from
    // scalar libm atan by ~1e-15; IIR accumulation keeps it well below 1e-4.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_simd_by_assets_vs_regular_database() {
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
                indicator_by_assets::<4>(&inputs_4, &options, Some(&[true, true]))
                    .expect("SIMD by_assets failed");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let (scalar_out, _) =
                    mama_indicator(&[close.as_slice()], &options, Some(&[true, true]))
                        .expect("scalar failed");

                for k in 0..4 {
                    let label = OUTPUT_LABELS[k];
                    let simd_line = &simd_results[asset_idx][k];
                    let scalar_line = &scalar_out[k];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        if sv.is_nan() {
                            panic!(
                                "SIMD by_assets {label} has NaN at {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if sv.is_infinite() {
                            panic!(
                                "SIMD by_assets {label} has Inf at {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        let diff = (sv - rv).abs();
                        assert!(
                            diff < 1e-4,
                            "{label} mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_options: N=4 different option sets on one asset must each match
    // their scalar counterpart within 1e-4 for all four outputs.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            let (simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(&[true, true]))
                    .expect("SIMD by_options failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_out, _) =
                    mama_indicator(&inputs, options, Some(&[true, true])).expect("scalar failed");

                for k in 0..4 {
                    let label = OUTPUT_LABELS[k];
                    let simd_line = &simd_results[opt_idx][k];
                    let scalar_line = &scalar_out[k];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        if sv.is_nan() {
                            panic!(
                                "SIMD by_options {label} has NaN at {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if sv.is_infinite() {
                            panic!(
                                "SIMD by_options {label} has Inf at {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        let diff = (sv - rv).abs();
                        assert!(
                            diff < 1e-4,
                            "{label} mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets state continuity: SIMD seed (first chunk) + scalar
    // batch_indicator (remainder) must match full scalar run within 1e-4 for
    // all four outputs.
    //
    // The SIMD seed state differs from the scalar seed state by ~1e-15 in
    // simd_atan residuals; the IIR pipeline keeps the propagated error <1e-4.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| (sym.clone(), get_close_array(eod)))
            .collect();

        for options in OPTIONS_LIST {
            let inputs_4_first: [&[&[f64]; 1]; 4] = [
                &[&stock_data[0].1[..FIRST_CHUNK]],
                &[&stock_data[1].1[..FIRST_CHUNK]],
                &[&stock_data[2].1[..FIRST_CHUNK]],
                &[&stock_data[3].1[..FIRST_CHUNK]],
            ];

            let (simd_first, mut states) =
                indicator_by_assets::<4>(&inputs_4_first, &options, Some(&[true, true]))
                    .expect("SIMD by_assets failed on first chunk");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let mut batch = [
                    simd_first[asset_idx][0].clone(),
                    simd_first[asset_idx][1].clone(),
                    simd_first[asset_idx][2].clone(),
                    simd_first[asset_idx][3].clone(),
                ];

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = states[asset_idx]
                        .batch_indicator(&[chunk], Some(&[true, true]))
                        .expect("batch_indicator failed");
                    for k in 0..4 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = states[asset_idx]
                        .batch_indicator(&[rem], Some(&[true, true]))
                        .expect("batch_indicator failed on remainder");
                    for k in 0..4 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }

                let (scalar_out, _) =
                    mama_indicator(&[close.as_slice()], &options, Some(&[true, true]))
                        .expect("scalar failed");

                for k in 0..4 {
                    let label = OUTPUT_LABELS[k];
                    assert_eq!(
                        batch[k].len(),
                        scalar_out[k].len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&bv, &sv)) in batch[k].iter().zip(scalar_out[k].iter()).enumerate() {
                        if bv.is_nan() {
                            panic!(
                                "SIMD by_assets {label} has NaN at {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if bv.is_infinite() {
                            panic!(
                                "SIMD by_assets {label} has Inf at {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        let diff = (bv - sv).abs();
                        assert!(
                            diff < 1e-4,
                            "{label} mismatch at {i}: simd+batch={bv}, scalar={sv}, \
                             diff={diff:.2e}, stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_options state continuity: SIMD seed (first chunk) + scalar
    // batch_indicator (remainder) must match full scalar run within 1e-4 for
    // all four outputs.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_mama_simd_by_options_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let first_inputs = [&close[..FIRST_CHUNK] as &[f64]];

            let (simd_first, mut states) =
                indicator_by_options::<4>(&first_inputs, &options_4, Some(&[true, true]))
                    .expect("SIMD by_options failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch = [
                    simd_first[opt_idx][0].clone(),
                    simd_first[opt_idx][1].clone(),
                    simd_first[opt_idx][2].clone(),
                    simd_first[opt_idx][3].clone(),
                ];

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = states[opt_idx]
                        .batch_indicator(&[chunk], Some(&[true, true]))
                        .expect("batch_indicator failed");
                    for k in 0..4 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = states[opt_idx]
                        .batch_indicator(&[rem], Some(&[true, true]))
                        .expect("batch_indicator failed on remainder");
                    for k in 0..4 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }

                let (scalar_out, _) =
                    mama_indicator(&[close.as_slice()], options, Some(&[true, true]))
                        .expect("scalar failed");

                for k in 0..4 {
                    let label = OUTPUT_LABELS[k];
                    assert_eq!(
                        batch[k].len(),
                        scalar_out[k].len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&bv, &sv)) in batch[k].iter().zip(scalar_out[k].iter()).enumerate() {
                        if bv.is_nan() {
                            panic!(
                                "SIMD by_options {label} has NaN at {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if bv.is_infinite() {
                            panic!(
                                "SIMD by_options {label} has Inf at {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        let diff = (bv - sv).abs();
                        assert!(
                            diff < 1e-4,
                            "{label} mismatch at {i}: simd+batch={bv}, scalar={sv}, \
                             diff={diff:.2e}, stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    }
