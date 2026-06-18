#[cfg(test)]
mod tests {
    use tulip_rs::indicator_types::TIndicatorState;
    use tulip_rs::indicators::homodynediscriminator::indicator as homodynediscriminator;
    use tulip_rs::indicators::homodynediscriminator::{
        indicator_by_assets, min_data, output_length,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // -------------------------------------------------------------------------
    // output_length and min_data sanity checks — no database needed.
    // -------------------------------------------------------------------------

    #[test]
    fn test_homodynediscriminator_min_data_and_output_length() {
        assert_eq!(min_data(&[]), 23);
        assert_eq!(output_length(23, &[]), 1);
        assert_eq!(output_length(100, &[]), 78);
        assert_eq!(output_length(1000, &[]), 978);
    }

    #[test]
    fn test_homodynediscriminator_not_enough_data() {
        let close: Vec<f64> = (0..22).map(|i| 100.0 + i as f64).collect();
        let inputs = [close.as_slice()];
        let result = homodynediscriminator(&inputs, &[], None);
        assert!(
            result.is_err(),
            "Expected NotEnoughData for {} bars (need 23)",
            close.len()
        );
    }

    // -------------------------------------------------------------------------
    // State continuity: indicator() first chunk + batch_indicator() remainder
    // must be bit-exact to a full single-call run.
    // -------------------------------------------------------------------------

    #[test]
    fn test_homodynediscriminator_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            let (ref_out, _) = homodynediscriminator(&[close.as_slice()], &[], None)
                .expect("reference run failed");
            let ref_dc = &ref_out[0];

            let (first_out, mut state) = homodynediscriminator(&[&close[..FIRST_CHUNK]], &[], None)
                .expect("seed run failed");

            let mut batch_dc = first_out[0].clone();

            let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
            for chunk in chunks.by_ref() {
                let out = state
                    .batch_indicator(&[chunk], None)
                    .expect("batch_indicator failed");
                batch_dc.extend_from_slice(&out[0]);
            }
            let rem = chunks.remainder();
            if !rem.is_empty() {
                let out = state
                    .batch_indicator(&[rem], None)
                    .expect("batch_indicator remainder failed");
                batch_dc.extend_from_slice(&out[0]);
            }

            assert_eq!(
                batch_dc.len(),
                ref_dc.len(),
                "dc_period length mismatch: stock={stock_symbol}"
            );
            for (i, (&bv, &rv)) in batch_dc.iter().zip(ref_dc.iter()).enumerate() {
                assert!(!bv.is_nan(), "NaN at {i}: stock={stock_symbol}");
                assert!(!bv.is_infinite(), "Inf at {i}: stock={stock_symbol}");
                assert_eq!(
                    bv, rv,
                    "mismatch at {i}: batch={bv}, ref={rv}, stock={stock_symbol}"
                );
            }
        }
    }

    // -------------------------------------------------------------------------
    // All dc_period values must be finite and within [6, 50] after convergence.
    // -------------------------------------------------------------------------

    #[test]
    fn test_homodynediscriminator_output_bounds() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const CONVERGENCE_SKIP: usize = 100;

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let (out, _) =
                homodynediscriminator(&[close.as_slice()], &[], None).expect("indicator failed");
            let dc = &out[0];

            for (i, &v) in dc.iter().enumerate() {
                assert!(!v.is_nan(), "NaN at {i}: stock={stock_symbol}");
                assert!(!v.is_infinite(), "Inf at {i}: stock={stock_symbol}");
                assert!(v >= 0.0, "negative at {i}: {v}, stock={stock_symbol}");
                if i >= CONVERGENCE_SKIP {
                    assert!(
                        v >= 6.0 && v <= 50.0,
                        "out of [6,50] at {i}: {v}, stock={stock_symbol}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // output_length formula must match actual slice length returned.
    // -------------------------------------------------------------------------

    #[test]
    fn test_homodynediscriminator_output_length_matches() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let n = close.len();
            let (out, _) =
                homodynediscriminator(&[close.as_slice()], &[], None).expect("indicator failed");
            assert_eq!(
                out[0].len(),
                output_length(n, &[]),
                "length mismatch: stock={stock_symbol}"
            );
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by_assets: N=4 outputs must match the scalar indicator to < 1e-6.
    // simd_atan is a Cephes-style polynomial accurate to ~1e-15 vs libm, but
    // IIR accumulation is bounded by the 0.2/0.8 smoothing, so 1e-6 is safe.
    // -------------------------------------------------------------------------

    #[test]
    fn test_homodynediscriminator_simd_by_assets() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        // Take the first 4 stocks for a 4-lane SIMD run.
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, stock)| (symbol.clone(), get_close_array(stock)))
            .collect();

        let inputs: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1],
            &[&stock_data[1].1],
            &[&stock_data[2].1],
            &[&stock_data[3].1],
        ];

        let (simd_results, _) =
            indicator_by_assets::<4>(&inputs, &[], None).expect("SIMD by_assets failed");

        for (stock_idx, (stock_symbol, stock_close)) in stock_data.iter().enumerate() {
            let (scalar_out, _) = homodynediscriminator(&[stock_close.as_slice()], &[], None)
                .expect("scalar indicator failed");

            let simd_dc = &simd_results[stock_idx][0];
            let scalar_dc = &scalar_out[0];

            assert_eq!(
                simd_dc.len(),
                scalar_dc.len(),
                "output length mismatch for stock {stock_symbol}"
            );

            for (i, (&sv, &rv)) in simd_dc.iter().zip(scalar_dc.iter()).enumerate() {
                assert!(!sv.is_nan(), "NaN at {i}: stock={stock_symbol}");
                assert!(!sv.is_infinite(), "Inf at {i}: stock={stock_symbol}");
                let diff = (sv - rv).abs();
                assert!(
                    diff < 1e-4,
                    "dc_period mismatch at index {i}: simd={sv}, scalar={rv}, \
                     diff={diff:.2e}, stock={stock_symbol}"
                );
            }

            println!(
                "✓ SIMD by_assets vs scalar passed for stock {stock_symbol} \
                 ({} bars, {} outputs)",
                stock_close.len(),
                simd_dc.len()
            );
        }

        println!("✓ All SIMD by_assets vs scalar Homodyne Discriminator tests passed!");
    }

    // -------------------------------------------------------------------------
    // SIMD by_assets state continuity:
    // indicator_by_assets() on the first 1000 bars returns (outputs, states);
    // batch_indicator() on the remainder (via each IndicatorState) must produce
    // a combined result that matches the full scalar indicator run to < 1e-4.
    //
    // This exercises the full scatter path (SimdState::write_states) that
    // unpacks SIMD lanes back into scalar State structs so that streaming
    // batch_indicator calls can continue from the SIMD-computed warmup.
    // -------------------------------------------------------------------------

    #[test]
    fn test_homodynediscriminator_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        // Take the first 4 stocks for a 4-lane SIMD run.
        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| (symbol.clone(), get_close_array(eod)))
            .collect();

        // Seed: run indicator_by_assets on the first 1000 bars, capturing states.
        let inputs_first: [&[&[f64]; 1]; 4] = [
            &[&stock_data[0].1[..FIRST_CHUNK]],
            &[&stock_data[1].1[..FIRST_CHUNK]],
            &[&stock_data[2].1[..FIRST_CHUNK]],
            &[&stock_data[3].1[..FIRST_CHUNK]],
        ];

        let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_first, &[], None)
            .expect("SIMD by_assets failed on first chunk");

        // For each asset: extend the first-chunk output with batch_indicator remainder,
        // then compare the combined result against the full scalar run.
        for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
            let mut batch_dc = simd_first[asset_idx][0].clone();

            let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
            for chunk in chunks.by_ref() {
                let out = states[asset_idx]
                    .batch_indicator(&[chunk], None)
                    .expect("batch_indicator failed");
                batch_dc.extend_from_slice(&out[0]);
            }
            let rem = chunks.remainder();
            if !rem.is_empty() {
                let out = states[asset_idx]
                    .batch_indicator(&[rem], None)
                    .expect("batch_indicator failed on remainder");
                batch_dc.extend_from_slice(&out[0]);
            }

            // Reference: full scalar run.
            let (scalar_out, _) = homodynediscriminator(&[close.as_slice()], &[], None)
                .expect("scalar indicator failed");
            let scalar_dc = &scalar_out[0];

            assert_eq!(
                batch_dc.len(),
                scalar_dc.len(),
                "dc_period length mismatch: stock={stock_symbol}"
            );

            for (i, (&bv, &rv)) in batch_dc.iter().zip(scalar_dc.iter()).enumerate() {
                assert!(!bv.is_nan(), "NaN at {i}: stock={stock_symbol}");
                assert!(!bv.is_infinite(), "Inf at {i}: stock={stock_symbol}");
                let diff = (bv - rv).abs();
                assert!(
                    diff < 1e-4,
                    "dc_period mismatch at index {i}: simd+batch={bv}, scalar={rv}, \
                     diff={diff:.2e}, stock={stock_symbol}"
                );
            }

            println!(
                "✓ SIMD by_assets state continuity passed for stock {stock_symbol} \
                 ({} bars, {} outputs)",
                close.len(),
                batch_dc.len()
            );
        }

        println!("✓ All SIMD by_assets state continuity tests passed!");
    }
}
