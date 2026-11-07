use tulip_rs::cse::{
    restorer_for_with_options, run_registered, IndicatorStateBox, Planable, Request,
};
use tulip_rs::indicator_types::{Indicator, TIndicatorState};
use tulip_rs::indicators::ccfisher::CcFisher;
use tulip_rs::indicators::cybercycle::Cybercycle;
use tulip_rs::indicators::keltnerchannel::KeltnerChannel;
use tulip_rs::indicators::mama::Mama;
use tulip_rs::indicators::psar::Psar;
use tulip_rs::indicators::smaenvelope::SmaEnvelope;
use tulip_rs::indicators::trendmode::TrendMode;
use tulip_rs::indicators::vidya::Vidya;
use tulip_test::database::{get_all_stock_data, init_database_data, EodData};

const CHUNK_SIZE: usize = 100;

fn get_close_array(stock_data: &[EodData]) -> Vec<f64> {
    stock_data.iter().map(|d| d.close).collect()
}

fn get_hl_arrays(stock_data: &[EodData]) -> (Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    (high, low)
}

fn get_hlc_arrays(stock_data: &[EodData]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = stock_data.iter().map(|d| d.high).collect();
    let low: Vec<f64> = stock_data.iter().map(|d| d.low).collect();
    let close: Vec<f64> = stock_data.iter().map(|d| d.close).collect();
    (high, low, close)
}

fn cse_rows_for<const I: usize, const O: usize, T>(
    inputs: &[&[f64]; I],
    options: &[f64; O],
    label: &str,
) -> Vec<Vec<f64>>
where
    T: Indicator<I, O> + Planable,
{
    let req = Request::new().add(T::recipe(options), None);
    let out = run_registered(&req, inputs)
        .unwrap_or_else(|| panic!("{label}: registry miss for standalone request"));

    let primary_count = T::INFO.outputs.len();
    let mut rows = vec![Vec::new(); primary_count];
    for (row, owner) in out.rows.iter().zip(out.owners.iter()) {
        if owner.req == 0 && owner.primary {
            rows[owner.idx] = row.to_vec();
        }
    }
    rows
}

fn extend_rows(base: &mut [Vec<f64>], chunk: Vec<Vec<f64>>, label: &str) {
    assert_eq!(
        base.len(),
        chunk.len(),
        "{label}: row count mismatch while extending"
    );
    for (dst, src) in base.iter_mut().zip(chunk.into_iter()) {
        dst.extend_from_slice(&src);
    }
}

fn assert_outputs_equal(label: &str, stock_symbol: &str, expected: &[Vec<f64>], got: &[Vec<f64>]) {
    assert_eq!(
        expected.len(),
        got.len(),
        "{label}: output row count mismatch for {stock_symbol}"
    );

    for (row_idx, (want_row, got_row)) in expected.iter().zip(got.iter()).enumerate() {
        assert_eq!(
            want_row.len(),
            got_row.len(),
            "{label}: row {row_idx} length mismatch for {stock_symbol}: expected={}, got={}",
            want_row.len(),
            got_row.len()
        );

        for (i, (&want, &have)) in want_row.iter().zip(got_row.iter()).enumerate() {
            assert_eq!(
                want, have,
                "{label}: row {row_idx} mismatch at index {i} for {stock_symbol}: expected={want}, got={have}"
            );
        }
    }
}

fn extend_with_state_chunks_1<S: TIndicatorState<1>>(
    state: &mut S,
    input: &[f64],
    start: usize,
    primary_count: usize,
    combined: &mut [Vec<f64>],
    label: &str,
) {
    let mut chunks = input[start..].chunks_exact(CHUNK_SIZE);
    for chunk in chunks.by_ref() {
        let out = state
            .batch_indicator(&[chunk], None)
            .unwrap_or_else(|e| panic!("{label}: batch_indicator failed on chunk: {e:?}"))
            .into_iter()
            .take(primary_count)
            .collect();
        extend_rows(combined, out, label);
    }

    let rem = chunks.remainder();
    if !rem.is_empty() {
        let out = state
            .batch_indicator(&[rem], None)
            .unwrap_or_else(|e| panic!("{label}: batch_indicator failed on remainder: {e:?}"))
            .into_iter()
            .take(primary_count)
            .collect();
        extend_rows(combined, out, label);
    }
}

fn extend_with_state_chunks_2<S: TIndicatorState<2>>(
    state: &mut S,
    input0: &[f64],
    input1: &[f64],
    start: usize,
    primary_count: usize,
    combined: &mut [Vec<f64>],
    label: &str,
) {
    let mut c0 = input0[start..].chunks_exact(CHUNK_SIZE);
    let mut c1 = input1[start..].chunks_exact(CHUNK_SIZE);

    for (x0, x1) in c0.by_ref().zip(c1.by_ref()) {
        let out = state
            .batch_indicator(&[x0, x1], None)
            .unwrap_or_else(|e| panic!("{label}: batch_indicator failed on chunk: {e:?}"))
            .into_iter()
            .take(primary_count)
            .collect();
        extend_rows(combined, out, label);
    }

    let r0 = c0.remainder();
    let r1 = c1.remainder();
    if !r0.is_empty() {
        let out = state
            .batch_indicator(&[r0, r1], None)
            .unwrap_or_else(|e| panic!("{label}: batch_indicator failed on remainder: {e:?}"))
            .into_iter()
            .take(primary_count)
            .collect();
        extend_rows(combined, out, label);
    }
}

fn extend_with_state_chunks_3<S: TIndicatorState<3>>(
    state: &mut S,
    input0: &[f64],
    input1: &[f64],
    input2: &[f64],
    start: usize,
    primary_count: usize,
    combined: &mut [Vec<f64>],
    label: &str,
) {
    let mut c0 = input0[start..].chunks_exact(CHUNK_SIZE);
    let mut c1 = input1[start..].chunks_exact(CHUNK_SIZE);
    let mut c2 = input2[start..].chunks_exact(CHUNK_SIZE);

    for ((x0, x1), x2) in c0.by_ref().zip(c1.by_ref()).zip(c2.by_ref()) {
        let out = state
            .batch_indicator(&[x0, x1, x2], None)
            .unwrap_or_else(|e| panic!("{label}: batch_indicator failed on chunk: {e:?}"))
            .into_iter()
            .take(primary_count)
            .collect();
        extend_rows(combined, out, label);
    }

    let r0 = c0.remainder();
    let r1 = c1.remainder();
    let r2 = c2.remainder();
    if !r0.is_empty() {
        let out = state
            .batch_indicator(&[r0, r1, r2], None)
            .unwrap_or_else(|e| panic!("{label}: batch_indicator failed on remainder: {e:?}"))
            .into_iter()
            .take(primary_count)
            .collect();
        extend_rows(combined, out, label);
    }
}

#[test]
fn cse_state_continuity_database_close_indicators() {
    init_database_data();
    let data = get_all_stock_data().expect("database unavailable");

    const CYBER_OPTIONS: [[f64; 1]; 2] = [[0.07], [0.2]];
    const TREND_OPTIONS: [[f64; 1]; 2] = [[0.07], [0.2]];
    const CCFISHER_OPTIONS: [[f64; 1]; 2] = [[0.07], [0.2]];
    const MAMA_OPTIONS: [[f64; 2]; 2] = [[0.5, 0.05], [0.8, 0.1]];
    const SMAENV_OPTIONS: [[f64; 2]; 2] = [[20.0, 1.0], [10.0, 2.0]];
    const VIDYA_OPTIONS: [[f64; 3]; 2] = [[5.0, 20.0, 0.2], [10.0, 30.0, 0.4]];

    for (stock_symbol, stock_data) in data {
        let close = get_close_array(&stock_data);

        for options in CYBER_OPTIONS {
            let label = format!("cybercycle options={options:?}");
            let inputs = [close.as_slice()];
            let (full_outputs, _) =
                Cybercycle::indicator(&inputs, &options, None).expect("cybercycle full run failed");
            let full_outputs: Vec<Vec<f64>> = full_outputs
                .into_iter()
                .take(Cybercycle::INFO.outputs.len())
                .collect();

            let first_len = Cybercycle::min_data(&options).max(CHUNK_SIZE);
            if close.len() <= first_len {
                let cse_full = cse_rows_for::<1, 1, Cybercycle>(&inputs, &options, &label);
                assert_outputs_equal(&label, &stock_symbol, &full_outputs, &cse_full);
                continue;
            }

            let first_inputs = [&close[..first_len]];
            let mut combined = cse_rows_for::<1, 1, Cybercycle>(&first_inputs, &options, &label);

            let restore = restorer_for_with_options("cybercycle", &options)
                .expect("missing option-aware restorer for cybercycle");
            let mut state = match restore(vec![], &first_inputs) {
                IndicatorStateBox::Cybercycle(state) => state,
                _ => panic!("{label}: wrong state variant from restorer"),
            };

            extend_with_state_chunks_1(
                &mut state,
                close.as_slice(),
                first_len,
                Cybercycle::INFO.outputs.len(),
                &mut combined,
                &label,
            );
            assert_outputs_equal(&label, &stock_symbol, &full_outputs, &combined);
        }

        for options in TREND_OPTIONS {
            let label = format!("trendmode options={options:?}");
            let inputs = [close.as_slice()];
            let (full_outputs, _) =
                TrendMode::indicator(&inputs, &options, None).expect("trendmode full run failed");
            let full_outputs: Vec<Vec<f64>> = full_outputs
                .into_iter()
                .take(TrendMode::INFO.outputs.len())
                .collect();

            let first_len = TrendMode::min_data(&options).max(CHUNK_SIZE);
            if close.len() <= first_len {
                let cse_full = cse_rows_for::<1, 1, TrendMode>(&inputs, &options, &label);
                assert_outputs_equal(&label, &stock_symbol, &full_outputs, &cse_full);
                continue;
            }

            let first_inputs = [&close[..first_len]];
            let mut combined = cse_rows_for::<1, 1, TrendMode>(&first_inputs, &options, &label);

            let restore = restorer_for_with_options("trendmode", &options)
                .expect("missing option-aware restorer for trendmode");
            let mut state = match restore(vec![], &first_inputs) {
                IndicatorStateBox::Trendmode(state) => state,
                _ => panic!("{label}: wrong state variant from restorer"),
            };

            extend_with_state_chunks_1(
                &mut state,
                close.as_slice(),
                first_len,
                TrendMode::INFO.outputs.len(),
                &mut combined,
                &label,
            );
            assert_outputs_equal(&label, &stock_symbol, &full_outputs, &combined);
        }

        for options in CCFISHER_OPTIONS {
            let label = format!("ccfisher options={options:?}");
            let inputs = [close.as_slice()];
            let (full_outputs, _) =
                CcFisher::indicator(&inputs, &options, None).expect("ccfisher full run failed");
            let full_outputs: Vec<Vec<f64>> = full_outputs
                .into_iter()
                .take(CcFisher::INFO.outputs.len())
                .collect();

            let first_len = CcFisher::min_data(&options).max(CHUNK_SIZE);
            if close.len() <= first_len {
                let cse_full = cse_rows_for::<1, 1, CcFisher>(&inputs, &options, &label);
                assert_outputs_equal(&label, &stock_symbol, &full_outputs, &cse_full);
                continue;
            }

            let first_inputs = [&close[..first_len]];
            let mut combined = cse_rows_for::<1, 1, CcFisher>(&first_inputs, &options, &label);

            let restore = restorer_for_with_options("ccfisher", &options)
                .expect("missing option-aware restorer for ccfisher");
            let mut state = match restore(vec![], &first_inputs) {
                IndicatorStateBox::Ccfisher(state) => state,
                _ => panic!("{label}: wrong state variant from restorer"),
            };

            extend_with_state_chunks_1(
                &mut state,
                close.as_slice(),
                first_len,
                CcFisher::INFO.outputs.len(),
                &mut combined,
                &label,
            );
            assert_outputs_equal(&label, &stock_symbol, &full_outputs, &combined);
        }

        for options in MAMA_OPTIONS {
            let label = format!("mama options={options:?}");
            let inputs = [close.as_slice()];
            let (full_outputs, _) =
                Mama::indicator(&inputs, &options, None).expect("mama full run failed");
            let full_outputs: Vec<Vec<f64>> = full_outputs
                .into_iter()
                .take(Mama::INFO.outputs.len())
                .collect();

            let first_len = Mama::min_data(&options).max(CHUNK_SIZE);
            if close.len() <= first_len {
                let cse_full = cse_rows_for::<1, 2, Mama>(&inputs, &options, &label);
                assert_outputs_equal(&label, &stock_symbol, &full_outputs, &cse_full);
                continue;
            }

            let first_inputs = [&close[..first_len]];
            let mut combined = cse_rows_for::<1, 2, Mama>(&first_inputs, &options, &label);

            let restore = restorer_for_with_options("mama", &options)
                .expect("missing option-aware restorer for mama");
            let mut state = match restore(vec![], &first_inputs) {
                IndicatorStateBox::Mama(state) => state,
                _ => panic!("{label}: wrong state variant from restorer"),
            };

            extend_with_state_chunks_1(
                &mut state,
                close.as_slice(),
                first_len,
                Mama::INFO.outputs.len(),
                &mut combined,
                &label,
            );
            assert_outputs_equal(&label, &stock_symbol, &full_outputs, &combined);
        }

        for options in SMAENV_OPTIONS {
            let label = format!("smaenvelope options={options:?}");
            let inputs = [close.as_slice()];
            let (full_outputs, _) = SmaEnvelope::indicator(&inputs, &options, None)
                .expect("smaenvelope full run failed");
            let full_outputs: Vec<Vec<f64>> = full_outputs
                .into_iter()
                .take(SmaEnvelope::INFO.outputs.len())
                .collect();

            let first_len = SmaEnvelope::min_data(&options).max(CHUNK_SIZE);
            if close.len() <= first_len {
                let cse_full = cse_rows_for::<1, 2, SmaEnvelope>(&inputs, &options, &label);
                assert_outputs_equal(&label, &stock_symbol, &full_outputs, &cse_full);
                continue;
            }

            let first_inputs = [&close[..first_len]];
            let mut combined = cse_rows_for::<1, 2, SmaEnvelope>(&first_inputs, &options, &label);

            let restore = restorer_for_with_options("smaenvelope", &options)
                .expect("missing option-aware restorer for smaenvelope");
            let mut state = match restore(vec![], &first_inputs) {
                IndicatorStateBox::Smaenvelope(state) => state,
                _ => panic!("{label}: wrong state variant from restorer"),
            };

            extend_with_state_chunks_1(
                &mut state,
                close.as_slice(),
                first_len,
                SmaEnvelope::INFO.outputs.len(),
                &mut combined,
                &label,
            );
            assert_outputs_equal(&label, &stock_symbol, &full_outputs, &combined);
        }

        for options in VIDYA_OPTIONS {
            let label = format!("vidya options={options:?}");
            let inputs = [close.as_slice()];
            let (full_outputs, _) =
                Vidya::indicator(&inputs, &options, None).expect("vidya full run failed");
            let full_outputs: Vec<Vec<f64>> = full_outputs
                .into_iter()
                .take(Vidya::INFO.outputs.len())
                .collect();

            let first_len = Vidya::min_data(&options).max(CHUNK_SIZE);
            if close.len() <= first_len {
                let cse_full = cse_rows_for::<1, 3, Vidya>(&inputs, &options, &label);
                assert_outputs_equal(&label, &stock_symbol, &full_outputs, &cse_full);
                continue;
            }

            let first_inputs = [&close[..first_len]];
            let mut combined = cse_rows_for::<1, 3, Vidya>(&first_inputs, &options, &label);

            let restore = restorer_for_with_options("vidya", &options)
                .expect("missing option-aware restorer for vidya");
            let mut state = match restore(vec![], &first_inputs) {
                IndicatorStateBox::Vidya(state) => state,
                _ => panic!("{label}: wrong state variant from restorer"),
            };

            extend_with_state_chunks_1(
                &mut state,
                close.as_slice(),
                first_len,
                Vidya::INFO.outputs.len(),
                &mut combined,
                &label,
            );
            assert_outputs_equal(&label, &stock_symbol, &full_outputs, &combined);
        }
    }
}

#[test]
fn cse_state_continuity_database_psar() {
    init_database_data();
    let data = get_all_stock_data().expect("database unavailable");

    const PSAR_OPTIONS: [[f64; 2]; 2] = [[0.02, 0.2], [0.03, 0.3]];

    for (stock_symbol, stock_data) in data {
        let (high, low) = get_hl_arrays(&stock_data);

        for options in PSAR_OPTIONS {
            let label = format!("psar options={options:?}");
            let inputs = [high.as_slice(), low.as_slice()];
            let (full_outputs, _) =
                Psar::indicator(&inputs, &options, None).expect("psar full run failed");
            let full_outputs: Vec<Vec<f64>> = full_outputs
                .into_iter()
                .take(Psar::INFO.outputs.len())
                .collect();

            let first_len = Psar::min_data(&options).max(CHUNK_SIZE);
            if high.len() <= first_len {
                let cse_full = cse_rows_for::<2, 2, Psar>(&inputs, &options, &label);
                assert_outputs_equal(&label, &stock_symbol, &full_outputs, &cse_full);
                continue;
            }

            let first_inputs = [&high[..first_len], &low[..first_len]];
            let mut combined = cse_rows_for::<2, 2, Psar>(&first_inputs, &options, &label);

            let restore = restorer_for_with_options("psar", &options)
                .expect("missing option-aware restorer for psar");
            let mut state = match restore(vec![], &first_inputs) {
                IndicatorStateBox::Psar(state) => state,
                _ => panic!("{label}: wrong state variant from restorer"),
            };

            extend_with_state_chunks_2(
                &mut state,
                high.as_slice(),
                low.as_slice(),
                first_len,
                Psar::INFO.outputs.len(),
                &mut combined,
                &label,
            );
            assert_outputs_equal(&label, &stock_symbol, &full_outputs, &combined);
        }
    }
}

#[test]
fn cse_state_continuity_database_keltnerchannel() {
    init_database_data();
    let data = get_all_stock_data().expect("database unavailable");

    const KELTNER_OPTIONS: [[f64; 2]; 2] = [[20.0, 2.0], [10.0, 1.5]];

    for (stock_symbol, stock_data) in data {
        let (high, low, close) = get_hlc_arrays(&stock_data);

        for options in KELTNER_OPTIONS {
            let label = format!("keltnerchannel options={options:?}");
            let inputs = [high.as_slice(), low.as_slice(), close.as_slice()];
            let (full_outputs, _) = KeltnerChannel::indicator(&inputs, &options, None)
                .expect("keltnerchannel full run failed");
            let full_outputs: Vec<Vec<f64>> = full_outputs
                .into_iter()
                .take(KeltnerChannel::INFO.outputs.len())
                .collect();

            let first_len = KeltnerChannel::min_data(&options).max(CHUNK_SIZE);
            if close.len() <= first_len {
                let cse_full = cse_rows_for::<3, 2, KeltnerChannel>(&inputs, &options, &label);
                assert_outputs_equal(&label, &stock_symbol, &full_outputs, &cse_full);
                continue;
            }

            let first_inputs = [&high[..first_len], &low[..first_len], &close[..first_len]];
            let mut combined =
                cse_rows_for::<3, 2, KeltnerChannel>(&first_inputs, &options, &label);

            let restore = restorer_for_with_options("keltnerchannel", &options)
                .expect("missing option-aware restorer for keltnerchannel");
            let mut state = match restore(vec![], &first_inputs) {
                IndicatorStateBox::Keltnerchannel(state) => state,
                _ => panic!("{label}: wrong state variant from restorer"),
            };

            extend_with_state_chunks_3(
                &mut state,
                high.as_slice(),
                low.as_slice(),
                close.as_slice(),
                first_len,
                KeltnerChannel::INFO.outputs.len(),
                &mut combined,
                &label,
            );
            assert_outputs_equal(&label, &stock_symbol, &full_outputs, &combined);
        }
    }
}
