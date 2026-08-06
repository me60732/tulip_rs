#[cfg(test)]
mod tests {
    use float_cmp::approx_eq;
    use tulip_rs::indicators::atr::Atr;
    use tulip_rs::indicators::medprice::Medprice;
    use tulip_rs::indicators::supertrend::{Indicator, SuperTrend, TIndicatorState, indicator_by_assets, indicator_by_options};
    use tulip_rs::indicators::tr::Tr;
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

    // [period, step]
    const OPTIONS_LIST: [[f64; 2]; 4] = [[7.0, 3.0], [10.0, 3.0], [14.0, 2.0], [20.0, 2.0]];

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
    // State continuity: chunked batch_indicator == full indicator output
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_database_state() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let (full_outputs, _) =
                    SuperTrend::indicator(&inputs, &options, None).expect("Rust Supertrend failed");

                let mut batch_st: Vec<f64> = Vec::new();
                let min_data_val = SuperTrend::min_data(&options).max(CHUNK_SIZE);

                if high.len() <= min_data_val {
                    let (outputs, _) = SuperTrend::indicator(&inputs, &options, None)
                        .expect("Rust Supertrend failed");
                    batch_st.extend_from_slice(&outputs[0]);
                } else {
                    let chunk_inputs = [
                        &high[..min_data_val],
                        &low[..min_data_val],
                        &close[..min_data_val],
                    ];
                    let (first_outputs, mut state) =
                        SuperTrend::indicator(&chunk_inputs, &options, None)
                            .expect("Rust Supertrend failed on first chunk");
                    batch_st.extend_from_slice(&first_outputs[0]);

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
                            .expect("Supertrend batch_indicator failed");
                        batch_st.extend_from_slice(&chunk_outputs[0]);
                    }

                    let hr = high_chunks.remainder();
                    let lr = low_chunks.remainder();
                    let cr = close_chunks.remainder();
                    if !hr.is_empty() {
                        let chunk_outputs = state
                            .batch_indicator(&[hr, lr, cr], None)
                            .expect("Supertrend batch_indicator failed on remainder");
                        batch_st.extend_from_slice(&chunk_outputs[0]);
                    }
                }

                assert_eq!(
                    full_outputs[0].len(),
                    batch_st.len(),
                    "supertrend length mismatch: stock={stock_symbol}, options={options:?}"
                );

                for (i, (&full_val, &batch_val)) in
                    full_outputs[0].iter().zip(batch_st.iter()).enumerate()
                {
                    assert_eq!(
                        full_val, batch_val,
                        "supertrend mismatch at index {i}: stock={stock_symbol}, options={options:?}, full={full_val}, batch={batch_val}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Optional atr output vs standalone ATR indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_optional_atr() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let atr_options = [options[0]];

            // optional_outputs[0] = atr → outputs[1]
            let (st_outputs, _) =
                SuperTrend::indicator(&inputs, &options, Some(&[true, false, false]))
                    .expect("Rust Supertrend failed");
            let st_atr = &st_outputs[1];

            let (atr_outputs, _) =
                Atr::indicator(&inputs, &atr_options, None).expect("Rust ATR indicator failed");
            let atr_line = &atr_outputs[0];

            let compare_len = st_atr.len().min(atr_line.len());
            for i in 0..compare_len {
                let st_val = st_atr[st_atr.len() - 1 - i];
                let atr_val = atr_line[atr_line.len() - 1 - i];

                if st_val.is_nan() || st_val.is_infinite() {
                    panic!(
                        "Supertrend atr optional is NaN/inf at index {i} (from end), options={options:?}"
                    );
                }
                if atr_val.is_nan() || atr_val.is_infinite() {
                    continue;
                }
                assert!(
                    approx_eq!(f64, st_val, atr_val, epsilon = EPSILON),
                    "Supertrend atr vs standalone atr mismatch at index {i} (from end): st={st_val}, atr={atr_val}, diff={}, options={options:?}",
                    (st_val - atr_val).abs()
                );
            }
        }
    }

    #[test]
    fn test_supertrend_optional_atr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let atr_options = [options[0]];

                let (st_outputs, _) =
                    SuperTrend::indicator(&inputs, &options, Some(&[true, false, false]))
                        .expect("Rust Supertrend failed");
                let st_atr = &st_outputs[1];

                let (atr_outputs, _) =
                    Atr::indicator(&inputs, &atr_options, None).expect("Rust ATR indicator failed");
                let atr_line = &atr_outputs[0];

                let compare_len = st_atr.len().min(atr_line.len());
                for i in 0..compare_len {
                    let st_val = st_atr[st_atr.len() - 1 - i];
                    let atr_val = atr_line[atr_line.len() - 1 - i];

                    if st_val.is_nan() || st_val.is_infinite() {
                        panic!(
                            "Supertrend atr optional is NaN/inf at index {i} (from end), options={options:?}, stock={stock_symbol}"
                        );
                    }
                    if atr_val.is_nan() || atr_val.is_infinite() {
                        continue;
                    }
                    assert!(
                        approx_eq!(f64, st_val, atr_val, epsilon = EPSILON),
                        "Supertrend atr vs standalone atr mismatch at index {i} (from end): st={st_val}, atr={atr_val}, diff={}, options={options:?}, stock={stock_symbol}",
                        (st_val - atr_val).abs()
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Optional tr output vs standalone TR indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_optional_tr() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

        for options in OPTIONS_LIST {
            let tr_options: [f64; 0] = [];

            // optional_outputs[1] = tr → outputs[2]
            let (st_outputs, _) =
                SuperTrend::indicator(&inputs, &options, Some(&[false, true, false]))
                    .expect("Rust Supertrend failed");
            let st_tr = &st_outputs[2];

            let (tr_outputs, _) =
                Tr::indicator(&inputs, &tr_options, None).expect("Rust TR indicator failed");
            let tr_line = &tr_outputs[0];

            let compare_len = st_tr.len().min(tr_line.len());
            for i in 0..compare_len {
                let st_val = st_tr[st_tr.len() - 1 - i];
                let tr_val = tr_line[tr_line.len() - 1 - i];

                if st_val.is_nan() || st_val.is_infinite() {
                    panic!(
                        "Supertrend tr optional is NaN/inf at index {i} (from end), options={options:?}"
                    );
                }
                if tr_val.is_nan() || tr_val.is_infinite() {
                    continue;
                }
                assert!(
                    approx_eq!(f64, st_val, tr_val, epsilon = EPSILON),
                    "Supertrend tr vs standalone tr mismatch at index {i} (from end): st={st_val}, tr={tr_val}, diff={}, options={options:?}",
                    (st_val - tr_val).abs()
                );
            }
        }
    }

    #[test]
    fn test_supertrend_optional_tr_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            for options in OPTIONS_LIST {
                let tr_options: [f64; 0] = [];

                let (st_outputs, _) =
                    SuperTrend::indicator(&inputs, &options, Some(&[false, true, false]))
                        .expect("Rust Supertrend failed");
                let st_tr = &st_outputs[2];

                let (tr_outputs, _) =
                    Tr::indicator(&inputs, &tr_options, None).expect("Rust TR indicator failed");
                let tr_line = &tr_outputs[0];

                let compare_len = st_tr.len().min(tr_line.len());
                for i in 0..compare_len {
                    let st_val = st_tr[st_tr.len() - 1 - i];
                    let tr_val = tr_line[tr_line.len() - 1 - i];

                    if st_val.is_nan() || st_val.is_infinite() {
                        panic!(
                            "Supertrend tr optional is NaN/inf at index {i} (from end), options={options:?}, stock={stock_symbol}"
                        );
                    }
                    if tr_val.is_nan() || tr_val.is_infinite() {
                        continue;
                    }
                    assert!(
                        approx_eq!(f64, st_val, tr_val, epsilon = EPSILON),
                        "Supertrend tr vs standalone tr mismatch at index {i} (from end): st={st_val}, tr={tr_val}, diff={}, options={options:?}, stock={stock_symbol}",
                        (st_val - tr_val).abs()
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Optional medprice output vs standalone medprice indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_optional_medprice() {
        let (high, low, close) = expand_inputs();
        let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
        let medprice_inputs = [high.as_slice(), low.as_slice()];
        let medprice_options: [f64; 0] = [];

        for options in OPTIONS_LIST {
            // optional_outputs[2] = medprice → outputs[3]
            let (st_outputs, _) =
                SuperTrend::indicator(&inputs, &options, Some(&[false, false, true]))
                    .expect("Rust Supertrend failed");
            let st_med = &st_outputs[3];

            let (med_outputs, _) = Medprice::indicator(&medprice_inputs, &medprice_options, None)
                .expect("Rust medprice indicator failed");
            let med_line = &med_outputs[0];

            let compare_len = st_med.len().min(med_line.len());
            for i in 0..compare_len {
                let st_val = st_med[st_med.len() - 1 - i];
                let med_val = med_line[med_line.len() - 1 - i];

                if st_val.is_nan() || st_val.is_infinite() {
                    panic!(
                        "Supertrend medprice optional is NaN/inf at index {i} (from end), options={options:?}"
                    );
                }
                assert!(
                    approx_eq!(f64, st_val, med_val, epsilon = EPSILON),
                    "Supertrend medprice vs standalone medprice mismatch at index {i} (from end): st={st_val}, med={med_val}, diff={}, options={options:?}",
                    (st_val - med_val).abs()
                );
            }
        }
    }

    #[test]
    fn test_supertrend_optional_medprice_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let medprice_inputs = [high.as_slice(), low.as_slice()];
            let medprice_options: [f64; 0] = [];

            for options in OPTIONS_LIST {
                let (st_outputs, _) =
                    SuperTrend::indicator(&inputs, &options, Some(&[false, false, true]))
                        .expect("Rust Supertrend failed");
                let st_med = &st_outputs[3];

                let (med_outputs, _) =
                    Medprice::indicator(&medprice_inputs, &medprice_options, None)
                        .expect("Rust medprice indicator failed");
                let med_line = &med_outputs[0];

                let compare_len = st_med.len().min(med_line.len());
                for i in 0..compare_len {
                    let st_val = st_med[st_med.len() - 1 - i];
                    let med_val = med_line[med_line.len() - 1 - i];

                    if st_val.is_nan() || st_val.is_infinite() {
                        panic!(
                            "Supertrend medprice optional is NaN/inf at index {i} (from end), options={options:?}, stock={stock_symbol}"
                        );
                    }
                    assert!(
                        approx_eq!(f64, st_val, med_val, epsilon = EPSILON),
                        "Supertrend medprice vs standalone medprice mismatch at index {i} (from end): st={st_val}, med={med_val}, diff={}, options={options:?}, stock={stock_symbol}",
                        (st_val - med_val).abs()
                    );
                }
            }
        }
    }
    // -------------------------------------------------------------------------
    // SIMD by-assets: outputs match scalar supertrend per asset (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_simd_by_assets_vs_regular_database() {
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

            let (simd_results, _) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by-assets Supertrend failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) = SuperTrend::indicator(&scalar_inputs, &options, None)
                    .expect("Scalar Supertrend failed");

                let simd_st = &simd_results[asset_idx][0];
                let scalar_st = &scalar_outputs[0];

                assert_eq!(
                    simd_st.len(),
                    scalar_st.len(),
                    "st length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_st.iter().zip(scalar_st.iter()).enumerate() {
                    assert_eq!(
                        sv, rv,
                        "st mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: outputs match scalar supertrend per option set (database)
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_simd_by_options_vs_regular_database() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (stock_symbol, stock_data) in data {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let (simd_results, _) = indicator_by_options::<4>(&inputs, &options_4, None)
                .expect("SIMD by-options Supertrend failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                if high.len() < SuperTrend::min_data(options) {
                    continue;
                }
                let (scalar_outputs, _) = SuperTrend::indicator(&inputs, options, None)
                    .expect("Scalar Supertrend failed");

                let simd_st = &simd_results[opt_idx][0];
                let scalar_st = &scalar_outputs[0];

                assert_eq!(
                    simd_st.len(),
                    scalar_st.len(),
                    "st length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&sv, &rv)) in simd_st.iter().zip(scalar_st.iter()).enumerate() {
                    assert_eq!(
                        sv, rv,
                        "st mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets: first 1000 bars via SIMD, rest via batch_indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_simd_by_assets_state_continuity() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        const FIRST_CHUNK: usize = 1000;

        let stock_data: Vec<(String, Vec<f64>, Vec<f64>, Vec<f64>)> = data
            .iter()
            .take(4)
            .map(|(symbol, eod)| {
                let (high, low, close) = get_arrays(eod);
                (symbol.clone(), high, low, close)
            })
            .collect();

        for options in OPTIONS_LIST {
            let asset0: [&[f64]; 3] = [
                &stock_data[0].1[..FIRST_CHUNK],
                &stock_data[0].2[..FIRST_CHUNK],
                &stock_data[0].3[..FIRST_CHUNK],
            ];
            let asset1: [&[f64]; 3] = [
                &stock_data[1].1[..FIRST_CHUNK],
                &stock_data[1].2[..FIRST_CHUNK],
                &stock_data[1].3[..FIRST_CHUNK],
            ];
            let asset2: [&[f64]; 3] = [
                &stock_data[2].1[..FIRST_CHUNK],
                &stock_data[2].2[..FIRST_CHUNK],
                &stock_data[2].3[..FIRST_CHUNK],
            ];
            let asset3: [&[f64]; 3] = [
                &stock_data[3].1[..FIRST_CHUNK],
                &stock_data[3].2[..FIRST_CHUNK],
                &stock_data[3].3[..FIRST_CHUNK],
            ];
            let inputs_4: [&[&[f64]; 3]; 4] = [&asset0, &asset1, &asset2, &asset3];

            let (simd_first, mut states) = indicator_by_assets::<4>(&inputs_4, &options, None)
                .expect("SIMD by-assets Supertrend failed on first chunk");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let mut batch_st = simd_first[asset_idx][0].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("batch_indicator failed");
                    batch_st.extend_from_slice(&chunk_outputs[0]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[asset_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_st.extend_from_slice(&chunk_outputs[0]);
                }

                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) = SuperTrend::indicator(&scalar_inputs, &options, None)
                    .expect("Scalar Supertrend failed");

                assert_eq!(
                    batch_st.len(),
                    scalar_outputs[0].len(),
                    "st length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_st.iter().zip(scalar_outputs[0].iter()).enumerate() {
                    assert_eq!(
                        bv, rv,
                        "st mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options: first 1000 bars via SIMD, rest via batch_indicator
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_simd_by_options_state_continuity() {
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
            let (high, low, close) = get_arrays(stock_data);

            let first_inputs = [
                &high[..FIRST_CHUNK],
                &low[..FIRST_CHUNK],
                &close[..FIRST_CHUNK],
            ];
            let (simd_first, mut states) =
                indicator_by_options::<4>(&first_inputs, &options_4, None)
                    .expect("SIMD by-options Supertrend failed on first chunk");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let mut batch_st = simd_first[opt_idx][0].clone();

                let mut high_chunks = high[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut low_chunks = low[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);
                let mut close_chunks = close[FIRST_CHUNK..].chunks_exact(CHUNK_SIZE);

                for ((hc, lc), cc) in high_chunks
                    .by_ref()
                    .zip(low_chunks.by_ref())
                    .zip(close_chunks.by_ref())
                {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[hc, lc, cc], None)
                        .expect("batch_indicator failed");
                    batch_st.extend_from_slice(&chunk_outputs[0]);
                }

                let high_rem = high_chunks.remainder();
                let low_rem = low_chunks.remainder();
                let close_rem = close_chunks.remainder();
                if !high_rem.is_empty() {
                    let chunk_outputs = states[opt_idx]
                        .batch_indicator(&[high_rem, low_rem, close_rem], None)
                        .expect("batch_indicator failed on remainder");
                    batch_st.extend_from_slice(&chunk_outputs[0]);
                }

                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) = SuperTrend::indicator(&scalar_inputs, options, None)
                    .expect("Scalar Supertrend failed");

                assert_eq!(
                    batch_st.len(),
                    scalar_outputs[0].len(),
                    "st length mismatch: stock={stock_symbol}, options={options:?}"
                );
                for (i, (&bv, &rv)) in batch_st.iter().zip(scalar_outputs[0].iter()).enumerate() {
                    assert_eq!(
                        bv, rv,
                        "st mismatch at index {i}: simd+batch={bv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                    );
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-assets optional outputs: atr, tr, medprice must match scalar
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_simd_by_assets_optional_outputs() {
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
                indicator_by_assets::<4>(&inputs_4, &options, Some(&[true, true, true]))
                    .expect("SIMD by-assets Supertrend with optional outputs failed");

            for (asset_idx, (stock_symbol, high, low, close)) in stock_data.iter().enumerate() {
                let scalar_inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
                let (scalar_outputs, _) =
                    SuperTrend::indicator(&scalar_inputs, &options, Some(&[true, true, true]))
                        .expect("Scalar Supertrend with optional outputs failed");

                // Check all 4 outputs: st[0], atr[1], tr[2], medprice[3]
                for out_idx in 0..4 {
                    let simd_out = &simd_results[asset_idx][out_idx];
                    let scalar_out = &scalar_outputs[out_idx];
                    let out_name = ["st", "atr", "tr", "medprice"][out_idx];

                    assert_eq!(
                        simd_out.len(),
                        scalar_out.len(),
                        "{out_name} length mismatch: stock={stock_symbol}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out.iter()).enumerate() {
                        assert_eq!(
                            sv, rv,
                            "{out_name} mismatch at index {i}: simd={sv}, scalar={rv}, stock={stock_symbol}, options={options:?}"
                        );
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // SIMD by-options optional outputs: atr, tr, medprice must match scalar
    // -------------------------------------------------------------------------

    #[test]
    fn test_supertrend_simd_by_options_optional_outputs() {
        init_database_data();
        let data = get_all_stock_data().unwrap();

        let options_4 = [
            &OPTIONS_LIST[0],
            &OPTIONS_LIST[1],
            &OPTIONS_LIST[2],
            &OPTIONS_LIST[3],
        ];

        for (_stock_symbol, stock_data) in data.iter().take(4) {
            let (high, low, close) = get_arrays(stock_data);
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];

            let (simd_results, _) =
                indicator_by_options::<4>(&inputs, &options_4, Some(&[true, true, true]))
                    .expect("SIMD by-options Supertrend with optional outputs failed");

            for (opt_idx, options) in OPTIONS_LIST.iter().enumerate() {
                let (scalar_outputs, _) =
                    SuperTrend::indicator(&inputs, options, Some(&[true, true, true]))
                        .expect("Scalar Supertrend with optional outputs failed");

                // Check all 4 outputs: st[0], atr[1], tr[2], medprice[3]
                for out_idx in 0..4 {
                    let simd_out = &simd_results[opt_idx][out_idx];
                    let scalar_out = &scalar_outputs[out_idx];
                    let out_name = ["st", "atr", "tr", "medprice"][out_idx];

                    assert_eq!(
                        simd_out.len(),
                        scalar_out.len(),
                        "{out_name} length mismatch: opt_idx={opt_idx}, options={options:?}"
                    );
                    for (i, (&sv, &rv)) in simd_out.iter().zip(scalar_out.iter()).enumerate() {
                        assert_eq!(
                            sv, rv,
                            "{out_name} mismatch at index {i}: simd={sv}, scalar={rv}, opt_idx={opt_idx}, options={options:?}"
                        );
                    }
                }
            }
        }
    }
}
