#[cfg(test)]
mod tests {
    use tulip_rs::indicator_types::{Indicator, IndicatorByOptions, TIndicatorState};
    use tulip_rs::indicators::cybercycle::{multiplier, Cybercycle};
    use tulip_rs::types::IndicatorError;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const FIRST_CHUNK: usize = 1000;
    const OPTIONS_LIST: [[f64; 1]; 4] = [[0.05], [0.07], [0.10], [0.15]];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // min_data / output_length sanity (no database needed)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_min_data_and_output_length() {
        // min_data is option-independent
        assert_eq!(Cybercycle::min_data(&[0.07]), 7, "min_data must be 7");
        assert_eq!(Cybercycle::min_data(&[0.05]), 7, "min_data must be 7");

        // output_length = data_len - 6
        assert_eq!(
            Cybercycle::output_length(7, &[0.07]),
            1,
            "Cybercycle::output_length(7) must be 1"
        );
        assert_eq!(
            Cybercycle::output_length(100, &[0.07]),
            94,
            "Cybercycle::output_length(100) must be 94"
        );
        assert_eq!(
            Cybercycle::output_length(1000, &[0.07]),
            994,
            "Cybercycle::output_length(1000) must be 994"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // NotEnoughData error
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_not_enough_data() {
        let close: Vec<f64> = (0..6).map(|i| 100.0 + i as f64).collect();
        let result = Cybercycle::indicator(&[close.as_slice()], &[0.07], None);
        assert!(
            matches!(result, Err(IndicatorError::NotEnoughData)),
            "Expected NotEnoughData for {} bars (need 7), got {:?}",
            close.len(),
            result.map(|_| ())
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Invalid alpha values
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_invalid_alpha() {
        let close: Vec<f64> = (0..100).map(|i| 100.0 + i as f64).collect();
        let inputs = [close.as_slice()];

        for bad_alpha in [0.0_f64, -0.1, 1.0, 1.5] {
            assert!(
                matches!(
                    Cybercycle::indicator(&inputs, &[bad_alpha], None),
                    Err(IndicatorError::InvalidOptions)
                ),
                "alpha={bad_alpha} should be InvalidOptions"
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // output slice length == Cybercycle::output_length(data_len, options)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_output_length_matches() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            for options in OPTIONS_LIST {
                let (out, _) = Cybercycle::indicator(&[close.as_slice()], &options, None)
                    .expect("indicator failed");
                assert_eq!(
                    out[0].len(),
                    Cybercycle::output_length(n, &options),
                    "length mismatch: stock={stock_symbol}, alpha={:?}",
                    options
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // No NaN or Inf in scalar output
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_scalar_no_nan_or_inf() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];
            for options in OPTIONS_LIST {
                let (out, _) =
                    Cybercycle::indicator(&inputs, &options, None).expect("CyberCycle failed");
                for (i, &v) in out[0].iter().enumerate() {
                    assert!(
                        v.is_finite(),
                        "cybercycle NaN/Inf at {i}: stock={stock_symbol}, alpha={:?}",
                        options
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Trigger formula: trigger[i] == cybercycle[i-1] for all i > 0
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_trigger_formula() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            for options in OPTIONS_LIST {
                let (out, _) = Cybercycle::indicator(&[close.as_slice()], &options, Some(&[true]))
                    .expect("failed");
                let cycle = &out[0];
                let trigger = &out[1];
                for i in 1..cycle.len() {
                    let diff = (trigger[i] - cycle[i - 1]).abs();
                    assert!(
                        diff < 1e-12,
                        "trigger[{i}]={} != cycle[{}]={} (diff={:.2e}): \
                         stock={stock_symbol}, alpha={:?}",
                        trigger[i],
                        i - 1,
                        cycle[i - 1],
                        diff,
                        options
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // High-pass property: mean of output ≈ 0 over a long stationary series
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_oscillator_zero_mean() {
        // 2000 bars of a sine-wave price around 100 — should produce near-zero mean.
        let n = 2000_usize;
        let close: Vec<f64> = (0..n)
            .map(|i| 100.0 + 5.0 * (2.0 * std::f64::consts::PI * i as f64 / 20.0).sin())
            .collect();
        let inputs = [close.as_slice()];

        for options in OPTIONS_LIST {
            let (out, _) = Cybercycle::indicator(&inputs, &options, None).expect("failed");
            let mean: f64 = out[0].iter().sum::<f64>() / out[0].len() as f64;
            assert!(
                mean.abs() < 0.5,
                "cybercycle mean={mean:.4} exceeds threshold 0.5 for alpha={:?}",
                options
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Optional outputs: populated only when requested; empty otherwise
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_optional_outputs() {
        let close: Vec<f64> = (0..200).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();
        let inputs = [close.as_slice()];
        let expected_len = Cybercycle::output_length(close.len(), &[0.07]);

        // None requested
        let (none, _) = Cybercycle::indicator(&inputs, &[0.07], None).unwrap();
        assert_eq!(none[0].len(), expected_len, "cybercycle len (none)");
        assert!(none[1].is_empty(), "trigger should be empty (none)");

        // Trigger requested
        let (with_trigger, _) = Cybercycle::indicator(&inputs, &[0.07], Some(&[true])).unwrap();
        assert_eq!(with_trigger[0].len(), expected_len, "cybercycle len");
        assert_eq!(with_trigger[1].len(), expected_len, "trigger len");

        // Values are the same regardless of optional flag (no interference)
        for (i, (&a, &b)) in with_trigger[0].iter().zip(none[0].iter()).enumerate() {
            assert_eq!(a, b, "cybercycle value changed at {i} when trigger added");
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // multiplier consistency
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_multiplier() {
        let alpha = 0.07_f64;
        let (coeff, d1, d2) = multiplier(alpha);
        let c = 1.0 - 0.5 * alpha;
        let b = 1.0 - alpha;
        assert!((coeff - c * c).abs() < 1e-15, "coeff mismatch");
        assert!((d1 - 2.0 * b).abs() < 1e-15, "d1 mismatch");
        assert!((d2 - b * b).abs() < 1e-15, "d2 mismatch");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // State continuity: indicator() first chunk + batch_indicator() remainder
    // must be bit-exact to a full single-call run (both outputs).
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let (ref_out, _) =
                    Cybercycle::indicator(&[close.as_slice()], &options, Some(&[true]))
                        .expect("ref run");

                let (first_out, mut state) =
                    Cybercycle::indicator(&[&close[..FIRST_CHUNK]], &options, Some(&[true]))
                        .expect("seed run");

                let mut batch = [first_out[0].clone(), first_out[1].clone()];

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = state
                        .batch_indicator(&[chunk], Some(&[true]))
                        .expect("batch_indicator failed");
                    for k in 0..2 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = state
                        .batch_indicator(&[rem], Some(&[true]))
                        .expect("remainder failed");
                    for k in 0..2 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }

                let labels = ["cybercycle", "trigger"];
                for k in 0..2 {
                    assert_eq!(
                        batch[k].len(),
                        ref_out[k].len(),
                        "{} length mismatch: stock={stock_symbol}, alpha={:?}",
                        labels[k],
                        options
                    );
                    for (i, (&bv, &rv)) in batch[k].iter().zip(ref_out[k].iter()).enumerate() {
                        assert!(
                            bv.is_finite(),
                            "{} NaN/Inf at {i}: stock={stock_symbol}",
                            labels[k]
                        );
                        assert_eq!(
                            bv, rv,
                            "{} mismatch at {i}: batch={bv}, ref={rv}, stock={stock_symbol}",
                            labels[k]
                        );
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets: N=4 assets processed in a single SIMD pass must match
    // the scalar run within 1e-10.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_simd_by_assets_vs_scalar() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| (sym.clone(), get_close_array(eod)))
            .collect();

        for options in OPTIONS_LIST {
            let inputs_4: [&[&[f64]; 1]; 4] = [
                &[&stock_data[0].1],
                &[&stock_data[1].1],
                &[&stock_data[2].1],
                &[&stock_data[3].1],
            ];

            let (simd_results, _) =
                Cybercycle::indicator_by_assets::<4>(&inputs_4, &options, Some(&[true]))
                    .expect("SIMD by_assets failed");

            let labels = ["cybercycle", "trigger"];
            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let (scalar_out, _) =
                    Cybercycle::indicator(&[close.as_slice()], &options, Some(&[true]))
                        .expect("scalar");

                for k in 0..2 {
                    let simd_line = &simd_results[asset_idx][k];
                    let scalar_line = &scalar_out[k];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{} length mismatch: stock={stock_symbol}, alpha={:?}",
                        labels[k],
                        options
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        assert!(
                            sv.is_finite(),
                            "SIMD {} NaN/Inf at {i}: stock={stock_symbol}",
                            labels[k]
                        );
                        let diff = (sv - rv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                             stock={stock_symbol}, alpha={:?}",
                            labels[k],
                            options
                        );
                    }
                }
            }
        }
        println!("✓ SIMD by_assets vs scalar CyberCycle passed for all options");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets state continuity: SIMD first chunk + scalar batch_indicator
    // remainder must match full scalar run within 1e-10.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_simd_by_assets_state_continuity() {
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

            let (simd_first, mut states) =
                Cybercycle::indicator_by_assets::<4>(&inputs_first, &options, Some(&[true]))
                    .expect("SIMD first chunk failed");

            let labels = ["cybercycle", "trigger"];
            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let mut batch = [
                    simd_first[asset_idx][0].clone(),
                    simd_first[asset_idx][1].clone(),
                ];

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = states[asset_idx]
                        .batch_indicator(&[chunk], Some(&[true]))
                        .expect("batch_indicator failed");
                    for k in 0..2 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = states[asset_idx]
                        .batch_indicator(&[rem], Some(&[true]))
                        .expect("remainder failed");
                    for k in 0..2 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }

                let (scalar_out, _) =
                    Cybercycle::indicator(&[close.as_slice()], &options, Some(&[true]))
                        .expect("scalar");

                for k in 0..2 {
                    assert_eq!(
                        batch[k].len(),
                        scalar_out[k].len(),
                        "{} length mismatch: stock={stock_symbol}, alpha={:?}",
                        labels[k],
                        options
                    );
                    for (i, (&bv, &rv)) in batch[k].iter().zip(scalar_out[k].iter()).enumerate() {
                        assert!(
                            bv.is_finite(),
                            "{} NaN/Inf at {i}: stock={stock_symbol}, alpha={:?}",
                            labels[k],
                            options
                        );
                        let diff = (bv - rv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i}: simd+batch={bv}, scalar={rv}, diff={diff:.2e}, \
                             stock={stock_symbol}, alpha={:?}",
                            labels[k],
                            options
                        );
                    }
                }
                println!(
                    "✓ SIMD state continuity passed: {stock_symbol} ({} bars), alpha={:?}",
                    close.len(),
                    options
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_options: N=4 alpha values on one asset must match scalar runs.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_simd_by_options_vs_scalar() {
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

            let (simd_results, _) =
                Cybercycle::indicator_by_options::<4>(&inputs, &options_4, Some(&[true]))
                    .expect("SIMD by_options failed");

            let labels = ["cybercycle", "trigger"];
            for (lane, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_out, _) =
                    Cybercycle::indicator(&inputs, options, Some(&[true])).expect("scalar failed");

                for k in 0..2 {
                    let simd_line = &simd_results[lane][k];
                    let scalar_line = &scalar_out[k];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{} length mismatch: stock={stock_symbol}, alpha={:?}",
                        labels[k],
                        options
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        assert!(
                            sv.is_finite(),
                            "SIMD by_options {} NaN/Inf at {i}: stock={stock_symbol}",
                            labels[k]
                        );
                        let diff = (sv - rv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                             stock={stock_symbol}, alpha={:?}",
                            labels[k],
                            options
                        );
                    }
                }
            }
            println!("✓ SIMD by_options vs scalar CyberCycle passed for {stock_symbol}");
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_options state continuity.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_cybercycle_simd_by_options_state_continuity() {
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

            let inputs_first = [&close[..FIRST_CHUNK] as &[f64]];
            let (simd_first, mut states) =
                Cybercycle::indicator_by_options::<4>(&inputs_first, &options_4, Some(&[true]))
                    .expect("SIMD by_options first chunk failed");

            let labels = ["cybercycle", "trigger"];
            for (lane, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch = [simd_first[lane][0].clone(), simd_first[lane][1].clone()];

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = states[lane]
                        .batch_indicator(&[chunk], Some(&[true]))
                        .expect("batch_indicator failed");
                    for k in 0..2 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = states[lane]
                        .batch_indicator(&[rem], Some(&[true]))
                        .expect("remainder failed");
                    for k in 0..2 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }

                let inputs_full = [close.as_slice()];
                let (scalar_out, _) = Cybercycle::indicator(&inputs_full, options, Some(&[true]))
                    .expect("scalar failed");

                for k in 0..2 {
                    assert_eq!(
                        batch[k].len(),
                        scalar_out[k].len(),
                        "{} length mismatch lane {lane}: stock={stock_symbol}, alpha={:?}",
                        labels[k],
                        options
                    );
                    for (i, (&bv, &rv)) in batch[k].iter().zip(scalar_out[k].iter()).enumerate() {
                        assert!(
                            bv.is_finite(),
                            "{} NaN/Inf at {i} lane {lane}: stock={stock_symbol}, alpha={:?}",
                            labels[k],
                            options
                        );
                        let diff = (bv - rv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i} lane {lane}: simd+batch={bv}, scalar={rv}, \
                             diff={diff:.2e}, stock={stock_symbol}, alpha={:?}",
                            labels[k],
                            options
                        );
                    }
                }
            }
            println!("✓ SIMD by_options state continuity passed for {stock_symbol}");
        }
    }
}
