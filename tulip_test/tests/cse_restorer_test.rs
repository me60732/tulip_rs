use tulip_rs::cse::{restorer_for, restorer_for_with_options, ChainState, IndicatorStateBox};

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

    let chaikinmf = restorer_for("chaikinmf").expect("missing restorer for chaikinmf");
    let cmf_high = [11.0, 12.0, 13.0];
    let cmf_low = [10.0, 11.0, 12.0];
    let cmf_close = [10.5, 11.5, 12.5];
    let cmf_volume = [100.0, 120.0, 130.0];
    let got = chaikinmf(
        vec![
            ChainState::Sma {
                sum: 1.0,
                period: 3,
            },
            ChainState::Sma {
                sum: 350.0,
                period: 3,
            },
        ],
        &[&cmf_high, &cmf_low, &cmf_close, &cmf_volume],
    );
    assert!(matches!(got, IndicatorStateBox::Chaikinmf(_)));

    let mfi = restorer_for("mfi").expect("missing restorer for mfi");
    let mfi_high = [11.0, 12.0, 13.0];
    let mfi_low = [10.0, 11.0, 12.0];
    let mfi_close = [10.5, 11.5, 12.5];
    let mfi_volume = [100.0, 120.0, 130.0];
    let got = mfi(
        vec![
            ChainState::RollAge {
                ring: vec![11.5],
                cursor: 0,
                n: 1,
            },
            ChainState::Sma {
                sum: 2_500.0,
                period: 3,
            },
            ChainState::Sma {
                sum: 1_500.0,
                period: 3,
            },
        ],
        &[&mfi_high, &mfi_low, &mfi_close, &mfi_volume],
    );
    assert!(matches!(got, IndicatorStateBox::Mfi(_)));

    let vortex = restorer_for("vortex").expect("missing restorer for vortex");
    let vx_high = [11.0, 12.0, 13.0, 14.0];
    let vx_low = [10.0, 11.0, 12.0, 13.0];
    let vx_close = [10.5, 11.5, 12.5, 13.5];
    let got = vortex(
        vec![
            ChainState::Sma {
                sum: 3.0,
                period: 3,
            },
            ChainState::Sma {
                sum: 3.0,
                period: 3,
            },
            ChainState::Sma {
                sum: 4.5,
                period: 3,
            },
        ],
        &[&vx_high, &vx_low, &vx_close],
    );
    assert!(matches!(got, IndicatorStateBox::Vortex(_)));

    let trima = restorer_for("trima").expect("missing restorer for trima");
    let trima_real = [100.0, 101.0, 102.0, 103.0, 104.0];
    let got = trima(
        vec![
            ChainState::Sma {
                sum: 306.0,
                period: 3,
            },
            ChainState::Sma {
                sum: 306.0,
                period: 3,
            },
        ],
        &[&trima_real],
    );
    assert!(matches!(got, IndicatorStateBox::Trima(_)));

    let roc = restorer_for("roc").expect("missing restorer for roc");
    let roc_tail = [100.0, 101.0, 103.0, 102.0];
    let got = roc(vec![], &[&roc_tail]);
    assert!(matches!(got, IndicatorStateBox::Roc(_)));

    let rocr = restorer_for("rocr").expect("missing restorer for rocr");
    let rocr_tail = [100.0, 101.0, 103.0, 102.0];
    let got = rocr(vec![], &[&rocr_tail]);
    assert!(matches!(got, IndicatorStateBox::Rocr(_)));

    let real_long: Vec<f64> = (1..=80).map(|x| x as f64).collect();

    let adaptivemsw = restorer_for("adaptivemsw").expect("missing restorer for adaptivemsw");
    let got = adaptivemsw(vec![], &[real_long.as_slice()]);
    assert!(matches!(got, IndicatorStateBox::AdaptiveMsw(_)));

    let cybercycle = restorer_for("cybercycle").expect("missing restorer for cybercycle");
    let got = cybercycle(
        vec![ChainState::Mirror { value: 0.07 }],
        &[real_long.as_slice()],
    );
    assert!(matches!(got, IndicatorStateBox::Cybercycle(_)));

    let instantaneoustrendline = restorer_for("instantaneoustrendline")
        .expect("missing restorer for instantaneoustrendline");
    let got = instantaneoustrendline(vec![], &[real_long.as_slice()]);
    assert!(matches!(got, IndicatorStateBox::Instantaneoustrendline(_)));

    let mama = restorer_for("mama").expect("missing restorer for mama");
    let got = mama(
        vec![
            ChainState::Mirror { value: 0.5 },
            ChainState::Mirror { value: 0.05 },
        ],
        &[real_long.as_slice()],
    );
    assert!(matches!(got, IndicatorStateBox::Mama(_)));

    let high: Vec<f64> = (1..=80).map(|x| x as f64 + 1.0).collect();
    let low: Vec<f64> = (1..=80).map(|x| x as f64 - 1.0).collect();

    let psar = restorer_for("psar").expect("missing restorer for psar");
    let got = psar(
        vec![
            ChainState::Mirror { value: 0.02 },
            ChainState::Mirror { value: 0.2 },
        ],
        &[high.as_slice(), low.as_slice()],
    );
    assert!(matches!(got, IndicatorStateBox::Psar(_)));

    let trendmode = restorer_for("trendmode").expect("missing restorer for trendmode");
    let got = trendmode(
        vec![ChainState::Mirror { value: 0.07 }],
        &[real_long.as_slice()],
    );
    assert!(matches!(got, IndicatorStateBox::Trendmode(_)));

    let ccfisher = restorer_for("ccfisher").expect("missing restorer for ccfisher");
    let got = ccfisher(
        vec![ChainState::Mirror { value: 0.07 }],
        &[real_long.as_slice()],
    );
    assert!(matches!(got, IndicatorStateBox::Ccfisher(_)));

    let homodynediscriminator =
        restorer_for("homodynediscriminator").expect("missing restorer for homodynediscriminator");
    let got = homodynediscriminator(vec![], &[real_long.as_slice()]);
    assert!(matches!(got, IndicatorStateBox::Homodynediscriminator(_)));

    let msw = restorer_for("msw").expect("missing restorer for msw");
    let msw_tail = [100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0];
    let got = msw(vec![], &[&msw_tail]);
    assert!(matches!(got, IndicatorStateBox::Msw(_)));

    let linreg = restorer_for("linreg").expect("missing restorer for linreg");
    let linreg_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let got = linreg(vec![], &[&linreg_tail]);
    assert!(matches!(got, IndicatorStateBox::Linreg(_)));

    let tsf = restorer_for("tsf").expect("missing restorer for tsf");
    let tsf_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let got = tsf(vec![], &[&tsf_tail]);
    assert!(matches!(got, IndicatorStateBox::Tsf(_)));

    let fosc = restorer_for("fosc").expect("missing restorer for fosc");
    let fosc_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let got = fosc(vec![], &[&fosc_tail]);
    assert!(matches!(got, IndicatorStateBox::Fosc(_)));

    let kama = restorer_for("kama").expect("missing restorer for kama");
    let kama_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let got = kama(vec![], &[&kama_tail]);
    assert!(matches!(got, IndicatorStateBox::Kama(_)));

    let keltnerchannel =
        restorer_for("keltnerchannel").expect("missing restorer for keltnerchannel");
    let kc_high = [11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let kc_low = [9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
    let kc_close = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let got = keltnerchannel(vec![], &[&kc_high, &kc_low, &kc_close]);
    assert!(matches!(got, IndicatorStateBox::Keltnerchannel(_)));

    let smaenvelope = restorer_for("smaenvelope").expect("missing restorer for smaenvelope");
    let se_real = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let got = smaenvelope(
        vec![ChainState::Sma {
            sum: 65.0,
            period: 5,
        }],
        &[&se_real],
    );
    assert!(matches!(got, IndicatorStateBox::Smaenvelope(_)));

    let vidya = restorer_for("vidya").expect("missing restorer for vidya");
    let vidya_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
    let got = vidya(vec![], &[&vidya_tail]);
    assert!(matches!(got, IndicatorStateBox::Vidya(_)));
}

#[test]
fn restorer_for_with_options_honors_non_default_options() {
    let real_long: Vec<f64> = (1..=100).map(|x| x as f64).collect();

    let cybercycle =
        restorer_for_with_options("cybercycle", &[0.2]).expect("missing option-aware cybercycle");
    let got = cybercycle(vec![], &[real_long.as_slice()]);
    match got {
        IndicatorStateBox::Cybercycle(state) => {
            let (_coef, d1, _d2) = tulip_rs::indicators::cybercycle::multiplier(0.2);
            assert!((state.d1 - d1).abs() < 1e-12);
        }
        _ => panic!("wrong variant for option-aware cybercycle"),
    }

    let trendmode =
        restorer_for_with_options("trendmode", &[0.2]).expect("missing option-aware trendmode");
    let got = trendmode(vec![], &[real_long.as_slice()]);
    match got {
        IndicatorStateBox::Trendmode(state) => assert!((state.alpha - 0.2).abs() < 1e-12),
        _ => panic!("wrong variant for option-aware trendmode"),
    }

    let ccfisher =
        restorer_for_with_options("ccfisher", &[0.2]).expect("missing option-aware ccfisher");
    let got = ccfisher(vec![], &[real_long.as_slice()]);
    match got {
        IndicatorStateBox::Ccfisher(state) => assert!((state.alpha - 0.2).abs() < 1e-12),
        _ => panic!("wrong variant for option-aware ccfisher"),
    }

    let mama = restorer_for_with_options("mama", &[0.8, 0.1]).expect("missing option-aware mama");
    let got = mama(vec![], &[real_long.as_slice()]);
    match got {
        IndicatorStateBox::Mama(state) => {
            assert!((state.fast_limit - 0.8).abs() < 1e-12);
            assert!((state.slow_limit - 0.1).abs() < 1e-12);
        }
        _ => panic!("wrong variant for option-aware mama"),
    }

    let kc_high: Vec<f64> = (1..=80).map(|x| x as f64 + 1.0).collect();
    let kc_low: Vec<f64> = (1..=80).map(|x| x as f64 - 1.0).collect();
    let kc_close: Vec<f64> = (1..=80).map(|x| x as f64).collect();

    let keltner = restorer_for_with_options("keltnerchannel", &[20.0, 3.5])
        .expect("missing option-aware keltnerchannel");
    let got = keltner(
        vec![],
        &[kc_high.as_slice(), kc_low.as_slice(), kc_close.as_slice()],
    );
    match got {
        IndicatorStateBox::Keltnerchannel(state) => assert!((state.step - 3.5).abs() < 1e-12),
        _ => panic!("wrong variant for option-aware keltnerchannel"),
    }

    // Invalid option payloads should fail fast (proves option-aware path is used,
    // not fallback defaults).
    let psar = restorer_for_with_options("psar", &[0.2, 0.1]).expect("missing option-aware psar");
    let psar_high = [11.0, 12.0, 13.0];
    let psar_low = [9.0, 10.0, 11.0];
    let got = psar(vec![], &[&psar_high, &psar_low]);
    assert!(matches!(got, IndicatorStateBox::Other));

    let smaenvelope = restorer_for_with_options("smaenvelope", &[20.0, 0.0])
        .expect("missing option-aware smaenvelope");
    let se_real = [
        10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0,
        25.0, 26.0, 27.0, 28.0, 29.0, 30.0,
    ];
    let got = smaenvelope(vec![], &[&se_real]);
    assert!(matches!(got, IndicatorStateBox::Other));

    let vidya =
        restorer_for_with_options("vidya", &[10.0, 20.0, 1.2]).expect("missing option-aware vidya");
    let vd_real: Vec<f64> = (1..=30).map(|x| x as f64).collect();
    let got = vidya(vec![], &[vd_real.as_slice()]);
    assert!(matches!(got, IndicatorStateBox::Other));
}

#[test]
fn restorer_for_unknown_kind_is_none() {
    assert!(restorer_for("definitely_not_a_kind").is_none());
    assert!(restorer_for_with_options("definitely_not_a_kind", &[]).is_none());
}

#[test]
fn restorer_for_existing_kind_returns_other_on_bad_payload() {
    let restore = restorer_for("ema").expect("ema restorer present");
    let got = restore(vec![], &[]);
    assert!(matches!(got, IndicatorStateBox::Other));
}
