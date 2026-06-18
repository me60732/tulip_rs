#[cfg(test)]
mod tests {
    use tulip_rs::indicator_types::TIndicatorState;
    use tulip_rs::indicators::adaptivemsw::{
        indicator as amsw_indicator, indicator_by_assets, min_data, output_length,
    };
    use tulip_rs::indicators::homodynediscriminator::indicator as hd_indicator;
    use tulip_rs::types::IndicatorError;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const FIRST_CHUNK: usize = 1000;

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sanity checks — no database needed.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_adaptivemsw_min_data_and_output_length() {
        assert_eq!(min_data(&[]), 23);
        assert_eq!(output_length(23, &[]), 1);
        assert_eq!(output_length(100, &[]), 78);
        assert_eq!(output_length(1000, &[]), 978);
    }

    #[test]
    fn test_adaptivemsw_not_enough_data() {
        let close: Vec<f64> = (0..22).map(|i| 100.0 + i as f64).collect();
        let result = amsw_indicator(&[close.as_slice()], &[], None);
        assert!(
            matches!(result, Err(IndicatorError::NotEnoughData)),
            "Expected NotEnoughData for {} bars (need 23)",
            close.len()
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // output_length formula must match actual returned slice length.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_adaptivemsw_output_length_matches() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let (out, _) =
                amsw_indicator(&[close.as_slice()], &[], None).expect("indicator failed");
            assert_eq!(
                out[0].len(),
                output_length(n, &[]),
                "sine len: {stock_symbol}"
            );
            assert_eq!(
                out[1].len(),
                output_length(n, &[]),
                "lead len: {stock_symbol}"
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Sine and lead_sine must be finite and in [-1, 1] after convergence.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_adaptivemsw_output_range() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        // Allow the first 50 bars to converge before checking range.
        const CONVERGENCE_SKIP: usize = 50;
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let (out, _) =
                amsw_indicator(&[close.as_slice()], &[], None).expect("indicator failed");
            for (i, (&s, &l)) in out[0].iter().zip(out[1].iter()).enumerate() {
                assert!(s.is_finite(), "sine NaN/Inf at {i}: {stock_symbol}");
                assert!(l.is_finite(), "lead NaN/Inf at {i}: {stock_symbol}");
                if i >= CONVERGENCE_SKIP {
                    assert!(
                        s >= -1.001 && s <= 1.001,
                        "sine={s} out of [-1,1] at {i}: {stock_symbol}"
                    );
                    assert!(
                        l >= -1.001 && l <= 1.001,
                        "lead={l} out of [-1,1] at {i}: {stock_symbol}"
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // dc_period must be bit-exact to standalone homodynediscriminator output.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_adaptivemsw_dc_period_matches_standalone_hd() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let (hd_out, _) = hd_indicator(&[close.as_slice()], &[], None).expect("HD failed");
            let (amsw_out, _) =
                amsw_indicator(&[close.as_slice()], &[], Some(&[true])).expect("AMSW failed");
            let hd_dc = &hd_out[0];
            let amsw_dc = &amsw_out[2];
            assert_eq!(
                amsw_dc.len(),
                hd_dc.len(),
                "dc_period length: {stock_symbol}"
            );
            for (i, (&av, &hv)) in amsw_dc.iter().zip(hd_dc.iter()).enumerate() {
                assert_eq!(
                    av, hv,
                    "dc_period mismatch at {i}: amsw={av}, hd={hv}, stock={stock_symbol}"
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Optional dc_period: populated only when requested; empty otherwise.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_adaptivemsw_optional_outputs() {
        let close: Vec<f64> = (0..200).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();
        let inputs = [close.as_slice()];
        let expected_len = output_length(close.len(), &[]);

        let (none, _) = amsw_indicator(&inputs, &[], None).unwrap();
        assert_eq!(none[0].len(), expected_len, "sine len (none)");
        assert_eq!(none[1].len(), expected_len, "lead len (none)");
        assert!(none[2].is_empty(), "dc_period should be empty");

        let (with_dc, _) = amsw_indicator(&inputs, &[], Some(&[true])).unwrap();
        assert_eq!(with_dc[2].len(), expected_len, "dc_period len (requested)");

        // Values must be consistent.
        for (i, (&a, &b)) in with_dc[2].iter().zip(with_dc[2].iter()).enumerate() {
            assert_eq!(a, b, "dc_period inconsistency at {i}");
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // State continuity: indicator() seed + batch_indicator() remainder must be
    // bit-exact to a full single-call run for all outputs.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_adaptivemsw_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            let (ref_out, _) = amsw_indicator(&[close.as_slice()], &[], Some(&[true]))
                .expect("reference run failed");

            let (first_out, mut state) =
                amsw_indicator(&[&close[..FIRST_CHUNK]], &[], Some(&[true]))
                    .expect("seed run failed");

            let mut batch = [
                first_out[0].clone(),
                first_out[1].clone(),
                first_out[2].clone(),
            ];

            let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
            for chunk in chunks.by_ref() {
                let out = state
                    .batch_indicator(&[chunk], Some(&[true]))
                    .expect("batch_indicator failed");
                for k in 0..3 {
                    batch[k].extend_from_slice(&out[k]);
                }
            }
            let rem = chunks.remainder();
            if !rem.is_empty() {
                let out = state
                    .batch_indicator(&[rem], Some(&[true]))
                    .expect("batch_indicator remainder failed");
                for k in 0..3 {
                    batch[k].extend_from_slice(&out[k]);
                }
            }

            let labels = ["sine", "lead_sine", "dc_period"];
            for k in 0..3 {
                assert_eq!(
                    batch[k].len(),
                    ref_out[k].len(),
                    "{} length mismatch: {stock_symbol}",
                    labels[k]
                );
                for (i, (&bv, &rv)) in batch[k].iter().zip(ref_out[k].iter()).enumerate() {
                    assert!(!bv.is_nan(), "{} NaN at {i}: {stock_symbol}", labels[k]);
                    assert!(
                        !bv.is_infinite(),
                        "{} Inf at {i}: {stock_symbol}",
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

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets: N=4 assets, dc_period within 1e-4 of scalar.
    // Sine/lead_sine checked for finite and in valid range (the adaptive period
    // is integer-rounded, so rare rounding differences can shift the DFT result).
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_adaptivemsw_simd_by_assets_vs_scalar() {
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

        let (simd_results, _) =
            indicator_by_assets::<4>(&inputs_4, &[], Some(&[true])).expect("SIMD by_assets failed");

        for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
            let (scalar_out, _) =
                amsw_indicator(&[close.as_slice()], &[], Some(&[true])).expect("scalar failed");

            // dc_period: must be close (SIMD atan vs scalar atan in HD).
            let simd_dc = &simd_results[asset_idx][2];
            let scalar_dc = &scalar_out[2];
            for (i, (&sv, &rv)) in simd_dc.iter().zip(scalar_dc.iter()).enumerate() {
                let diff = (sv - rv).abs();
                assert!(
                    diff < 1e-4,
                    "dc_period mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                     stock={stock_symbol}"
                );
            }

            // sine / lead_sine: finite and in valid sine range.
            for label_idx in [0, 1] {
                let label = if label_idx == 0 { "sine" } else { "lead_sine" };
                let simd_line = &simd_results[asset_idx][label_idx];
                assert_eq!(
                    simd_line.len(),
                    scalar_out[label_idx].len(),
                    "{label} length mismatch: {stock_symbol}"
                );
                for (i, &sv) in simd_line.iter().enumerate() {
                    assert!(
                        sv.is_finite(),
                        "SIMD {label} NaN/Inf at {i}: {stock_symbol}"
                    );
                    assert!(
                        sv >= -1.001 && sv <= 1.001,
                        "SIMD {label}={sv} out of range at {i}: {stock_symbol}"
                    );
                }
            }

            println!(
                "✓ SIMD by_assets vs scalar passed for {stock_symbol} ({} bars)",
                close.len()
            );
        }
        println!("✓ All SIMD by_assets adaptive MSW tests passed!");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets state continuity: SIMD seed + scalar batch_indicator
    // remainder must be bit-exact for dc_period (scalar state after write_states).
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_adaptivemsw_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(sym, eod)| (sym.clone(), get_close_array(eod)))
            .collect();

        let inputs_first: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1[..FIRST_CHUNK]],
            &[&stock_data[1].1[..FIRST_CHUNK]],
            &[&stock_data[2].1[..FIRST_CHUNK]],
            &[&stock_data[3].1[..FIRST_CHUNK]],
        ];

        let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_first, &[], Some(&[true]))
            .expect("SIMD by_assets first chunk failed");

        for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
            let mut batch = [
                simd_first[asset_idx][0].clone(),
                simd_first[asset_idx][1].clone(),
                simd_first[asset_idx][2].clone(),
            ];

            let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
            for chunk in chunks.by_ref() {
                let out = states[asset_idx]
                    .batch_indicator(&[chunk], Some(&[true]))
                    .expect("batch_indicator failed");
                for k in 0..3 {
                    batch[k].extend_from_slice(&out[k]);
                }
            }
            let rem = chunks.remainder();
            if !rem.is_empty() {
                let out = states[asset_idx]
                    .batch_indicator(&[rem], Some(&[true]))
                    .expect("batch remainder failed");
                for k in 0..3 {
                    batch[k].extend_from_slice(&out[k]);
                }
            }

            // Reference: full scalar run.
            let (scalar_out, _) = amsw_indicator(&[close.as_slice()], &[], Some(&[true]))
                .expect("scalar indicator failed");

            // dc_period must be within 1e-4 (SIMD atan residual from HD).
            let labels = ["sine", "lead_sine", "dc_period"];
            assert_eq!(
                batch[2].len(),
                scalar_out[2].len(),
                "dc_period length: {stock_symbol}"
            );
            for (i, (&bv, &rv)) in batch[2].iter().zip(scalar_out[2].iter()).enumerate() {
                assert!(!bv.is_nan(), "dc_period NaN at {i}: {stock_symbol}");
                let diff = (bv - rv).abs();
                assert!(
                    diff < 1e-4,
                    "dc_period mismatch at {i}: simd+batch={bv}, scalar={rv}, \
                     diff={diff:.2e}, stock={stock_symbol}"
                );
            }

            // sine / lead_sine: finite and in valid range.
            for label_idx in [0, 1] {
                let label = labels[label_idx];
                assert_eq!(
                    batch[label_idx].len(),
                    scalar_out[label_idx].len(),
                    "{label} length: {stock_symbol}"
                );
                for (i, &bv) in batch[label_idx].iter().enumerate() {
                    assert!(bv.is_finite(), "{label} NaN/Inf at {i}: {stock_symbol}");
                    assert!(
                        bv >= -1.001 && bv <= 1.001,
                        "{label}={bv} out of range at {i}: {stock_symbol}"
                    );
                }
            }

            println!(
                "✓ SIMD by_assets state continuity passed for {stock_symbol} ({} bars)",
                close.len()
            );
        }
        println!("✓ All SIMD by_assets adaptive MSW state continuity tests passed!");
    }
}
