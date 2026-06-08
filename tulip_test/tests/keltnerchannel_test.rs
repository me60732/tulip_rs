#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::atr::indicator as atr_indicator;
    use tulip_rs::indicators::ema::indicator as ema_indicator;
    use tulip_rs::indicators::keltnerchannel::{
        indicator, indicator_by_assets, indicator_by_options, min_data, TIndicatorState,
    };
    use tulip_rs::indicators::tr::indicator as tr_indicator;
    use tulip_test::database::{get_all_stock_data, init_database_data};

    const CHUNK_SIZE: usize = 100;
    const EPSILON: f64 = 1e-10;

    const HIGH: [f64; 15] = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ];
    const LOW: [f64; 15] = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ];
    const CLOSE: [f64; 15] = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];

    // period, step — step doesn't affect middle/atr/tr, any valid value works
    const OPTIONS_LIST: [[f64; 2]; 4] = [[5.0, 3.0], [14.0, 2.0], [30.0, 2.0], [50.0, 2.0]];

    fn expand_inputs() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut high_vec = HIGH.to_vec();
        let mut low_vec = LOW.to_vec();
        let mut close_vec = CLOSE.to_vec();
        for _ in 0..15 {
            high_vec.extend_from_slice(&HIGH);
            low_vec.extend_from_slice(&LOW);
            close_vec.extend_from_slice(&CLOSE);
        }
        (high_vec, low_vec, close_vec)
    }

    fn get_arrays(stock_data: &[tulip_test::database::EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
        let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
        let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
        (high, low, close)
    }

    // -------------------------------------------------------------------------
    // chunked state == full output
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let (full_outputs, _) =
                    indicator(&inputs, &options, None).expect("Rust KC indicator failed");

                let mut batch_lower: Vec<f64> = Vec::new();
                let mut batch_middle: Vec<f64> = Vec::new();
                let mut batch_upper: Vec<f64> = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    let (outputs, _) =
                        indicator(&inputs, &options, None).expect("Rust KC indicator failed");
                    batch_lower.extend_from_slice(&outputs[0]);
                    batch_middle.extend_from_slice(&outputs[1]);
                    batch_upper.extend_from_slice(&outputs[2]);
                } else {
                    let chunk_inputs = [
                        &high[..min_data_val],
                        &low[..min_data_val],
                        &close[..min_data_val],
                    ];
                    let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("Rust KC indicator failed on first chunk");
                    batch_lower.extend_from_slice(&first_outputs[0]);
                    batch_middle.extend_from_slice(&first_outputs[1]);
                    batch_upper.extend_from_slice(&first_outputs[2]);

                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for ((hc, lc), cc) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                    {
                        let chunk_outputs = state
                            .batch_indicator(&[hc, lc, cc], None)
                            .expect("KC batch_indicator failed");
                        batch_lower.extend_from_slice(&chunk_outputs[0]);
                        batch_middle.extend_from_slice(&chunk_outputs[1]);
                        batch_upper.extend_from_slice(&chunk_outputs[2]);
                    }

                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();
                    if !high_rem.is_empty() {
                        let chunk_outputs = state
                            .batch_indicator(&[high_rem, low_rem, close_rem], None)
                            .expect("KC batch_indicator failed on remainder");
                        batch_lower.extend_from_slice(&chunk_outputs[0]);
                        batch_middle.extend_from_slice(&chunk_outputs[1]);
                        batch_upper.extend_from_slice(&chunk_outputs[2]);
                    }
                }

                for (band_label, full, batch) in [
                    ("lower", &full_outputs[0], &batch_lower),
                    ("middle", &full_outputs[1], &batch_middle),
                    ("upper", &full_outputs[2], &batch_upper),
                ] {
                    assert_eq!(
                        full.len(),
                        batch.len(),
                        "{band_label} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&fv, &bv)) in full.iter().zip(batch.iter()).enumerate() {
                        assert_eq!(
                            fv, bv,
                            "{band_label} mismatch at index {i}: stock={stock_symbol}, \
                             options={options:?}, full={fv}, batch={bv}"
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // middle band == standalone EMA of close
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_middle_ema() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let ema_inputs = [close.as_slice()];

        for options in OPTIONS_LIST {
            let ema_options = [options[0]];

            let (kc_outputs, _) =
                indicator(&inputs, &options, None).expect("Rust KC indicator failed");
            let kc_middle = &kc_outputs[1];

            let (ema_outputs, _) =
                ema_indicator(&ema_inputs, &ema_options, None).expect("Rust EMA indicator failed");
            let ema_line = &ema_outputs[0];

            let compare_len = kc_middle.len().min(ema_line.len());
            for i in 0..compare_len {
                let kc_val = kc_middle[kc_middle.len() - 1 - i];
                let ema_val = ema_line[ema_line.len() - 1 - i];

                if kc_val.is_nan() || kc_val.is_infinite() {
                    panic!("KC middle is NaN/inf at index {i} (from end), options={options:?}");
                }
                if ema_val.is_nan() || ema_val.is_infinite() {
                    continue;
                }
                assert!(
                    approx_eq!(f64, kc_val, ema_val, epsilon = EPSILON),
                    "KC middle vs EMA mismatch at index {i} (from end): \
                     kc={kc_val}, ema={ema_val}, diff={}, options={options:?}",
                    (kc_val - ema_val).abs()
                );
            }
        }
    }

    #[test]
    fn test_keltnerchannel_middle_ema_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let ema_inputs = [close.as_slice()];

            for options in OPTIONS_LIST {
                let ema_options = [options[0]];

                let (kc_outputs, _) =
                    indicator(&inputs, &options, None).expect("Rust KC indicator failed");
                let kc_middle = &kc_outputs[1];

                let (ema_outputs, _) = ema_indicator(&ema_inputs, &ema_options, None)
                    .expect("Rust EMA indicator failed");
                let ema_line = &ema_outputs[0];

                let compare_len = kc_middle.len().min(ema_line.len());
                for i in 0..compare_len {
                    let kc_val = kc_middle[kc_middle.len() - 1 - i];
                    let ema_val = ema_line[ema_line.len() - 1 - i];

                    if kc_val.is_nan() || kc_val.is_infinite() {
                        panic!(
                            "KC middle is NaN/inf at index {i} (from end), \
                             options={options:?}, stock={stock_symbol}"
                        );
                    }
                    if ema_val.is_nan() || ema_val.is_infinite() {
                        continue;
                    }
                    assert!(
                        approx_eq!(f64, kc_val, ema_val, epsilon = EPSILON),
                        "KC middle vs EMA mismatch at index {i} (from end): \
                         kc={kc_val}, ema={ema_val}, diff={}, options={options:?}, \
                         stock={stock_symbol}",
                        (kc_val - ema_val).abs()
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // optional atr output vs standalone atr indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_optional_atr() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let atr_options = [options[0]];

            // outputs: [lower, middle, upper, atr, tr]
            let (kc_outputs, _) = indicator(&inputs, &options, Some(&[true, false]))
                .expect("Rust KC indicator failed");
            let kc_atr = &kc_outputs[3];

            let (atr_outputs, _) =
                atr_indicator(&inputs, &atr_options, None).expect("Rust ATR indicator failed");
            let atr_line = &atr_outputs[0];

            let compare_len = kc_atr.len().min(atr_line.len());
            for i in 0..compare_len {
                let kc_val = kc_atr[kc_atr.len() - 1 - i];
                let atr_val = atr_line[atr_line.len() - 1 - i];

                if kc_val.is_nan() || kc_val.is_infinite() {
                    panic!(
                        "KC atr optional is NaN/inf at index {i} (from end), options={options:?}"
                    );
                }
                if atr_val.is_nan() || atr_val.is_infinite() {
                    continue;
                }
                assert!(
                    approx_eq!(f64, kc_val, atr_val, epsilon = EPSILON),
                    "KC atr vs standalone atr mismatch at index {i} (from end): \
                     kc={kc_val}, atr={atr_val}, diff={}, options={options:?}",
                    (kc_val - atr_val).abs()
                );
            }
        }
    }

    #[test]
    fn test_keltnerchannel_optional_atr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let atr_options = [options[0]];

                let (kc_outputs, _) = indicator(&inputs, &options, Some(&[true, false]))
                    .expect("Rust KC indicator failed");
                let kc_atr = &kc_outputs[3];

                let (atr_outputs, _) =
                    atr_indicator(&inputs, &atr_options, None).expect("Rust ATR indicator failed");
                let atr_line = &atr_outputs[0];

                let compare_len = kc_atr.len().min(atr_line.len());
                for i in 0..compare_len {
                    let kc_val = kc_atr[kc_atr.len() - 1 - i];
                    let atr_val = atr_line[atr_line.len() - 1 - i];

                    if kc_val.is_nan() || kc_val.is_infinite() {
                        panic!(
                            "KC atr optional is NaN/inf at index {i} (from end), \
                             options={options:?}, stock={stock_symbol}"
                        );
                    }
                    if atr_val.is_nan() || atr_val.is_infinite() {
                        continue;
                    }
                    assert!(
                        approx_eq!(f64, kc_val, atr_val, epsilon = EPSILON),
                        "KC atr vs standalone atr mismatch at index {i} (from end): \
                         kc={kc_val}, atr={atr_val}, diff={}, options={options:?}, \
                         stock={stock_symbol}",
                        (kc_val - atr_val).abs()
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // optional tr output vs standalone tr indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_optional_tr() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let tr_options: [f64; 0] = [];

            // outputs: [lower, middle, upper, atr, tr]
            let (kc_outputs, _) = indicator(&inputs, &options, Some(&[true, true]))
                .expect("Rust KC indicator failed");
            let kc_tr = &kc_outputs[4];

            let (tr_outputs, _) =
                tr_indicator(&inputs, &tr_options, None).expect("Rust TR indicator failed");
            let tr_line = &tr_outputs[0];

            let compare_len = kc_tr.len().min(tr_line.len());
            for i in 0..compare_len {
                let kc_val = kc_tr[kc_tr.len() - 1 - i];
                let tr_val = tr_line[tr_line.len() - 1 - i];

                if kc_val.is_nan() || kc_val.is_infinite() {
                    panic!(
                        "KC tr optional is NaN/inf at index {i} (from end), options={options:?}"
                    );
                }
                if tr_val.is_nan() || tr_val.is_infinite() {
                    continue;
                }
                assert!(
                    approx_eq!(f64, kc_val, tr_val, epsilon = EPSILON),
                    "KC tr vs standalone tr mismatch at index {i} (from end): \
                     kc={kc_val}, tr={tr_val}, diff={}, options={options:?}",
                    (kc_val - tr_val).abs()
                );
            }
        }
    }

    #[test]
    fn test_keltnerchannel_optional_tr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let tr_options: [f64; 0] = [];

                let (kc_outputs, _) = indicator(&inputs, &options, Some(&[true, true]))
                    .expect("Rust KC indicator failed");
                let kc_tr = &kc_outputs[4];

                let (tr_outputs, _) =
                    tr_indicator(&inputs, &tr_options, None).expect("Rust TR indicator failed");
                let tr_line = &tr_outputs[0];

                let compare_len = kc_tr.len().min(tr_line.len());
                for i in 0..compare_len {
                    let kc_val = kc_tr[kc_tr.len() - 1 - i];
                    let tr_val = tr_line[tr_line.len() - 1 - i];

                    if kc_val.is_nan() || kc_val.is_infinite() {
                        panic!(
                            "KC tr optional is NaN/inf at index {i} (from end), \
                             options={options:?}, stock={stock_symbol}"
                        );
                    }
                    if tr_val.is_nan() || tr_val.is_infinite() {
                        continue;
                    }
                    assert!(
                        approx_eq!(f64, kc_val, tr_val, epsilon = EPSILON),
                        "KC tr vs standalone tr mismatch at index {i} (from end): \
                         kc={kc_val}, tr={tr_val}, diff={}, options={options:?}, \
                         stock={stock_symbol}",
                        (kc_val - tr_val).abs()
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets == regular indicator (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_simd_by_assets_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        let asset0: [&[f64]; 3] = [&stock_data[0].1, &stock_data[0].2, &stock_data[0].3];
        let asset1: [&[f64]; 3] = [&stock_data[1].1, &stock_data[1].2, &stock_data[1].3];
        let asset2: [&[f64]; 3] = [&stock_data[2].1, &stock_data[2].2, &stock_data[2].3];
        let asset3: [&[f64]; 3] = [&stock_data[3].1, &stock_data[3].2, &stock_data[3].3];
        let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

        for options in OPTIONS_LIST {
            let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by assets KC indicator failed");

            for (stock_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (regular_results, _) =
                    indicator(&inputs, &options, None).expect("KC indicator failed");

                for (out_idx, out_name) in ["lower", "middle", "upper"].iter().enumerate() {
                    let simd = &simd_results[stock_idx][out_idx];
                    let regular = &regular_results[out_idx];

                    assert_eq!(
                        simd.len(),
                        regular.len(),
                        "SIMD by assets {out_name} length mismatch: \
                         stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd.iter().zip(regular.iter()).enumerate() {
                        if sv.is_nan() || sv.is_infinite() {
                            panic!(
                                "SIMD by assets KC {out_name} is NaN/inf at index {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        assert!(
                            approx_eq!(f64, sv, rv, epsilon = EPSILON),
                            "SIMD by assets KC {out_name} mismatch at index {i}: \
                             simd={sv}, regular={rv}, diff={}, \
                             stock={stock_symbol}, options={options:?}",
                            (sv - rv).abs()
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options == regular indicator (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by options KC indicator failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    indicator(&inputs, options, None).expect("KC indicator failed");

                for (out_idx, out_name) in ["lower", "middle", "upper"].iter().enumerate() {
                    let simd = &simd_results[opt_idx][out_idx];
                    let regular = &regular_results[out_idx];

                    assert_eq!(
                        simd.len(),
                        regular.len(),
                        "SIMD by options {out_name} length mismatch: \
                         stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd.iter().zip(regular.iter()).enumerate() {
                        if sv.is_nan() || sv.is_infinite() {
                            panic!(
                                "SIMD by options KC {out_name} is NaN/inf at index {i}: \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                        assert!(
                            approx_eq!(f64, sv, rv, epsilon = EPSILON),
                            "SIMD by options KC {out_name} mismatch at index {i}: \
                             simd={sv}, regular={rv}, diff={}, \
                             stock={stock_symbol}, options={options:?}",
                            (sv - rv).abs()
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options optional ATR == scalar optional ATR (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_simd_by_options_optional_atr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            // outputs: [lower, middle, upper, atr, tr(empty)]
            let (simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(&[true, false]))
                    .expect("SIMD by options KC indicator failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    indicator(&inputs, options, Some(&[true, false])).expect("KC indicator failed");

                let simd_atr = &simd_results[opt_idx][3];
                let regular_atr = &regular_results[3];

                assert_eq!(
                    simd_atr.len(),
                    regular_atr.len(),
                    "SIMD by options ATR length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_atr.iter().zip(regular_atr.iter()).enumerate() {
                    if sv.is_nan() || sv.is_infinite() {
                        panic!(
                            "SIMD by options KC ATR is NaN/inf at index {i}: \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert!(
                        approx_eq!(f64, sv, rv, epsilon = EPSILON),
                        "SIMD by options KC ATR mismatch at index {i}: \
                         simd={sv}, regular={rv}, diff={}, \
                         stock={stock_symbol}, options={options:?}",
                        (sv - rv).abs()
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options optional TR == scalar optional TR (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_simd_by_options_optional_tr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let options_4 = [
                &OPTIONS_LIST[0],
                &OPTIONS_LIST[1],
                &OPTIONS_LIST[2],
                &OPTIONS_LIST[3],
            ];
            // outputs: [lower, middle, upper, atr(empty), tr]
            let (simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(&[false, true]))
                    .expect("SIMD by options KC indicator failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    indicator(&inputs, options, Some(&[false, true])).expect("KC indicator failed");

                let simd_tr = &simd_results[opt_idx][4];
                let regular_tr = &regular_results[4];

                assert_eq!(
                    simd_tr.len(),
                    regular_tr.len(),
                    "SIMD by options TR length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_tr.iter().zip(regular_tr.iter()).enumerate() {
                    if sv.is_nan() || sv.is_infinite() {
                        panic!(
                            "SIMD by options KC TR is NaN/inf at index {i}: \
                             stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if !approx_eq!(f64, sv, rv, epsilon = EPSILON) {
                        println!(
                            "Test failed at index {}: \nSIMD = {:?}, \n\n\nRegular = {:?}, Options = {:?}",
                            i, simd_tr, regular_tr, options
                        );
                        panic!(
                            "SIMD by options KC TR mismatch at index {i}: \
                             simd={sv}, regular={rv}, diff={}, \
                             stock={stock_symbol}, options={options:?}",
                            (sv - rv).abs()
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets optional outputs: ATR and TR must match scalar
    // -------------------------------------------------------------------------

    #[test]
    fn test_keltnerchannel_simd_by_assets_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 3] = [&stock_data[0].1, &stock_data[0].2, &stock_data[0].3];
            let asset1: [&[f64]; 3] = [&stock_data[1].1, &stock_data[1].2, &stock_data[1].3];
            let asset2: [&[f64]; 3] = [&stock_data[2].1, &stock_data[2].2, &stock_data[2].3];
            let asset3: [&[f64]; 3] = [&stock_data[3].1, &stock_data[3].2, &stock_data[3].3];
            let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_results, _) =
                indicator_by_assets::<4>(&inputs_4, &options, Some(&[true, true]))
                    .expect("SIMD by-assets Keltner Channel with optional outputs failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_results, _) = indicator(&scalar_inputs, &options, Some(&[true, true]))
                    .expect("Scalar Keltner Channel with optional outputs failed");

                // Compare all 5 outputs (lower, middle, upper, atr, tr)
                for out_idx in 0..5 {
                    let simd_out = &simd_results[asset_idx][out_idx];
                    let scalar_out = &scalar_results[out_idx];
                    assert_eq!(
                        simd_out.len(), scalar_out.len(),
                        "output[{out_idx}] length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out).enumerate() {
                        if !approx_eq!(f64, sv, rv, epsilon = EPSILON) {
                            let start = i.saturating_sub(5);
                            let end = (i + 6).min(simd_out.len());
                            println!(
                                "output[{out_idx}] mismatch at index {i}: simd={:?}, scalar={:?}, options={options:?}",
                                &simd_out[start..end], &scalar_out[start..end]
                            );
                            panic!(
                                "output[{out_idx}] mismatch at index {i}: simd={sv}, scalar={rv}, \
                                 stock={stock_symbol}, options={options:?}"
                            );
                        }
                    }
                }
                println!(
                    "✓ SIMD by-assets optional outputs match scalar for stock={stock_symbol}, options={options:?}"
                );
            }
        }
        println!("✓ All SIMD by-assets Keltner Channel optional output tests passed!");
    }
}
