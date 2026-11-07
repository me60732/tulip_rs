use tulip_rs::cse::{restorer_for, ChainState, IndicatorStateBox};

#[test]
fn restorer_for_maps_new_stateless_variants() {
    let cases = [
        ("avgprice", "avgprice"),
        ("typprice", "typprice"),
        ("wcprice", "wcprice"),
        ("bop", "bop"),
        ("marketfi", "marketfi"),
    ];

    for (kind, label) in cases {
        let restore = restorer_for(kind).unwrap_or_else(|| panic!("missing restorer for {label}"));
        let got = restore(vec![], &[]);
        match (kind, got) {
            ("avgprice", IndicatorStateBox::AvgPrice(_)) => {}
            ("typprice", IndicatorStateBox::Typprice(_)) => {}
            ("wcprice", IndicatorStateBox::WcPrice(_)) => {}
            ("bop", IndicatorStateBox::Bop(_)) => {}
            ("marketfi", IndicatorStateBox::Marketfi(_)) => {}
            (_, other) => panic!(
                "{label}: wrong state box variant: {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }
}

#[test]
fn restorer_for_maps_new_stateful_variants() {
    let dema = restorer_for("dema").expect("missing restorer for dema");
    let got = dema(
        vec![
            ChainState::Ema {
                value: 1.25,
                period: 10,
            },
            ChainState::Ema {
                value: 1.10,
                period: 10,
            },
        ],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Dema(_)));

    let tema = restorer_for("tema").expect("missing restorer for tema");
    let got = tema(
        vec![
            ChainState::Ema {
                value: 1.25,
                period: 10,
            },
            ChainState::Ema {
                value: 1.10,
                period: 10,
            },
            ChainState::Ema {
                value: 1.05,
                period: 10,
            },
        ],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Tema(_)));

    let trix = restorer_for("trix").expect("missing restorer for trix");
    let got = trix(
        vec![
            ChainState::Ema {
                value: 1.25,
                period: 10,
            },
            ChainState::Ema {
                value: 1.10,
                period: 10,
            },
            ChainState::Ema {
                value: 1.05,
                period: 10,
            },
        ],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Trix(_)));

    let qstick = restorer_for("qstick").expect("missing restorer for qstick");
    let open = [1.0, 2.0, 3.0, 4.0];
    let close = [1.1, 2.1, 3.1, 4.1];
    let got = qstick(
        vec![ChainState::Sma {
            sum: 0.5,
            period: 4,
        }],
        &[&open, &close],
    );
    assert!(matches!(got, IndicatorStateBox::Qstick(_)));

    let zlema = restorer_for("zlema").expect("missing restorer for zlema");
    let zlema_tail = [10.0, 11.0, 12.0, 13.0, 14.0];
    let got = zlema(
        vec![ChainState::Ema {
            value: 9.25,
            period: 11,
        }],
        &[&zlema_tail],
    );
    assert!(matches!(got, IndicatorStateBox::Zlema(_)));

    let highpass = restorer_for("highpass").expect("missing restorer for highpass");
    let got = highpass(
        vec![ChainState::HighPass {
            y1: 0.75,
            prev_real: 42.0,
            period: 10,
        }],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Highpass(_)));

    let supersmoother = restorer_for("supersmoother").expect("missing restorer for supersmoother");
    let got = supersmoother(
        vec![ChainState::SuperSmoother {
            y1: 0.5,
            y2: 0.25,
            prev_real: 41.0,
            period: 12,
        }],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Supersmoother(_)));

    let roofingfilter = restorer_for("roofingfilter").expect("missing restorer for roofingfilter");
    let got = roofingfilter(
        vec![
            ChainState::SuperSmoother {
                y1: 0.5,
                y2: 0.25,
                prev_real: 41.0,
                period: 10,
            },
            ChainState::HighPass {
                y1: 0.75,
                prev_real: 42.0,
                period: 48,
            },
        ],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Roofingfilter(_)));

    let dpo = restorer_for("dpo").expect("missing restorer for dpo");
    let dpo_tail = [100.0, 101.0, 103.0, 102.0, 104.0, 105.0];
    let got = dpo(
        vec![ChainState::Sma {
            sum: 615.0,
            period: 6,
        }],
        &[&dpo_tail],
    );
    assert!(matches!(got, IndicatorStateBox::Dpo(_)));

    let emv = restorer_for("emv").expect("missing restorer for emv");
    let emv_high = [10.0, 11.0, 12.0];
    let emv_low = [9.0, 10.0, 11.0];
    let got = emv(
        vec![ChainState::RollAge {
            ring: vec![10.5],
            cursor: 0,
            n: 1,
        }],
        &[&emv_high, &emv_low],
    );
    assert!(matches!(got, IndicatorStateBox::Emv(_)));

    let obv = restorer_for("obv").expect("missing restorer for obv");
    let obv_close = [100.0, 101.0, 103.0];
    let got = obv(vec![ChainState::Mirror { value: 42.0 }], &[&obv_close]);
    assert!(matches!(got, IndicatorStateBox::Obv(_)));

    let wad = restorer_for("wad").expect("missing restorer for wad");
    let wad_high = [101.0, 102.0, 103.0];
    let wad_low = [99.0, 100.0, 101.0];
    let wad_close = [100.0, 101.0, 102.0];
    let got = wad(
        vec![ChainState::Mirror { value: 12.5 }],
        &[&wad_high, &wad_low, &wad_close],
    );
    assert!(matches!(got, IndicatorStateBox::Wad(_)));

    let vwap = restorer_for("vwap").expect("missing restorer for vwap");
    let got = vwap(
        vec![
            ChainState::Mirror { value: 1_250.0 },
            ChainState::Mirror { value: 125.0 },
        ],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Vwap(_)));

    let vwma = restorer_for("vwma").expect("missing restorer for vwma");
    let vwma_close = [100.0, 101.0, 102.0];
    let vwma_volume = [10.0, 12.0, 14.0];
    let got = vwma(
        vec![
            ChainState::Sma {
                sum: 3_430.0,
                period: 3,
            },
            ChainState::Sma {
                sum: 36.0,
                period: 3,
            },
        ],
        &[&vwma_close, &vwma_volume],
    );
    assert!(matches!(got, IndicatorStateBox::Vwma(_)));

    let vosc = restorer_for("vosc").expect("missing restorer for vosc");
    let vosc_volume = [10.0, 11.0, 12.0, 13.0];
    let got = vosc(
        vec![
            ChainState::Sma {
                sum: 24.0,
                period: 2,
            },
            ChainState::Sma {
                sum: 46.0,
                period: 4,
            },
        ],
        &[&vosc_volume],
    );
    assert!(matches!(got, IndicatorStateBox::Vosc(_)));

    let roc = restorer_for("roc").expect("missing restorer for roc");
    let roc_tail = [100.0, 101.0, 103.0, 102.0];
    let got = roc(vec![], &[&roc_tail]);
    assert!(matches!(got, IndicatorStateBox::Roc(_)));

    let rocr = restorer_for("rocr").expect("missing restorer for rocr");
    let rocr_tail = [100.0, 101.0, 103.0, 102.0];
    let got = rocr(vec![], &[&rocr_tail]);
    assert!(matches!(got, IndicatorStateBox::Rocr(_)));
}

#[test]
fn restorer_for_unknown_kind_is_none() {
    assert!(restorer_for("definitely_not_a_kind").is_none());
}

#[test]
fn restorer_for_existing_kind_returns_other_on_bad_payload() {
    let restore = restorer_for("ema").expect("ema restorer present");
    let got = restore(vec![], &[]);
    assert!(matches!(got, IndicatorStateBox::Other));
}
