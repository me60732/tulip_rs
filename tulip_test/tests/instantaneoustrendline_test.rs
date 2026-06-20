#[cfg(test)]
mod tests {
    use tulip_rs::indicator_types::TIndicatorState;
    use tulip_rs::indicators::homodynediscriminator::indicator as hd_indicator;
    use tulip_rs::indicators::instantaneoustrendline::{
        indicator as it_indicator, indicator_by_assets, min_data, output_length,
    };
    use tulip_rs::types::IndicatorError;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const FIRST_CHUNK: usize = 1000;

    /// Indices into the outputs vec: [trendline, trigger, dc_period, alpha]
    const OUTPUT_LABELS: [&str; 4] = ["trendline", "trigger", "dc_period", "alpha"];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Basic sanity checks — no database needed.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_min_data_and_output_length() {
        assert_eq!(min_data(&[]), 23, "min_data must be 23");
        assert_eq!(output_length(23, &[]), 1, "output_length(23) must be 1");
        assert_eq!(output_length(100, &[]), 78, "output_length(100) must be 78");
        assert_eq!(
            output_length(1000, &[]),
            978,
            "output_length(1000) must be 978"
        );
    }

    #[test]
    fn test_it_not_enough_data() {
        let close: Vec<f64> = (0..22).map(|i| 100.0 + i as f64).collect();
        let result = it_indicator(&[close.as_slice()], &[], None);
        assert!(
            matches!(result, Err(IndicatorError::NotEnoughData)),
            "Expected NotEnoughData for {} bars (need 23)",
            close.len()
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // output_length formula must match the actual returned slice length.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_output_length_matches() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let (out, _) = it_indicator(&[close.as_slice()], &[], None).expect("indicator failed");
            let expected = output_length(n, &[]);
            assert_eq!(
                out[0].len(),
                expected,
                "trendline length mismatch: stock={stock_symbol}"
            );
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // All trendline and trigger values must be finite.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_output_finite() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let (out, _) = it_indicator(&[close.as_slice()], &[], Some(&[true, false, false]))
                .expect("indicator failed");
            let trendline = &out[0];
            let trigger = &out[1];
            for (i, &v) in trendline.iter().enumerate() {
                assert!(
                    v.is_finite(),
                    "trendline NaN/Inf at {i}: stock={stock_symbol}"
                );
            }
            for (i, &v) in trigger.iter().enumerate() {
                assert!(
                    v.is_finite(),
                    "trigger NaN/Inf at {i}: stock={stock_symbol}"
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // alpha must lie in (0, 1): HD guarantees DC ∈ [6, 50] → α ∈ [0.038, 0.25].
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_alpha_bounds() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Skip the first few outputs while the HD IIR converges from zero.
        const CONVERGENCE_SKIP: usize = 50;

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let (out, _) = it_indicator(&[close.as_slice()], &[], Some(&[false, false, true]))
                .expect("indicator failed");
            let alpha = &out[3];
            for (i, &a) in alpha.iter().enumerate() {
                assert!(a.is_finite(), "alpha NaN/Inf at {i}: stock={stock_symbol}");
                if i >= CONVERGENCE_SKIP {
                    assert!(
                        a > 0.0 && a < 1.0,
                        "alpha={a} outside (0,1) at {i}: stock={stock_symbol}"
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // dc_period identity: IT's embedded HD must produce bit-identical output
    // to running homodynediscriminator::indicator() on the same data.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_dc_period_matches_standalone_hd() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let (hd_out, _) = hd_indicator(&[close.as_slice()], &[], None).expect("HD failed");
            let (it_out, _) = it_indicator(&[close.as_slice()], &[], Some(&[false, true, false]))
                .expect("IT failed");
            let hd_dc = &hd_out[0];
            let it_dc = &it_out[2];
            assert_eq!(
                it_dc.len(),
                hd_dc.len(),
                "dc_period length mismatch: stock={stock_symbol}"
            );
            for (i, (&iv, &hv)) in it_dc.iter().zip(hd_dc.iter()).enumerate() {
                assert_eq!(
                    iv, hv,
                    "dc_period mismatch at {i}: it={iv}, hd={hv}, stock={stock_symbol}"
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // trigger formula: trigger[i] == 2·trendline[i] − trendline[i-1] for i > 0.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_trigger_formula() {
        init_database_data();
        let data = get_all_stock_data().unwrap();
        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let (out, _) = it_indicator(&[close.as_slice()], &[], Some(&[true, false, false]))
                .expect("indicator failed");
            let trendline = &out[0];
            let trigger = &out[1];
            // Check trigger[i] = 2*trendline[i] - trendline[i-1] for i > 0.
            for i in 1..trendline.len() {
                let expected = 2.0 * trendline[i] - trendline[i - 1];
                let actual = trigger[i];
                let diff = (actual - expected).abs();
                assert!(
                    diff < 1e-12,
                    "trigger formula mismatch at {i}: got={actual}, expected={expected}, \
                     diff={diff:.2e}, stock={stock_symbol}"
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Optional outputs: populated only when requested; empty otherwise.
    // Values must be consistent across different request modes.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_optional_outputs() {
        let close: Vec<f64> = (0..200).map(|i| 100.0 + (i as f64).sin() * 5.0).collect();
        let inputs = [close.as_slice()];
        let expected_len = output_length(close.len(), &[]);

        // None requested.
        let (none, _) = it_indicator(&inputs, &[], None).unwrap();
        assert_eq!(none[0].len(), expected_len, "trendline len (none)");
        assert!(none[1].is_empty(), "trigger should be empty (none)");
        assert!(none[2].is_empty(), "dc_period should be empty (none)");
        assert!(none[3].is_empty(), "alpha should be empty (none)");

        // Only trigger.
        let (trigger_only, _) = it_indicator(&inputs, &[], Some(&[true, false, false])).unwrap();
        assert_eq!(trigger_only[1].len(), expected_len, "trigger len");
        assert!(trigger_only[2].is_empty(), "dc_period should be empty");
        assert!(trigger_only[3].is_empty(), "alpha should be empty");

        // Only dc_period.
        let (dc_only, _) = it_indicator(&inputs, &[], Some(&[false, true, false])).unwrap();
        assert!(dc_only[1].is_empty(), "trigger should be empty");
        assert_eq!(dc_only[2].len(), expected_len, "dc_period len");
        assert!(dc_only[3].is_empty(), "alpha should be empty");

        // Only alpha.
        let (alpha_only, _) = it_indicator(&inputs, &[], Some(&[false, false, true])).unwrap();
        assert!(alpha_only[1].is_empty(), "trigger should be empty");
        assert!(alpha_only[2].is_empty(), "dc_period should be empty");
        assert_eq!(alpha_only[3].len(), expected_len, "alpha len");

        // All three.
        let (both, _) = it_indicator(&inputs, &[], Some(&[true, true, true])).unwrap();
        assert_eq!(both[1].len(), expected_len, "trigger len (all)");
        assert_eq!(both[2].len(), expected_len, "dc_period len (all)");
        assert_eq!(both[3].len(), expected_len, "alpha len (all)");

        // Values consistent across request modes.
        for (i, (&ta, &tb)) in trigger_only[1].iter().zip(both[1].iter()).enumerate() {
            assert_eq!(ta, tb, "trigger inconsistency at {i}");
        }
        for (i, (&da, &db)) in dc_only[2].iter().zip(both[2].iter()).enumerate() {
            assert_eq!(da, db, "dc_period inconsistency at {i}");
        }
        for (i, (&aa, &ab)) in alpha_only[3].iter().zip(both[3].iter()).enumerate() {
            assert_eq!(aa, ab, "alpha inconsistency at {i}");
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // State continuity: indicator() first chunk + batch_indicator() remainder
    // must be bit-exact to a full single-call run for all four outputs.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            let (ref_out, _) = it_indicator(&[close.as_slice()], &[], Some(&[true, true, true]))
                .expect("reference run failed");

            let (first_out, mut state) =
                it_indicator(&[&close[..FIRST_CHUNK]], &[], Some(&[true, true, true]))
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
                    .batch_indicator(&[chunk], Some(&[true, true, true]))
                    .expect("batch_indicator failed");
                for k in 0..4 {
                    batch[k].extend_from_slice(&out[k]);
                }
            }
            let rem = chunks.remainder();
            if !rem.is_empty() {
                let out = state
                    .batch_indicator(&[rem], Some(&[true, true, true]))
                    .expect("batch_indicator remainder failed");
                for k in 0..4 {
                    batch[k].extend_from_slice(&out[k]);
                }
            }

            for k in 0..4 {
                let label = OUTPUT_LABELS[k];
                assert_eq!(
                    batch[k].len(),
                    ref_out[k].len(),
                    "{label} length mismatch: stock={stock_symbol}"
                );
                for (i, (&bv, &rv)) in batch[k].iter().zip(ref_out[k].iter()).enumerate() {
                    assert!(!bv.is_nan(), "{label} NaN at {i}: stock={stock_symbol}");
                    assert!(
                        !bv.is_infinite(),
                        "{label} Inf at {i}: stock={stock_symbol}"
                    );
                    assert_eq!(
                        bv, rv,
                        "{label} mismatch at {i}: batch={bv}, ref={rv}, stock={stock_symbol}"
                    );
                }
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets: N=4 assets processed in a single SIMD pass must match
    // the scalar run for all four outputs within 1e-4.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_simd_by_assets_vs_scalar() {
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

        let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &[], Some(&[true, true, true]))
            .expect("SIMD by_assets failed");

        for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
            let (scalar_out, _) = it_indicator(&[close.as_slice()], &[], Some(&[true, true, true]))
                .expect("scalar failed");

            for k in 0..4 {
                let label = OUTPUT_LABELS[k];
                let simd_line = &simd_results[asset_idx][k];
                let scalar_line = &scalar_out[k];
                assert_eq!(
                    simd_line.len(),
                    scalar_line.len(),
                    "{label} length mismatch: stock={stock_symbol}"
                );
                for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                    assert!(
                        !sv.is_nan(),
                        "SIMD by_assets {label} NaN at {i}: stock={stock_symbol}"
                    );
                    assert!(
                        !sv.is_infinite(),
                        "SIMD by_assets {label} Inf at {i}: stock={stock_symbol}"
                    );
                    let diff = (sv - rv).abs();
                    assert!(
                        diff < 1e-4,
                        "{label} mismatch at {i}: simd={sv}, scalar={rv}, diff={diff:.2e}, \
                         stock={stock_symbol}"
                    );
                }
            }
            println!(
                "✓ SIMD by_assets vs scalar passed for stock {stock_symbol} ({} bars)",
                close.len()
            );
        }
        println!("✓ All SIMD by_assets vs scalar IT tests passed!");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // SIMD by_assets state continuity: SIMD seed (first chunk) + scalar
    // batch_indicator (remainder) must match full scalar run within 1e-4.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_it_simd_by_assets_state_continuity() {
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

        let (simd_first, mut states) =
            indicator_by_assets::<4>(&inputs_first, &[], Some(&[true, true, true]))
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
                    .batch_indicator(&[chunk], Some(&[true, true, true]))
                    .expect("batch_indicator failed");
                for k in 0..4 {
                    batch[k].extend_from_slice(&out[k]);
                }
            }
            let rem = chunks.remainder();
            if !rem.is_empty() {
                let out = states[asset_idx]
                    .batch_indicator(&[rem], Some(&[true, true, true]))
                    .expect("batch_indicator failed on remainder");
                for k in 0..4 {
                    batch[k].extend_from_slice(&out[k]);
                }
            }

            let (scalar_out, _) = it_indicator(&[close.as_slice()], &[], Some(&[true, true, true]))
                .expect("scalar indicator failed");

            for k in 0..4 {
                let label = OUTPUT_LABELS[k];
                assert_eq!(
                    batch[k].len(),
                    scalar_out[k].len(),
                    "{label} length mismatch: stock={stock_symbol}"
                );
                for (i, (&bv, &rv)) in batch[k].iter().zip(scalar_out[k].iter()).enumerate() {
                    assert!(!bv.is_nan(), "{label} NaN at {i}: stock={stock_symbol}");
                    assert!(
                        !bv.is_infinite(),
                        "{label} Inf at {i}: stock={stock_symbol}"
                    );
                    let diff = (bv - rv).abs();
                    assert!(
                        diff < 1e-4,
                        "{label} mismatch at {i}: simd+batch={bv}, scalar={rv}, \
                         diff={diff:.2e}, stock={stock_symbol}"
                    );
                }
            }
            println!(
                "✓ SIMD by_assets state continuity passed for stock {stock_symbol} ({} bars)",
                close.len()
            );
        }
        println!("✓ All SIMD by_assets state continuity IT tests passed!");
    }

    }
