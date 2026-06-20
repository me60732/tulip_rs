#[cfg(test)]
mod tests {
    use tulip_rs::indicator_types::TIndicatorState;
    use tulip_rs::indicators::highpass::indicator as highpass_indicator;
    use tulip_rs::indicators::roofingfilter::indicator as roofingfilter;
    use tulip_rs::indicators::roofingfilter::indicator_by_assets;
    use tulip_rs::indicators::roofingfilter::indicator_by_options;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    /// [ss_period, hp_period]
    const OPTIONS_LIST: [[f64; 2]; 4] = [[10.0, 48.0], [14.0, 40.0], [20.0, 5.0], [20.0, 60.0]];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // -------------------------------------------------------------------------
    // State continuity: indicator() first chunk + batch_indicator() remainder
    // must equal a full single-call run, for both the roofing and optional
    // highpass outputs.
    // NaN and infinity are also checked inline on every value compared.
    // -------------------------------------------------------------------------

    #[test]
    fn test_roofingfilter_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // Full reference run (with optional highpass).
                let (ref_out, _) = roofingfilter(&[close.as_slice()], &options, Some(&[true]))
                    .expect("RoofingFilter reference run failed");
                let ref_rf = &ref_out[0];
                let ref_hp = &ref_out[1];

                // Seeded run.
                let (first_out, mut state) =
                    roofingfilter(&[&close[..FIRST_CHUNK]], &options, Some(&[true]))
                        .expect("RoofingFilter seed failed");

                let mut batch_rf = first_out[0].clone();
                let mut batch_hp = first_out[1].clone();

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = state
                        .batch_indicator(&[chunk], Some(&[true]))
                        .expect("batch_indicator failed");
                    batch_rf.extend_from_slice(&out[0]);
                    batch_hp.extend_from_slice(&out[1]);
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = state
                        .batch_indicator(&[rem], Some(&[true]))
                        .expect("batch_indicator failed on remainder");
                    batch_rf.extend_from_slice(&out[0]);
                    batch_hp.extend_from_slice(&out[1]);
                }

                assert_eq!(
                    batch_rf.len(),
                    ref_rf.len(),
                    "RF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_rf.iter().zip(ref_rf.iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "RF has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "RF has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, rv,
                        "RF mismatch at {i}: batch={bv}, ref={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                assert_eq!(
                    batch_hp.len(),
                    ref_hp.len(),
                    "HP length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_hp.iter().zip(ref_hp.iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "HP has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "HP has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, rv,
                        "HP mismatch at {i}: batch={bv}, ref={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Optional highpass output must be bit-exact to the standalone highpass
    // indicator run with the same hp_period.
    // -------------------------------------------------------------------------

    #[test]
    fn test_roofingfilter_optional_hp_matches_highpass_indicator() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let hp_period = options[1];

                let (rf_out, _) =
                    roofingfilter(&inputs, &options, Some(&[true])).expect("RoofingFilter failed");
                let rf_hp = &rf_out[1];

                let (hp_out, _) =
                    highpass_indicator(&inputs, &[hp_period], None).expect("HighPass failed");
                let hp_standalone = &hp_out[0];

                assert_eq!(
                    rf_hp.len(),
                    hp_standalone.len(),
                    "Length mismatch: stock={stock_symbol}, options={options:?}, \
                     rf_hp={}, hp_standalone={}",
                    rf_hp.len(),
                    hp_standalone.len()
                );
                for (i, (&rv, &hv)) in rf_hp.iter().zip(hp_standalone.iter()).enumerate() {
                    if rv.is_nan() {
                        panic!(
                            "RoofingFilter HP has NaN at index {i}: value={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if rv.is_infinite() {
                        panic!(
                            "RoofingFilter HP has infinity at index {i}: value={rv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        rv, hv,
                        "Mismatch at index {i}: roofing_hp={rv}, standalone_hp={hv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: rf and optional hp outputs match scalar per asset.
    // -------------------------------------------------------------------------

    #[test]
    fn test_roofingfilter_simd_by_assets_vs_regular_database() {
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
            let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &options, Some(&[true]))
                .expect("SIMD by-assets RoofingFilter failed");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let (scalar_out, _) = roofingfilter(&[close.as_slice()], &options, Some(&[true]))
                    .expect("Scalar RoofingFilter failed");

                // RF line
                let simd_rf = &simd_results[asset_idx][0];
                let scalar_rf = &scalar_out[0];
                assert_eq!(
                    simd_rf.len(),
                    scalar_rf.len(),
                    "RF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_rf.iter().zip(scalar_rf.iter()).enumerate() {
                    if sv.is_nan() {
                        panic!(
                            "SIMD by-assets RF has NaN at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if sv.is_infinite() {
                        panic!(
                            "SIMD by-assets RF has infinity at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        sv, rv,
                        "RF mismatch at {i}: simd={sv}, scalar={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                // Optional HP line
                let simd_hp = &simd_results[asset_idx][1];
                let scalar_hp = &scalar_out[1];
                assert_eq!(
                    simd_hp.len(),
                    scalar_hp.len(),
                    "HP length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_hp.iter().zip(scalar_hp.iter()).enumerate() {
                    if sv.is_nan() {
                        panic!(
                            "SIMD by-assets HP has NaN at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if sv.is_infinite() {
                        panic!(
                            "SIMD by-assets HP has infinity at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        sv, rv,
                        "HP mismatch at {i}: simd={sv}, scalar={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: rf and optional hp outputs match scalar per option set.
    // -------------------------------------------------------------------------

    #[test]
    fn test_roofingfilter_simd_by_options_vs_regular_database() {
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

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, Some(&[true]))
                .expect("SIMD by-options RoofingFilter failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_out, _) = roofingfilter(&inputs, options, Some(&[true]))
                    .expect("Scalar RoofingFilter failed");

                // RF line
                let simd_rf = &simd_results[opt_idx][0];
                let scalar_rf = &scalar_out[0];
                assert_eq!(
                    simd_rf.len(),
                    scalar_rf.len(),
                    "RF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_rf.iter().zip(scalar_rf.iter()).enumerate() {
                    if sv.is_nan() {
                        panic!(
                            "SIMD by-options RF has NaN at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if sv.is_infinite() {
                        panic!(
                            "SIMD by-options RF has infinity at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        sv, rv,
                        "RF mismatch at {i}: simd={sv}, scalar={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                // Optional HP line
                let simd_hp = &simd_results[opt_idx][1];
                let scalar_hp = &scalar_out[1];
                assert_eq!(
                    simd_hp.len(),
                    scalar_hp.len(),
                    "HP length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_hp.iter().zip(scalar_hp.iter()).enumerate() {
                    if sv.is_nan() {
                        panic!(
                            "SIMD by-options HP has NaN at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if sv.is_infinite() {
                        panic!(
                            "SIMD by-options HP has infinity at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        sv, rv,
                        "HP mismatch at {i}: simd={sv}, scalar={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: state continuity — SIMD first chunk + batch_indicator
    // remainder must equal full scalar run (rf and hp).
    // -------------------------------------------------------------------------

    #[test]
    fn test_roofingfilter_simd_by_assets_state_continuity() {
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
                indicator_by_assets::<4>(&inputs_4, &options, Some(&[true]))
                    .expect("SIMD by-assets RoofingFilter failed on first chunk");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let mut batch_rf = simd_first[asset_idx][0].clone();
                let mut batch_hp = simd_first[asset_idx][1].clone();

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = states[asset_idx]
                        .batch_indicator(&[chunk], Some(&[true]))
                        .expect("batch_indicator failed");
                    batch_rf.extend_from_slice(&out[0]);
                    batch_hp.extend_from_slice(&out[1]);
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = states[asset_idx]
                        .batch_indicator(&[rem], Some(&[true]))
                        .expect("batch_indicator failed on remainder");
                    batch_rf.extend_from_slice(&out[0]);
                    batch_hp.extend_from_slice(&out[1]);
                }

                let (scalar_out, _) = roofingfilter(&[close.as_slice()], &options, Some(&[true]))
                    .expect("Scalar RoofingFilter failed");

                assert_eq!(
                    batch_rf.len(),
                    scalar_out[0].len(),
                    "RF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &sv)) in batch_rf.iter().zip(scalar_out[0].iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "SIMD by-assets RF has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "SIMD by-assets RF has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, sv,
                        "RF mismatch at {i}: simd+batch={bv}, scalar={sv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                assert_eq!(
                    batch_hp.len(),
                    scalar_out[1].len(),
                    "HP length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &sv)) in batch_hp.iter().zip(scalar_out[1].iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "SIMD by-assets HP has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "SIMD by-assets HP has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, sv,
                        "HP mismatch at {i}: simd+batch={bv}, scalar={sv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: state continuity — SIMD first chunk + batch_indicator
    // remainder must equal full scalar run (rf and hp).
    // -------------------------------------------------------------------------

    #[test]
    fn test_roofingfilter_simd_by_options_state_continuity() {
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
                indicator_by_options::<4>(&first_inputs, &options_4, Some(&[true]))
                    .expect("SIMD by-options RoofingFilter failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch_rf = simd_first[opt_idx][0].clone();
                let mut batch_hp = simd_first[opt_idx][1].clone();

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = states[opt_idx]
                        .batch_indicator(&[chunk], Some(&[true]))
                        .expect("batch_indicator failed");
                    batch_rf.extend_from_slice(&out[0]);
                    batch_hp.extend_from_slice(&out[1]);
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = states[opt_idx]
                        .batch_indicator(&[rem], Some(&[true]))
                        .expect("batch_indicator failed on remainder");
                    batch_rf.extend_from_slice(&out[0]);
                    batch_hp.extend_from_slice(&out[1]);
                }

                let (scalar_out, _) = roofingfilter(&[close.as_slice()], options, Some(&[true]))
                    .expect("Scalar RoofingFilter failed");

                assert_eq!(
                    batch_rf.len(),
                    scalar_out[0].len(),
                    "RF length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &sv)) in batch_rf.iter().zip(scalar_out[0].iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "SIMD by-options RF has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "SIMD by-options RF has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, sv,
                        "RF mismatch at {i}: simd+batch={bv}, scalar={sv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }

                assert_eq!(
                    batch_hp.len(),
                    scalar_out[1].len(),
                    "HP length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &sv)) in batch_hp.iter().zip(scalar_out[1].iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "SIMD by-options HP has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "SIMD by-options HP has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, sv,
                        "HP mismatch at {i}: simd+batch={bv}, scalar={sv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    }
