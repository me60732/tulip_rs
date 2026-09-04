#[cfg(test)]
mod tests {
    use tulip_rs::indicators::supersmoother::{
        Indicator, IndicatorByOptions, SuperSmoother, TIndicatorState,
    };
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;

    const OPTIONS_LIST: [[f64; 1]; 4] = [[5.0], [10.0], [14.0], [20.0]];

    fn get_close_array(stock_data: &[tulip_test::database::EodData]) -> Vec<f64> {
        stock_data.iter().map(|d| d.close).collect()
    }

    // -------------------------------------------------------------------------
    // Scalar NaN/infinity guard: the scalar indicator must never produce NaN
    // or infinity for any stock in the database.
    // -------------------------------------------------------------------------

    #[test]
    fn test_supersmoother_scalar_no_nan_or_inf() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);
            let inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let (outputs, _) = SuperSmoother::indicator(&inputs, &options, None)
                    .expect("SuperSmoother failed");

                for (i, &val) in outputs[0].iter().enumerate() {
                    if val.is_nan() {
                        panic!(
                            "SuperSmoother has NaN at index {i}: value={val}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if val.is_infinite() {
                        panic!(
                            "SuperSmoother has infinity at index {i}: value={val}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Scalar state continuity: indicator() first chunk + batch_indicator()
    // remainder must equal a full single-call run.
    // NaN and infinity are also checked inline on every value compared.
    // -------------------------------------------------------------------------

    #[test]
    fn test_supersmoother_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        for (stock_symbol, stock_data) in data {
            let close = get_close_array(stock_data);

            for options in OPTIONS_LIST {
                // Full reference run.
                let (ref_out, _) = SuperSmoother::indicator(&[close.as_slice()], &options, None)
                    .expect("SuperSmoother reference run failed");

                // Seeded run.
                let (first_out, mut state) =
                    SuperSmoother::indicator(&[&close[..FIRST_CHUNK]], &options, None)
                        .expect("SuperSmoother seed failed");

                let mut batch_output = first_out[0].clone();

                let mut chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in chunks.by_ref() {
                    let out = state
                        .batch_indicator(&[chunk], None)
                        .expect("batch_indicator failed");
                    batch_output.extend_from_slice(&out[0]);
                }
                let rem = chunks.remainder();
                if !rem.is_empty() {
                    let out = state
                        .batch_indicator(&[rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_output.extend_from_slice(&out[0]);
                }

                assert_eq!(
                    batch_output.len(),
                    ref_out[0].len(),
                    "Length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_output.iter().zip(ref_out[0].iter()).enumerate() {
                    if bv.is_nan() {
                        panic!(
                            "SuperSmoother has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "SuperSmoother has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, rv,
                        "Mismatch at index {i}: batch={bv}, ref={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: outputs match scalar per asset (database).
    // NaN and infinity are also checked inline on every value compared.
    // -------------------------------------------------------------------------

    #[test]
    fn test_supersmoother_simd_by_assets_vs_regular_database() {
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
                SuperSmoother::indicator_by_assets::<4>(&inputs_4, &options, None)
                    .expect("SIMD by-assets SuperSmoother failed");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let (scalar_outputs, _) =
                    SuperSmoother::indicator(&[close.as_slice()], &options, None)
                        .expect("Scalar SuperSmoother failed");

                let simd_out = &simd_results[asset_idx][0];
                let scalar_out = &scalar_outputs[0];

                assert_eq!(
                    simd_out.len(),
                    scalar_out.len(),
                    "Length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out.iter()).enumerate() {
                    if sv.is_nan() {
                        panic!(
                            "SIMD by-assets SuperSmoother has NaN at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if sv.is_infinite() {
                        panic!(
                            "SIMD by-assets SuperSmoother has infinity at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        sv, rv,
                        "Mismatch at index {i}: simd={sv}, scalar={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: outputs match scalar per option set (database).
    // NaN and infinity are also checked inline on every value compared.
    // -------------------------------------------------------------------------

    #[test]
    fn test_supersmoother_simd_by_options_vs_regular_database() {
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
                SuperSmoother::indicator_by_options::<4>(&inputs, &options_4, None)
                    .expect("SIMD by-options SuperSmoother failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_outputs, _) = SuperSmoother::indicator(&inputs, options, None)
                    .expect("Scalar SuperSmoother failed");

                let simd_out = &simd_results[opt_idx][0];
                let scalar_out = &scalar_outputs[0];

                assert_eq!(
                    simd_out.len(),
                    scalar_out.len(),
                    "Length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out.iter()).enumerate() {
                    if sv.is_nan() {
                        panic!(
                            "SIMD by-options SuperSmoother has NaN at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if sv.is_infinite() {
                        panic!(
                            "SIMD by-options SuperSmoother has infinity at index {i}: value={sv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        sv, rv,
                        "Mismatch at index {i}: simd={sv}, scalar={rv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: state continuity — SIMD first chunk + batch_indicator
    // remainder must equal full scalar run.
    // -------------------------------------------------------------------------

    #[test]
    fn test_supersmoother_simd_by_assets_state_continuity() {
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
                SuperSmoother::indicator_by_assets::<4>(&inputs_4, &options, None)
                    .expect("SIMD by-assets SuperSmoother failed on first chunk");

            for (asset_idx, (stock_symbol, close)) in stock_data.iter().enumerate() {
                let mut batch_output = simd_first[asset_idx][0].clone();

                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in close_chunks.by_ref() {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[chunk], None)
                        .expect("batch_indicator failed");
                    batch_output.extend_from_slice(&chunk_outputs[0]);
                }
                let rem = close_chunks.remainder();
                if !rem.is_empty() {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_output.extend_from_slice(&chunk_outputs[0]);
                }

                let (scalar_outputs, _) =
                    SuperSmoother::indicator(&[close.as_slice()], &options, None)
                        .expect("Scalar SuperSmoother failed");

                assert_eq!(
                    batch_output.len(),
                    scalar_outputs[0].len(),
                    "Length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &sv)) in batch_output
                    .iter()
                    .zip(scalar_outputs[0].iter())
                    .enumerate()
                {
                    if bv.is_nan() {
                        panic!(
                            "SIMD by-assets SuperSmoother has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "SIMD by-assets SuperSmoother has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, sv,
                        "Mismatch at index {i}: simd+batch={bv}, scalar={sv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: state continuity — SIMD first chunk + batch_indicator
    // remainder must equal full scalar run.
    // -------------------------------------------------------------------------

    #[test]
    fn test_supersmoother_simd_by_options_state_continuity() {
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
                SuperSmoother::indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD by-options SuperSmoother failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch_output = simd_first[opt_idx][0].clone();

                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                for chunk in close_chunks.by_ref() {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[chunk], None)
                        .expect("batch_indicator failed");
                    batch_output.extend_from_slice(&chunk_outputs[0]);
                }
                let rem = close_chunks.remainder();
                if !rem.is_empty() {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_output.extend_from_slice(&chunk_outputs[0]);
                }

                let (scalar_outputs, _) =
                    SuperSmoother::indicator(&[close.as_slice()], options, None)
                        .expect("Scalar SuperSmoother failed");

                assert_eq!(
                    batch_output.len(),
                    scalar_outputs[0].len(),
                    "Length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &sv)) in batch_output
                    .iter()
                    .zip(scalar_outputs[0].iter())
                    .enumerate()
                {
                    if bv.is_nan() {
                        panic!(
                            "SIMD by-options SuperSmoother has NaN at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if bv.is_infinite() {
                        panic!(
                            "SIMD by-options SuperSmoother has infinity at index {i}: batch={bv}, \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert_eq!(
                        bv, sv,
                        "Mismatch at index {i}: simd+batch={bv}, scalar={sv}, \
                         stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }
}
