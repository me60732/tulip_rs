#[cfg(test)]
mod tests {
    use tulip_rs::indicator_types::{Indicator, TIndicatorState};
    use tulip_rs::indicators::ccfisher::CcFisher;
    use tulip_rs::indicators::cybercycle::Cybercycle;
    use tulip_rs::types::IndicatorError;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const FIRST_CHUNK: usize = 1000;
    const OPTIONS_LIST: [[f64; 1]; 4] = [[0.05], [0.07], [0.10], [0.0]];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // min_data / output_length sanity (no database needed)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_min_data() {
        assert_eq!(CcFisher::min_data(&[0.07]), 56, "min_data must be 56");

        assert_eq!(CcFisher::output_length(56, &[0.07]), 1, "CcFisher::output_length(56) must be 1");
        assert_eq!(
            CcFisher::output_length(100, &[0.07]),
            45,
            "CcFisher::output_length(100) must be 45"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // NotEnoughData error
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_not_enough_data() {
        let close: Vec<f64> = (0..55).map(|i| 100.0 + i as f64).collect();
        let result = CcFisher::indicator(&[close.as_slice()], &[0.07], None);
        assert!(
            matches!(result, Err(IndicatorError::NotEnoughData)),
            "Expected NotEnoughData for {} bars (need 56), got {:?}",
            close.len(),
            result.map(|_| ())
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Invalid alpha values
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_invalid_alpha() {
        let close: Vec<f64> = (0..100).map(|i| 100.0 + i as f64).collect();
        let inputs = [close.as_slice()];

        // 0.0 is now valid (adaptive mode); only truly invalid values remain.
        for bad_alpha in [-0.1_f64, 1.0, 1.5] {
            assert!(
                matches!(
                    CcFisher::indicator(&inputs, &[bad_alpha], None),
                    Err(IndicatorError::InvalidOptions)
                ),
                "alpha={bad_alpha} should be InvalidOptions"
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Adaptive alpha (alpha=0.0): fisher/signal are finite; trendmode in {0,1}
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_adaptive_alpha() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            let (out, _) = CcFisher::indicator(&inputs, &[0.0], Some(&[true, true, true]))
                .expect("CCFisher adaptive failed");

            // fisher and signal: finite
            for k in 0..2 {
                let label = if k == 0 { "fisher" } else { "signal" };
                for (i, &v) in out[k].iter().enumerate() {
                    assert!(
                        v.is_finite(),
                        "adaptive {label}[{i}]={v} NaN/Inf: stock={stock_symbol}"
                    );
                }
            }
            // trendmode: exactly 0.0 or 1.0
            for (i, &v) in out[2].iter().enumerate() {
                assert!(
                    v == 0.0 || v == 1.0,
                    "adaptive trendmode[{i}]={v} not in {{0.0,1.0}}: stock={stock_symbol}"
                );
            }
            // cycle and peak: finite, peak >= 0
            for (i, &v) in out[3].iter().enumerate() {
                assert!(
                    v.is_finite(),
                    "adaptive cycle[{i}]={v} NaN/Inf: stock={stock_symbol}"
                );
            }
            for (i, &v) in out[4].iter().enumerate() {
                assert!(
                    v.is_finite() && v >= 0.0,
                    "adaptive peak[{i}]={v} invalid: stock={stock_symbol}"
                );
            }

            // Signal must still be a 1-bar lag of fisher in adaptive mode.
            let fisher = &out[0];
            let signal = &out[1];
            for i in 1..fisher.len() {
                let diff = (signal[i] - fisher[i - 1]).abs();
                assert!(
                    diff < 1e-14,
                    "adaptive signal[{i}] != fisher[{}] (diff={diff:.2e}): stock={stock_symbol}",
                    i - 1
                );
            }

            // Adaptive must differ from fixed alpha=0.07.
            let (fixed_out, _) = CcFisher::indicator(&inputs, &[0.07], None).expect("fixed run");
            let differ = out[0]
                .iter()
                .zip(fixed_out[0].iter())
                .any(|(&a, &b)| a != b);
            assert!(
                differ,
                "adaptive fisher == fixed fisher — adaptive may not be working: \
                 stock={stock_symbol}"
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // No NaN or Inf in any output (fisher, signal, trendmode, cycle, peak)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_no_nan_or_inf() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];
            for options in OPTIONS_LIST {
                let (out, _) = CcFisher::indicator(&inputs, &options, Some(&[true, true, true]))
                    .expect("CCFisher failed");
                let labels = ["fisher", "signal", "trendmode", "cycle", "peak"];
                for k in 0..5 {
                    for (i, &v) in out[k].iter().enumerate() {
                        assert!(
                            v.is_finite(),
                            "{} NaN/Inf at {i}: stock={stock_symbol}, alpha={:?}",
                            labels[k],
                            options
                        );
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Optional trendmode output (out[2]) must be exactly 0.0 or 1.0
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_trendmode_values() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];
            for options in OPTIONS_LIST {
                let (out, _) = CcFisher::indicator(&inputs, &options, Some(&[true, false, false]))
                    .expect("CCFisher failed");
                for (i, &v) in out[2].iter().enumerate() {
                    assert!(
                        v == 0.0 || v == 1.0,
                        "trendmode[{i}]={v} not in {{0.0, 1.0}}: \
                         stock={stock_symbol}, alpha={:?}",
                        options
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Optional cycle output (out[3]) matches cc_indicator output[0]
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_optional_cycle() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];
            for options in OPTIONS_LIST {
                // Adaptive mode (alpha=0.0): CCFisher derives alpha from HD per bar;
                // standalone cc_indicator has no HD, so the cycles diverge — skip.
                if options[0] == 0.0 {
                    continue;
                }
                let (cf_out, _) = CcFisher::indicator(&inputs, &options, Some(&[false, true, false]))
                    .expect("CCFisher failed");
                let (cc_out, _) = Cybercycle::indicator(&inputs, &options, None).expect("CyberCycle failed");

                // CCFisher outputs start at bar 55 (min_data=56, output_length=n-55).
                // CyberCycle outputs start at bar 6 (min_data=7, output_length=n-6).
                // So CCFisher output[i] corresponds to CyberCycle output[i + 49].
                let cc_offset = CcFisher::min_data(&options) - Cybercycle::min_data(&options);
                let cycle_from_cf = &cf_out[3];
                let cycle_from_cc = &cc_out[0][cc_offset..];

                assert_eq!(
                    cycle_from_cf.len(),
                    cycle_from_cc.len(),
                    "cycle length mismatch after alignment: stock={stock_symbol}, alpha={:?}",
                    options
                );
                for (i, (&cfv, &ccv)) in cycle_from_cf.iter().zip(cycle_from_cc.iter()).enumerate()
                {
                    let diff = (cfv - ccv).abs();
                    assert!(
                        diff < 1e-10,
                        "cycle mismatch at {i}: ccfisher_cycle={cfv}, cc={ccv}, \
                         diff={diff:.2e}, stock={stock_symbol}, alpha={:?}",
                        options
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Optional peak output (out[4]): non-negative and finite
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_optional_peak() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];
            for options in OPTIONS_LIST {
                let (out, _) = CcFisher::indicator(&inputs, &options, Some(&[false, false, true]))
                    .expect("CCFisher failed");
                let peak = &out[4];
                for (i, &v) in peak.iter().enumerate() {
                    assert!(
                        v.is_finite(),
                        "peak NaN/Inf at {i}: stock={stock_symbol}, alpha={:?}",
                        options
                    );
                    assert!(
                        v >= 0.0,
                        "peak[{i}]={v} < 0: stock={stock_symbol}, alpha={:?}",
                        options
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Signal (out[1]) is a 1-bar lag of fisher (out[0])
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_signal_is_lagged_fisher() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];
            for options in OPTIONS_LIST {
                let (out, _) = CcFisher::indicator(&inputs, &options, None).expect("CCFisher failed");
                let fisher = &out[0];
                let signal = &out[1];
                for i in 1..fisher.len() {
                    let a = signal[i];
                    let b = fisher[i - 1];
                    assert!(
                        (a - b).abs() < 1e-14,
                        "signal[{i}]={a} != fisher[{}]={b} (diff={:.2e}): \
                         stock={stock_symbol}, alpha={:?}",
                        i - 1,
                        (a - b).abs(),
                        options
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // State continuity: CcFisher::indicator() first chunk + batch_indicator() remainder
    // must be bit-exact to a full single-call run (all five outputs).
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                let (ref_out, _) =
                    CcFisher::indicator(&[close.as_slice()], &options, Some(&[true, true, true]))
                        .expect("ref run");

                let (first_out, mut state) = CcFisher::indicator(
                    &[&close[..FIRST_CHUNK]],
                    &options,
                    Some(&[true, true, true]),
                )
                .expect("seed run");

                let mut batch = [
                    first_out[0].clone(), // fisher
                    first_out[1].clone(), // signal
                    first_out[2].clone(), // trendmode
                    first_out[3].clone(), // cycle
                    first_out[4].clone(), // peak
                ];

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = state
                        .batch_indicator(&[chunk], Some(&[true, true, true]))
                        .expect("batch_indicator failed");
                    for k in 0..5 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = state
                        .batch_indicator(&[rem], Some(&[true, true, true]))
                        .expect("remainder failed");
                    for k in 0..5 {
                        batch[k].extend_from_slice(&out[k]);
                    }
                }

                let labels = ["fisher", "signal", "trendmode", "cycle", "peak"];
                for k in 0..5 {
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
                            "{} mismatch at {i}: batch={bv}, ref={rv}, \
                             stock={stock_symbol}, alpha={:?}",
                            labels[k], options
                        );
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets: N=4 assets processed in a single SIMD pass must match
    // the scalar run within 1e-10 (fisher and signal outputs).
    // ─────────────────────────────────────────────────────────────────────────

    /*#[test]
    fn test_ccfisher_simd_by_assets() {
        use tulip_rs::indicators::ccfisher::indicator_by_assets;

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
                indicator_by_assets::<4>(&inputs_4, &options, None).expect("SIMD by_assets failed");

            let labels = ["fisher", "signal"];
            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let (scalar_out, _) =
                    CcFisher::indicator(&[close.as_slice()], &options, None).expect("scalar");

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
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_options mixed: [0.0, 0.07, 0.10, 0.0] — adaptive + fixed lanes
    // Each lane must match the scalar run with the same alpha within 1e-10.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_simd_by_options_mixed_adaptive() {
        use tulip_rs::indicators::ccfisher::indicator_by_options;

        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Lanes 0 and 3 are adaptive (0.0); lanes 1 and 2 are fixed.
        let mixed_options: [&[f64; 1]; 4] = [&[0.0], &[0.07], &[0.10], &[0.0]];
        let scalar_alphas = [[0.0_f64], [0.07_f64], [0.10_f64], [0.0_f64]];

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &mixed_options, None)
                .expect("SIMD by_options mixed failed");

            let labels = ["fisher", "signal"];
            for (lane, alpha) in scalar_alphas.iter().enumerate() {
                let (scalar_out, _) = CcFisher::indicator(&inputs, alpha, None).expect("scalar failed");

                for k in 0..2 {
                    let simd_line = &simd_results[lane][k];
                    let scalar_line = &scalar_out[k];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{} length mismatch: lane={lane} stock={stock_symbol}",
                        labels[k]
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        assert!(
                            sv.is_finite(),
                            "SIMD mixed {} NaN/Inf at {i}: lane={lane} stock={stock_symbol}",
                            labels[k]
                        );
                        let diff = (sv - rv).abs();
                        assert!(
                            diff < 1e-10,
                            "{} mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                             lane={lane} stock={stock_symbol}",
                            labels[k]
                        );
                    }
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_options: N=4 alpha values on one asset must match scalar runs
    // within 1e-10 (fisher and signal outputs).
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_ccfisher_simd_by_options() {
        use tulip_rs::indicators::ccfisher::indicator_by_options;

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

            let labels = ["fisher", "signal"];
            for (lane, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_out, _) = CcFisher::indicator(&inputs, options, None).expect("scalar failed");

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
        }
    }*/
}
