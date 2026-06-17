#[cfg(test)]
mod tests {
    use tulip_rs::indicator_types::TIndicatorState;
    use tulip_rs::indicators::highpass::indicator as highpass_indicator;
    use tulip_rs::indicators::hilberttransform::indicator as hilberttransform;
    use tulip_rs::indicators::hilberttransform::indicator_by_assets;
    use tulip_rs::indicators::hilberttransform::indicator_by_options;
    use tulip_rs::indicators::roofingfilter::indicator as roofingfilter;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    /// [ss_period, hp_period]
    const OPTIONS_LIST: [[f64; 2]; 4] = [[10.0, 48.0], [14.0, 40.0], [20.0, 5.0], [20.0, 60.0]];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // -------------------------------------------------------------------------
    // State continuity: indicator() first chunk + batch_indicator() remainder
    // must be bit-exact to a full single-call run for all four outputs
    // (in_phase, quadrature, optional roofing, optional highpass).
    // NaN and infinity are also checked inline on every value compared.
    // -------------------------------------------------------------------------

    #[test]
    fn test_hilberttransform_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // Full reference run with both optional outputs enabled.
                let (ref_out, _) =
                    hilberttransform(&[close.as_slice()], &options, Some(&[true, true]))
                        .expect("HilbertTransform reference run failed");
                let ref_p = &ref_out[0];
                let ref_q = &ref_out[1];
                let ref_rf = &ref_out[2];
                let ref_hp = &ref_out[3];

                // Seeded run on the first chunk.
                let (first_out, mut state) =
                    hilberttransform(&[&close[..FIRST_CHUNK]], &options, Some(&[true, true]))
                        .expect("HilbertTransform seed failed");

                let mut batch_p = first_out[0].clone();
                let mut batch_q = first_out[1].clone();
                let mut batch_rf = first_out[2].clone();
                let mut batch_hp = first_out[3].clone();

                // Stream the remainder in CHUNK_SIZE-bar batches.
                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = state
                        .batch_indicator(&[chunk], Some(&[true, true]))
                        .expect("batch_indicator failed");
                    batch_p.extend_from_slice(&out[0]);
                    batch_q.extend_from_slice(&out[1]);
                    batch_rf.extend_from_slice(&out[2]);
                    batch_hp.extend_from_slice(&out[3]);
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = state
                        .batch_indicator(&[rem], Some(&[true, true]))
                        .expect("batch_indicator failed on remainder");
                    batch_p.extend_from_slice(&out[0]);
                    batch_q.extend_from_slice(&out[1]);
                    batch_rf.extend_from_slice(&out[2]);
                    batch_hp.extend_from_slice(&out[3]);
                }

                // in_phase
                assert_eq!(
                    batch_p.len(),
                    ref_p.len(),
                    "in_phase length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_p.iter().zip(ref_p.iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "in_phase has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "in_phase has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, rv,
                        "in_phase mismatch at {i}: batch={bv}, ref={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                // quadrature
                assert_eq!(
                    batch_q.len(),
                    ref_q.len(),
                    "quadrature length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_q.iter().zip(ref_q.iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "quadrature has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "quadrature has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, rv,
                        "quadrature mismatch at {i}: batch={bv}, ref={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                // optional roofing
                assert_eq!(
                    batch_rf.len(),
                    ref_rf.len(),
                    "roofing length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_rf.iter().zip(ref_rf.iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "roofing has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "roofing has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, rv,
                        "roofing mismatch at {i}: batch={bv}, ref={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                // optional highpass
                assert_eq!(
                    batch_hp.len(),
                    ref_hp.len(),
                    "highpass length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_hp.iter().zip(ref_hp.iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "highpass has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "highpass has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, rv,
                        "highpass mismatch at {i}: batch={bv}, ref={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Optional roofing output (outputs[2]) must be bit-exact to the standalone
    // roofingfilter indicator run with the same [ss_period, hp_period] options.
    // NaN and infinity are also checked inline on every value compared.
    // -------------------------------------------------------------------------

    #[test]
    fn test_hilberttransform_optional_rf_matches_roofingfilter() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                // Enable only the roofing optional output (index 0 = true, index 1 = false).
                let (ht_out, _) = hilberttransform(&inputs, &options, Some(&[true, false]))
                    .expect("HilbertTransform failed");
                let ht_rf = &ht_out[2];

                let (rf_out, _) =
                    roofingfilter(&inputs, &options, None).expect("RoofingFilter failed");
                let rf_standalone = &rf_out[0];

                assert_eq!(
                    ht_rf.len(),
                    rf_standalone.len(),
                    "Roofing length mismatch: stock={stock_symbol}, options={options:?}, \
                     ht_rf={}, rf_standalone={}",
                    ht_rf.len(),
                    rf_standalone.len()
                );
                for (i, (&hv, &rv)) in ht_rf.iter().zip(rf_standalone.iter()).enumerate() {
                    if hv.is_nan() {
                        panic!(
                            "HilbertTransform roofing has NaN at index {i}: value={hv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if hv.is_infinite() {
                        panic!(
                            "HilbertTransform roofing has infinity at index {i}: value={hv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        hv, rv,
                        "Roofing mismatch at index {i}: ht_rf={hv}, standalone_rf={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Optional highpass output (outputs[3]) must be bit-exact to the standalone
    // highpass indicator run with the same hp_period (options[1]).
    // NaN and infinity are also checked inline on every value compared.
    // -------------------------------------------------------------------------

    #[test]
    fn test_hilberttransform_optional_hp_matches_highpass() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let hp_period = options[1];

                // Enable only the highpass optional output (index 0 = false, index 1 = true).
                let (ht_out, _) = hilberttransform(&inputs, &options, Some(&[false, true]))
                    .expect("HilbertTransform failed");
                let ht_hp = &ht_out[3];

                let (hp_out, _) =
                    highpass_indicator(&inputs, &[hp_period], None).expect("HighPass failed");
                let hp_standalone = &hp_out[0];

                assert_eq!(
                    ht_hp.len(),
                    hp_standalone.len(),
                    "Highpass length mismatch: stock={stock_symbol}, options={options:?}, \
                     ht_hp={}, hp_standalone={}",
                    ht_hp.len(),
                    hp_standalone.len()
                );
                for (i, (&hv, &sv)) in ht_hp.iter().zip(hp_standalone.iter()).enumerate() {
                    if hv.is_nan() {
                        panic!(
                            "HilbertTransform highpass has NaN at index {i}: value={hv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if hv.is_infinite() {
                        panic!(
                            "HilbertTransform highpass has infinity at index {i}: value={hv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        hv, sv,
                        "Highpass mismatch at index {i}: ht_hp={hv}, standalone_hp={sv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: all four outputs match the scalar run per asset.
    // -------------------------------------------------------------------------

    #[test]
    fn test_hilberttransform_simd_by_assets_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| (symbol.clone(), get_close_array(eod)))
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
                    .expect("SIMD by-assets HilbertTransform failed");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let (scalar_out, _) =
                    hilberttransform(&[close.as_slice()], &options, Some(&[true, true]))
                        .expect("Scalar HilbertTransform failed");

                let labels = ["in_phase", "quadrature", "roofing", "highpass"];
                for out_idx in 0..4 {
                    let simd_line = &simd_results[asset_idx][out_idx];
                    let scalar_line = &scalar_out[out_idx];
                    let label = labels[out_idx];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        if sv.is_nan() {
                            panic!(
                                "SIMD by-assets {label} has NaN at index {i}: value={sv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if sv.is_infinite() {
                            panic!(
                                "SIMD by-assets {label} has infinity at index {i}: value={sv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        assert_eq!(
                            sv, rv,
                            "{label} mismatch at {i}: simd={sv}, scalar={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: all four outputs match the scalar run per option set.
    // -------------------------------------------------------------------------

    #[test]
    fn test_hilberttransform_simd_by_options_vs_regular_database() {
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
                    .expect("SIMD by-options HilbertTransform failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_out, _) = hilberttransform(&inputs, options, Some(&[true, true]))
                    .expect("Scalar HilbertTransform failed");

                let labels = ["in_phase", "quadrature", "roofing", "highpass"];
                for out_idx in 0..4 {
                    let simd_line = &simd_results[opt_idx][out_idx];
                    let scalar_line = &scalar_out[out_idx];
                    let label = labels[out_idx];
                    assert_eq!(
                        simd_line.len(),
                        scalar_line.len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_line.iter().zip(scalar_line.iter()).enumerate() {
                        if sv.is_nan() {
                            panic!(
                                "SIMD by-options {label} has NaN at index {i}: value={sv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if sv.is_infinite() {
                            panic!(
                                "SIMD by-options {label} has infinity at index {i}: value={sv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        assert_eq!(
                            sv, rv,
                            "{label} mismatch at {i}: simd={sv}, scalar={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: state continuity — SIMD first chunk + batch_indicator
    // remainder must equal full scalar run for all four outputs.
    // -------------------------------------------------------------------------

    #[test]
    fn test_hilberttransform_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let stock_data: Vec<(String, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| (symbol.clone(), get_close_array(eod)))
            .collect();

        for options in OPTIONS_LIST {
            let inputs_4: [&[&[f64]; 1]; 4] = [
                &[&stock_data[0].1[..FIRST_CHUNK]],
                &[&stock_data[1].1[..FIRST_CHUNK]],
                &[&stock_data[2].1[..FIRST_CHUNK]],
                &[&stock_data[3].1[..FIRST_CHUNK]],
            ];

            let (simd_first, mut states) =
                indicator_by_assets::<4>(&inputs_4, &options, Some(&[true, true]))
                    .expect("SIMD by-assets HilbertTransform failed on first chunk");

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
                    hilberttransform(&[close.as_slice()], &options, Some(&[true, true]))
                        .expect("Scalar HilbertTransform failed");

                let labels = ["in_phase", "quadrature", "roofing", "highpass"];
                for k in 0..4 {
                    let label = labels[k];
                    assert_eq!(
                        batch[k].len(),
                        scalar_out[k].len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&bv, &sv)) in batch[k].iter().zip(scalar_out[k].iter()).enumerate() {
                        if bv.is_nan() {
                            panic!(
                                "SIMD by-assets {label} has NaN at index {i}: batch={bv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if bv.is_infinite() {
                            panic!(
                                "SIMD by-assets {label} has infinity at index {i}: batch={bv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        assert_eq!(
                            bv, sv,
                            "{label} mismatch at {i}: simd+batch={bv}, scalar={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: state continuity — SIMD first chunk + batch_indicator
    // remainder must equal full scalar run for all four outputs.
    // -------------------------------------------------------------------------

    #[test]
    fn test_hilberttransform_simd_by_options_state_continuity() {
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
            let close = get_close_array(stock_data);
            let first_inputs = [&close[..FIRST_CHUNK] as &[f64]];

            let (simd_first, mut states) =
                indicator_by_options::<4>(&first_inputs, &options_4, Some(&[true, true]))
                    .expect("SIMD by-options HilbertTransform failed on first chunk");

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
                    hilberttransform(&[close.as_slice()], options, Some(&[true, true]))
                        .expect("Scalar HilbertTransform failed");

                let labels = ["in_phase", "quadrature", "roofing", "highpass"];
                for k in 0..4 {
                    let label = labels[k];
                    assert_eq!(
                        batch[k].len(),
                        scalar_out[k].len(),
                        "{label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&bv, &sv)) in batch[k].iter().zip(scalar_out[k].iter()).enumerate() {
                        if bv.is_nan() {
                            panic!(
                                "SIMD by-options {label} has NaN at index {i}: batch={bv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if bv.is_infinite() {
                            panic!(
                                "SIMD by-options {label} has infinity at index {i}: batch={bv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        assert_eq!(
                            bv, sv,
                            "{label} mismatch at {i}: simd+batch={bv}, scalar={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }
}
