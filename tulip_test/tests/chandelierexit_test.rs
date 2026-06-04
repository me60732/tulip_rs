#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::atr::indicator as atr_indicator;
    use tulip_rs::indicators::chandelierexit::{
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
    fn test_chandelierexit_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let (full_outputs, _) =
                    indicator(&inputs, &options, None).expect("Rust CE indicator failed");

                let mut batch_long: Vec<f64> = Vec::new();
                let mut batch_short: Vec<f64> = Vec::new();

                let min_data_val = min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    let (outputs, _) =
                        indicator(&inputs, &options, None).expect("Rust CE indicator failed");
                    batch_long.extend_from_slice(&outputs[0]);
                    batch_short.extend_from_slice(&outputs[1]);
                } else {
                    let chunk_inputs = [
                        &high[..min_data_val],
                        &low[..min_data_val],
                        &close[..min_data_val],
                    ];
                    let (first_outputs, mut state) = indicator(&chunk_inputs, &options, None)
                        .expect("Rust CE indicator failed on first chunk");
                    batch_long.extend_from_slice(&first_outputs[0]);
                    batch_short.extend_from_slice(&first_outputs[1]);

                    let mut high_chunks = high[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut low_chunks = low[min_data_val..].chunks_exact(CHUNK_SIZE);
                    let mut close_chunks = close[min_data_val..].chunks_exact(CHUNK_SIZE);

                    for ((high_chunk, low_chunk), close_chunk) in high_chunks
                        .by_ref()
                        .zip(low_chunks.by_ref())
                        .zip(close_chunks.by_ref())
                    {
                        let chunk_outputs = state
                            .batch_indicator(&[high_chunk, low_chunk, close_chunk], None)
                            .expect("CE batch_indicator failed");
                        batch_long.extend_from_slice(&chunk_outputs[0]);
                        batch_short.extend_from_slice(&chunk_outputs[1]);
                    }

                    let high_rem = high_chunks.remainder();
                    let low_rem = low_chunks.remainder();
                    let close_rem = close_chunks.remainder();

                    if !high_rem.is_empty() {
                        let chunk_outputs = state
                            .batch_indicator(&[high_rem, low_rem, close_rem], None)
                            .expect("CE batch_indicator failed on remainder");
                        batch_long.extend_from_slice(&chunk_outputs[0]);
                        batch_short.extend_from_slice(&chunk_outputs[1]);
                    }
                }

                assert_eq!(
                    full_outputs[0].len(),
                    batch_long.len(),
                    "long length mismatch: stock={stock_symbol}, options={options:?}"
                );
                assert_eq!(
                    full_outputs[1].len(),
                    batch_short.len(),
                    "short length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&full_val, &batch_val)) in
                    full_outputs[0].iter().zip(batch_long.iter()).enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "long mismatch at index {i}: stock={stock_symbol}, options={options:?}, full={full_val}, batch={batch_val}"
                    );
                }
                for (i, (&full_val, &batch_val)) in
                    full_outputs[1].iter().zip(batch_short.iter()).enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "short mismatch at index {i}: stock={stock_symbol}, options={options:?}, full={full_val}, batch={batch_val}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // optional atr output vs standalone atr indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_chandelierexit_optional_atr() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let atr_options = [options[0]];

            let (ce_outputs, _) = indicator(&inputs, &options, Some(&[true, false]))
                .expect("Rust CE indicator failed");
            let ce_atr = &ce_outputs[2];

            let (atr_outputs, _) =
                atr_indicator(&inputs, &atr_options, None).expect("Rust ATR indicator failed");
            let atr_line = &atr_outputs[0];

            let compare_len = ce_atr.len().min(atr_line.len());
            for i in 0..compare_len {
                let ce_val = ce_atr[ce_atr.len() - 1 - i];
                let atr_val = atr_line[atr_line.len() - 1 - i];

                if ce_val.is_nan() || ce_val.is_infinite() {
                    panic!(
                        "CE atr optional is NaN/inf at index {i} (from end), options={options:?}"
                    );
                }
                if atr_val.is_nan() || atr_val.is_infinite() {
                    continue;
                }
                assert!(
                    approx_eq!(f64, ce_val, atr_val, epsilon = EPSILON),
                    "CE atr vs standalone atr mismatch at index {i} (from end): ce={ce_val}, atr={atr_val}, diff={}, options={options:?}",
                    (ce_val - atr_val).abs()
                );
            }
        }
    }

    #[test]
    fn test_chandelierexit_optional_atr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let atr_options = [options[0]];

                let (ce_outputs, _) = indicator(&inputs, &options, Some(&[true, false]))
                    .expect("Rust CE indicator failed");
                let ce_atr = &ce_outputs[2];

                let (atr_outputs, _) =
                    atr_indicator(&inputs, &atr_options, None).expect("Rust ATR indicator failed");
                let atr_line = &atr_outputs[0];

                let compare_len = ce_atr.len().min(atr_line.len());
                for i in 0..compare_len {
                    let ce_val = ce_atr[ce_atr.len() - 1 - i];
                    let atr_val = atr_line[atr_line.len() - 1 - i];

                    if ce_val.is_nan() || ce_val.is_infinite() {
                        panic!(
                            "CE atr optional is NaN/inf at index {i} (from end), options={options:?}, stock={stock_symbol}"
                        );
                    }
                    if atr_val.is_nan() || atr_val.is_infinite() {
                        continue;
                    }
                    assert!(
                        approx_eq!(f64, ce_val, atr_val, epsilon = EPSILON),
                        "CE atr vs standalone atr mismatch at index {i} (from end): ce={ce_val}, atr={atr_val}, diff={}, options={options:?}, stock={stock_symbol}",
                        (ce_val - atr_val).abs()
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // optional tr output vs standalone tr indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_chandelierexit_optional_tr() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let tr_options: [f64; 0] = [];

            let (ce_outputs, _) = indicator(&inputs, &options, Some(&[true, true]))
                .expect("Rust CE indicator failed");
            let ce_tr = &ce_outputs[3];

            let (tr_outputs, _) =
                tr_indicator(&inputs, &tr_options, None).expect("Rust TR indicator failed");
            let tr_line = &tr_outputs[0];

            let compare_len = ce_tr.len().min(tr_line.len());
            for i in 0..compare_len {
                let ce_val = ce_tr[ce_tr.len() - 1 - i];
                let tr_val = tr_line[tr_line.len() - 1 - i];

                if ce_val.is_nan() || ce_val.is_infinite() {
                    panic!(
                        "CE tr optional is NaN/inf at index {i} (from end), options={options:?}"
                    );
                }
                if tr_val.is_nan() || tr_val.is_infinite() {
                    continue;
                }
                assert!(
                    approx_eq!(f64, ce_val, tr_val, epsilon = EPSILON),
                    "CE tr vs standalone tr mismatch at index {i} (from end): ce={ce_val}, tr={tr_val}, diff={}, options={options:?}",
                    (ce_val - tr_val).abs()
                );
            }
        }
    }

    #[test]
    fn test_chandelierexit_optional_tr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let tr_options: [f64; 0] = [];

                let (ce_outputs, _) = indicator(&inputs, &options, Some(&[true, true]))
                    .expect("Rust CE indicator failed");
                let ce_tr = &ce_outputs[3];

                let (tr_outputs, _) =
                    tr_indicator(&inputs, &tr_options, None).expect("Rust TR indicator failed");
                let tr_line = &tr_outputs[0];

                let compare_len = ce_tr.len().min(tr_line.len());
                for i in 0..compare_len {
                    let ce_val = ce_tr[ce_tr.len() - 1 - i];
                    let tr_val = tr_line[tr_line.len() - 1 - i];

                    if ce_val.is_nan() || ce_val.is_infinite() {
                        panic!(
                            "CE tr optional is NaN/inf at index {i} (from end), options={options:?}, stock={stock_symbol}"
                        );
                    }
                    if tr_val.is_nan() || tr_val.is_infinite() {
                        continue;
                    }
                    assert!(
                        approx_eq!(f64, ce_val, tr_val, epsilon = EPSILON),
                        "CE tr vs standalone tr mismatch at index {i} (from end): ce={ce_val}, tr={tr_val}, diff={}, options={options:?}, stock={stock_symbol}",
                        (ce_val - tr_val).abs()
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets == regular indicator (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_chandelierexit_simd_by_assets_vs_regular_database() {
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
                .expect("SIMD by assets CE indicator failed");

            for (stock_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (regular_results, _) =
                    indicator(&inputs, &options, None).expect("CE indicator failed");

                for (out_idx, out_name) in ["long", "short"].iter().enumerate() {
                    let simd = &simd_results[stock_idx][out_idx];
                    let regular = &regular_results[out_idx];

                    assert_eq!(
                        simd.len(),
                        regular.len(),
                        "SIMD by assets {out_name} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd.iter().zip(regular.iter()).enumerate() {
                        if sv.is_nan() || sv.is_infinite() {
                            panic!(
                                "SIMD by assets CE {out_name} is NaN/inf at index {i}: stock={stock_symbol}, options={options:?}"
                            );
                        }
                        if !approx_eq!(f64, sv, rv, epsilon = EPSILON) {
                            let from = i.saturating_sub(10);
                            let to = if i + 10 < simd.len() {
                                i + 10
                            } else {
                                simd.len()
                            };
                            println!(
                                "\nSimd: {:?}\n\n\nRegular: {:?}",
                                &simd[from..to],
                                &regular[from..to]
                            );
                            panic!(
                                "SIMD by assets CE {out_name} mismatch at index {i}: simd={sv}, regular={rv}, diff={}, stock={stock_symbol}, options={options:?}",
                                (sv - rv).abs()
                            );
                        }
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options == regular indicator (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_chandelierexit_simd_by_options_vs_regular_database() {
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
                .expect("SIMD by options CE indicator failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    indicator(&inputs, options, None).expect("CE indicator failed");

                for (out_idx, out_name) in ["long", "short"].iter().enumerate() {
                    let simd = &simd_results[opt_idx][out_idx];
                    let regular = &regular_results[out_idx];

                    assert_eq!(
                        simd.len(),
                        regular.len(),
                        "SIMD by options {out_name} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd.iter().zip(regular.iter()).enumerate() {
                        if sv.is_nan() || sv.is_infinite() {
                            panic!(
                                "SIMD by options CE {out_name} is NaN/inf at index {i}: stock={stock_symbol}, options={options:?}"
                            );
                        }
                        assert!(
                            approx_eq!(f64, sv, rv, epsilon = EPSILON),
                            "SIMD by options CE {out_name} mismatch at index {i}: simd={sv}, regular={rv}, diff={}, stock={stock_symbol}, options={options:?}",
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
    fn test_chandelierexit_simd_by_options_optional_atr_database() {
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
            // outputs: [long, short, atr, tr(empty)]
            let (simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(&[true, false]))
                    .expect("SIMD by options CE indicator failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    indicator(&inputs, options, Some(&[true, false])).expect("CE indicator failed");

                let simd_atr = &simd_results[opt_idx][2];
                let regular_atr = &regular_results[2];

                assert_eq!(
                    simd_atr.len(),
                    regular_atr.len(),
                    "SIMD by options ATR length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_atr.iter().zip(regular_atr.iter()).enumerate() {
                    if sv.is_nan() || sv.is_infinite() {
                        panic!(
                            "SIMD by options CE ATR is NaN/inf at index {i}: stock={stock_symbol}, options={options:?}"
                        );
                    }
                    assert!(
                        approx_eq!(f64, sv, rv, epsilon = EPSILON),
                        "SIMD by options CE ATR mismatch at index {i}: simd={sv}, regular={rv}, diff={}, stock={stock_symbol}, options={options:?}",
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
    fn test_chandelierexit_simd_by_options_optional_tr_database() {
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
            // outputs: [long, short, atr(empty), tr]
            let (simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(&[false, true]))
                    .expect("SIMD by options CE indicator failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (regular_results, _) =
                    indicator(&inputs, options, Some(&[false, true])).expect("CE indicator failed");

                let simd_tr = &simd_results[opt_idx][3];
                let regular_tr = &regular_results[3];

                assert_eq!(
                    simd_tr.len(),
                    regular_tr.len(),
                    "SIMD by options TR length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_tr.iter().zip(regular_tr.iter()).enumerate() {
                    if sv.is_nan() || sv.is_infinite() {
                        panic!(
                            "SIMD by options CE TR is NaN/inf at index {i}: stock={stock_symbol}, options={options:?}"
                        );
                    }
                    if !approx_eq!(f64, sv, rv, epsilon = EPSILON) {
                        println!(
                            "Test failed at index {}: \nSIMD = {:?}, \n\n\nRegular = {:?}, Options = {:?}",
                            i, simd_tr, regular_tr, options
                        );
                        panic!(
                            "SIMD by options CE TR mismatch at index {i}: simd={sv}, regular={rv}, diff={}, stock={stock_symbol}, options={options:?}",
                            (sv - rv).abs()
                        );
                    }
                }
            }
        }
    }
}
