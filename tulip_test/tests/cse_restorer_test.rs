use tulip_rs::cse::{restorer_for, restorer_for_with_options, ChainState, IndicatorStateBox};

/// A `Node::RollSum` tail slot **as the decoder actually hands it**:
/// `ChainState::RollSumRing`. These fixtures used to build `ChainState::Sma`, a
/// variant no `RollSum` node ever decodes to — so they were testing a fiction,
/// and they kept passing while the real request path declined the engine's own
/// carries (C15 found eight of those restorers). Any fixture that changes shape
/// here must be justified by what `carries_to_chains` returns, not by what the
/// restorer used to accept.
fn roll(sum: f64, period: u32) -> ChainState {
    ChainState::RollSumRing {
        sum,
        // The ring is sized to `period`, because that is what a FULL group
        // exports and what the restorers now check before they will build a
        // buffer from it (`trima` refuses a ring whose length differs from its
        // capacity — and it is right to: a wrong-length ring would rebuild a
        // plausible-looking buffer of the wrong window). A zero-filled ring is
        // fine here because these fixtures assert the VARIANT mapping, not the
        // numbers; the values are gated by the continuation probes in
        // `cse_state_capture_test.rs`.
        ring: vec![0.0; period as usize],
        cursor: 0,
        period,
    }
}

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
        let got = restore(vec![], &[], &[]);
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
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Trix(_)));

    let qstick = restorer_for("qstick").expect("missing restorer for qstick");
    let open = [1.0, 2.0, 3.0, 4.0];
    let close = [1.1, 2.1, 3.1, 4.1];
    let got = qstick(vec![roll(0.5, 4)], &[&open, &close], &[]);
    assert!(matches!(got, IndicatorStateBox::Qstick(_)));

    let zlema = restorer_for("zlema").expect("missing restorer for zlema");
    let zlema_tail = [10.0, 11.0, 12.0, 13.0, 14.0];
    let got = zlema(
        vec![ChainState::Ema {
            value: 9.25,
            period: 11,
        }],
        &[&zlema_tail],
        &[],
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
        &[],
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
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Emv(_)));

    let obv = restorer_for("obv").expect("missing restorer for obv");
    let obv_close = [100.0, 101.0, 103.0];
    let got = obv(vec![ChainState::Mirror { value: 42.0 }], &[&obv_close], &[]);
    assert!(matches!(got, IndicatorStateBox::Obv(_)));

    let wad = restorer_for("wad").expect("missing restorer for wad");
    let wad_high = [101.0, 102.0, 103.0];
    let wad_low = [99.0, 100.0, 101.0];
    let wad_close = [100.0, 101.0, 102.0];
    let got = wad(
        vec![ChainState::Mirror { value: 12.5 }],
        &[&wad_high, &wad_low, &wad_close],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Wad(_)));

    let vwap = restorer_for("vwap").expect("missing restorer for vwap");
    let got = vwap(
        vec![
            ChainState::Mirror { value: 1_250.0 },
            ChainState::Mirror { value: 125.0 },
        ],
        &[],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Vwap(_)));

    let vwma = restorer_for("vwma").expect("missing restorer for vwma");
    let vwma_close = [100.0, 101.0, 102.0];
    let vwma_volume = [10.0, 12.0, 14.0];
    let got = vwma(
        vec![roll(3_430.0, 3), roll(36.0, 3)],
        &[&vwma_close, &vwma_volume],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Vwma(_)));

    let vosc = restorer_for("vosc").expect("missing restorer for vosc");
    let vosc_volume = [10.0, 11.0, 12.0, 13.0];
    let got = vosc(vec![roll(24.0, 2), roll(46.0, 4)], &[&vosc_volume], &[]);
    assert!(matches!(got, IndicatorStateBox::Vosc(_)));

    let chaikinmf = restorer_for("chaikinmf").expect("missing restorer for chaikinmf");
    let cmf_high = [11.0, 12.0, 13.0];
    let cmf_low = [10.0, 11.0, 12.0];
    let cmf_close = [10.5, 11.5, 12.5];
    let cmf_volume = [100.0, 120.0, 130.0];
    let got = chaikinmf(
        vec![roll(1.0, 3), roll(350.0, 3)],
        &[&cmf_high, &cmf_low, &cmf_close, &cmf_volume],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Chaikinmf(_)));

    let mfi = restorer_for("mfi").expect("missing restorer for mfi");
    // Five bars for `period = 3`: the restorer builds each money-flow pair from
    // `tp[i]` versus `tp[i-1]` and needs the pair for the seam bar itself, so it
    // reads `period + 2` bars. Fewer than that is a request this state cannot be
    // assembled for, and it must refuse rather than invent the first pair.
    let mfi_high = [11.0, 12.0, 13.0, 13.5, 14.0];
    let mfi_low = [10.0, 11.0, 12.0, 12.5, 13.0];
    let mfi_close = [10.5, 11.5, 12.5, 13.0, 13.5];
    let mfi_volume = [100.0, 120.0, 130.0, 140.0, 150.0];
    let got = mfi(
        vec![
            ChainState::RollAge {
                ring: vec![11.5],
                cursor: 0,
                n: 1,
            },
            roll(2_500.0, 3),
            roll(1_500.0, 3),
        ],
        &[&mfi_high, &mfi_low, &mfi_close, &mfi_volume],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Mfi(_)));

    let vortex = restorer_for("vortex").expect("missing restorer for vortex");
    let vx_high = [11.0, 12.0, 13.0, 14.0];
    let vx_low = [10.0, 11.0, 12.0, 13.0];
    let vx_close = [10.5, 11.5, 12.5, 13.5];
    let got = vortex(
        vec![roll(3.0, 3), roll(3.0, 3), roll(4.5, 3)],
        &[&vx_high, &vx_low, &vx_close],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Vortex(_)));

    let trima = restorer_for("trima").expect("missing restorer for trima");
    let trima_real = [100.0, 101.0, 102.0, 103.0, 104.0];
    let got = trima(vec![roll(306.0, 3), roll(306.0, 3)], &[&trima_real], &[]);
    assert!(matches!(got, IndicatorStateBox::Trima(_)));

    let roc = restorer_for("roc").expect("missing restorer for roc");
    let roc_tail = [100.0, 101.0, 103.0, 102.0];
    let got = roc(vec![], &[&roc_tail], &[]);
    assert!(matches!(got, IndicatorStateBox::Roc(_)));

    let rocr = restorer_for("rocr").expect("missing restorer for rocr");
    let rocr_tail = [100.0, 101.0, 103.0, 102.0];
    let got = rocr(vec![], &[&rocr_tail], &[]);
    assert!(matches!(got, IndicatorStateBox::Rocr(_)));

    let real_long: Vec<f64> = (1..=80).map(|x| x as f64).collect();

    let adaptivemsw = restorer_for("adaptivemsw").expect("missing restorer for adaptivemsw");
    // Became CARRIED 2026-11-17: the empty chain list used to be accepted because
    // this restorer replayed `init_state` over the window, which is the fallback
    // that must now refuse. Its HD accumulator arrives as the 38-cell bridge; the
    // 50-bar price ring is rebuilt from the window, and the window alone is no
    // longer enough to answer at all.
    let got = adaptivemsw(
        vec![ChainState::HdPipeline {
            acc: vec![0.0; 38],
            extras: [0.0; 3],
            st: 0.0,
        }],
        &[real_long.as_slice()],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::AdaptiveMsw(_)));

    let cybercycle = restorer_for("cybercycle").expect("missing restorer for cybercycle");
    // The cybercycle family became CARRIED 2026-11-17. The `Mirror`-shaped alpha
    // these three fixtures used to pass is exactly the spelling that must now
    // REFUSE — it was reading the option out of the data and silently fell back to
    // 0.07 for any other request — so each fixture hands over the carried slots in
    // its recipe's `tail_inputs` order plus the options the emitter used.
    let got = cybercycle(
        vec![ChainState::CyberCycle {
            cycle_prev: 1.5,
            cycle_prev2: -0.5,
        }],
        &[real_long.as_slice()],
        &[0.07],
    );
    assert!(matches!(got, IndicatorStateBox::Cybercycle(_)));

    let instantaneoustrendline = restorer_for("instantaneoustrendline")
        .expect("missing restorer for instantaneoustrendline");
    // ITL is CARRIED (2026-11-17): the empty chain list that used to be accepted
    // here is exactly the shape that must now REFUSE, so the fixture hands over
    // the cell `it_step` maintains — 38 HD cells plus the `[IT[t], IT[t-1], 0.0]`
    // kvacc triple.
    let got = instantaneoustrendline(
        vec![ChainState::HdPipeline {
            acc: vec![0.0; 38],
            extras: [3.0, 2.0, 0.0],
            st: 0.0,
        }],
        &[real_long.as_slice()],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Instantaneoustrendline(_)));

    let mama = restorer_for("mama").expect("missing restorer for mama");
    let got = mama(
        vec![ChainState::HdPipeline {
            acc: vec![0.0; 38],
            extras: [7.0, 5.0, 30.0],
            st: 0.25,
        }],
        &[real_long.as_slice()],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Mama(_)));

    let high: Vec<f64> = (1..=80).map(|x| x as f64 + 1.0).collect();
    let low: Vec<f64> = (1..=80).map(|x| x as f64 - 1.0).collect();

    let psar = restorer_for("psar").expect("missing restorer for psar");
    // PSAR became CARRIED 2026-11-17: the two `Mirror`s it used to accept were the
    // option pair read out of the data, which is the spelling that must now refuse.
    // What replaces them is the emitter's own latch state plus the options.
    let got = psar(
        vec![ChainState::Psar {
            psar: 10.5,
            extream: 12.0,
            accel: 0.04,
            uptrend: true,
        }],
        &[high.as_slice(), low.as_slice()],
        &[0.02, 0.2],
    );
    assert!(matches!(got, IndicatorStateBox::Psar(_)));

    let trendmode = restorer_for("trendmode").expect("missing restorer for trendmode");
    let got = trendmode(
        vec![
            ChainState::CyberCycle {
                cycle_prev: 1.5,
                cycle_prev2: -0.5,
            },
            ChainState::Mirror { value: 2.25 },
        ],
        &[real_long.as_slice()],
        &[0.07],
    );
    assert!(matches!(got, IndicatorStateBox::Trendmode(_)));

    let ccfisher = restorer_for("ccfisher").expect("missing restorer for ccfisher");
    let got = ccfisher(
        vec![
            ChainState::CyberCycle {
                cycle_prev: 1.5,
                cycle_prev2: -0.5,
            },
            ChainState::Mirror { value: 2.25 },
            ChainState::IirClamped { value: 0.4 },
            ChainState::Mirror { value: 0.1 },
        ],
        &[real_long.as_slice()],
        &[0.07],
    );
    assert!(matches!(got, IndicatorStateBox::Ccfisher(_)));

    let homodynediscriminator =
        restorer_for("homodynediscriminator").expect("missing restorer for homodynediscriminator");
    // The HD pipeline is CARRIED now: the restorer takes the 38-cell the kernel
    // keeps (`hd_acc[k]`) and refuses without it, because re-warming these IIR
    // stages from a window yields a plausible state that resumes wrong.
    // `index == 1` with `vals[(N - a) % N]` is how a newest-first shift register
    // maps into the hand's rings; scalars at 32..38 are the IIR/period states.
    let mut hd_acc = vec![0.0_f64; 38];
    for (a, v) in [12.0, 11.0, 10.0, 9.0].iter().enumerate() {
        hd_acc[a] = *v;
    }
    for a in 0..7 {
        hd_acc[4 + a] = 100.0 + a as f64;
        hd_acc[11 + a] = 100.0 + a as f64;
        hd_acc[18 + a] = a as f64;
        hd_acc[25 + a] = a as f64;
    }
    hd_acc[32] = 0.5;
    hd_acc[33] = -0.5;
    hd_acc[36] = 20.0;
    hd_acc[37] = 20.0;
    let got = homodynediscriminator(
        vec![ChainState::HdPipeline {
            acc: hd_acc,
            extras: [0.0; 3],
            st: 0.0,
        }],
        &[real_long.as_slice()],
        &[],
    );
    assert!(matches!(got, IndicatorStateBox::Homodynediscriminator(_)));

    let msw = restorer_for("msw").expect("missing restorer for msw");
    let msw_tail = [100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 106.0, 107.0];
    // `msw` now needs BOTH its carried SDFT pair and the period from options: an
    // empty chain list is the shape that used to be accepted, and accepting it was
    // the bug (the window DFT is not the accumulator — 295/300 wrong when resumed
    // that way, see `engine_carry_msw_continues_bit_exact`).
    let got = msw(
        vec![ChainState::SdftRotate {
            rp: 1.5,
            ip: -0.5,
            lead_sine: 0.25,
        }],
        &[&msw_tail],
        &[7.0],
    );
    assert!(matches!(got, IndicatorStateBox::Msw(_)));

    let linreg = restorer_for("linreg").expect("missing restorer for linreg");
    let linreg_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let got = linreg(vec![], &[&linreg_tail], &[]);
    assert!(matches!(got, IndicatorStateBox::Linreg(_)));

    let tsf = restorer_for("tsf").expect("missing restorer for tsf");
    let tsf_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    let got = tsf(vec![], &[&tsf_tail], &[]);
    assert!(matches!(got, IndicatorStateBox::Tsf(_)));

    let fosc = restorer_for("fosc").expect("missing restorer for fosc");
    let fosc_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let got = fosc(vec![], &[&fosc_tail], &[]);
    assert!(matches!(got, IndicatorStateBox::Fosc(_)));

    let kama = restorer_for("kama").expect("missing restorer for kama");
    let kama_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let got = kama(vec![], &[&kama_tail], &[]);
    assert!(matches!(got, IndicatorStateBox::Kama(_)));

    let keltnerchannel =
        restorer_for("keltnerchannel").expect("missing restorer for keltnerchannel");
    let kc_high = [11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let kc_low = [9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
    let kc_close = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    // Became CARRIED 2026-11-17: its state is the ATR and EMA chains, so the empty
    // chain list that used to be replayed over the window must now refuse.
    let got = keltnerchannel(
        vec![
            ChainState::Wilders {
                value: 1.5,
                period: 5,
            },
            ChainState::Ema {
                value: 12.5,
                period: 5,
            },
        ],
        &[&kc_high, &kc_low, &kc_close],
        &[5.0, 2.0],
    );
    assert!(matches!(got, IndicatorStateBox::Keltnerchannel(_)));

    let smaenvelope = restorer_for("smaenvelope").expect("missing restorer for smaenvelope");
    let se_real = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
    // The percentage is the request's second option now — a restorer that used to
    // hard-code 1.0 and take the period from the window length.
    let got = smaenvelope(
        vec![ChainState::Sma {
            sum: 65.0,
            period: 5,
        }],
        &[&se_real],
        &[5.0, 1.0],
    );
    assert!(matches!(got, IndicatorStateBox::Smaenvelope(_)));

    let vidya = restorer_for("vidya").expect("missing restorer for vidya");
    let vidya_tail = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
    let got = vidya(
        vec![
            ChainState::Stddev {
                sum: 60.0,
                sum_sq: 730.0,
                period: 5,
            },
            ChainState::Stddev {
                sum: 108.0,
                sum_sq: 1496.0,
                period: 8,
            },
            ChainState::Mirror { value: 13.5 },
        ],
        &[&vidya_tail],
        &[5.0, 8.0, 0.8],
    );
    assert!(matches!(got, IndicatorStateBox::Vidya(_)));
}

#[test]
fn restorer_for_with_options_honors_non_default_options() {
    let real_long: Vec<f64> = (1..=100).map(|x| x as f64).collect();

    // The cybercycle family reads its non-default option through the OPTIONS
    // vector now rather than through a replay-table closure that captured it, so
    // these three go via `restorer_for` — and the assertion is the same one the
    // old replay arms made: a request for α = 0.2 must not come back with the
    // 0.07 default the deleted `unwrap_or` used to hand out.
    let cybercycle = restorer_for("cybercycle").expect("missing restorer for cybercycle");
    let got = cybercycle(
        vec![ChainState::CyberCycle {
            cycle_prev: 1.5,
            cycle_prev2: -0.5,
        }],
        &[real_long.as_slice()],
        &[0.2],
    );
    match got {
        IndicatorStateBox::Cybercycle(state) => {
            let (_coef, d1, _d2) = tulip_rs::indicators::cybercycle::multiplier(0.2);
            assert!((state.d1 - d1).abs() < 1e-12);
        }
        _ => panic!("wrong variant for option-aware cybercycle"),
    }

    let trendmode = restorer_for("trendmode").expect("missing restorer for trendmode");
    let got = trendmode(
        vec![
            ChainState::CyberCycle {
                cycle_prev: 1.5,
                cycle_prev2: -0.5,
            },
            ChainState::Mirror { value: 2.25 },
        ],
        &[real_long.as_slice()],
        &[0.2],
    );
    match got {
        IndicatorStateBox::Trendmode(state) => assert!((state.alpha - 0.2).abs() < 1e-12),
        _ => panic!("wrong variant for option-aware trendmode"),
    }

    let ccfisher = restorer_for("ccfisher").expect("missing restorer for ccfisher");
    let got = ccfisher(
        vec![
            ChainState::CyberCycle {
                cycle_prev: 1.5,
                cycle_prev2: -0.5,
            },
            ChainState::Mirror { value: 2.25 },
            ChainState::IirClamped { value: 0.4 },
            ChainState::Mirror { value: 0.1 },
        ],
        &[real_long.as_slice()],
        &[0.2],
    );
    match got {
        IndicatorStateBox::Ccfisher(state) => assert!((state.alpha - 0.2).abs() < 1e-12),
        _ => panic!("wrong variant for option-aware ccfisher"),
    }

    // `mama` is no longer a replay-table kind: it reads its limits from the
    // options vector through `restorer_for`'s trait arm and everything else from
    // the carried cell, so this asserts BOTH halves — the options must not be
    // inferred from the chains any more, and the carries must not be re-warmed.
    let mama = restorer_for("mama").expect("missing restorer for mama");
    let got = mama(
        vec![ChainState::HdPipeline {
            acc: vec![0.0; 38],
            extras: [7.0, 5.0, 30.0],
            st: 0.25,
        }],
        &[real_long.as_slice()],
        &[0.8, 0.1],
    );
    match got {
        IndicatorStateBox::Mama(state) => {
            assert!((state.fast_limit - 0.8).abs() < 1e-12);
            assert!((state.slow_limit - 0.1).abs() < 1e-12);
            assert_eq!(state.mama, 7.0);
            assert_eq!(state.fama, 5.0);
            assert_eq!(state.prev_phase, 30.0);
            assert_eq!(state.alpha, 0.25);
        }
        _ => panic!("wrong variant for option-aware mama"),
    }

    let kc_high: Vec<f64> = (1..=80).map(|x| x as f64 + 1.0).collect();
    let kc_low: Vec<f64> = (1..=80).map(|x| x as f64 - 1.0).collect();
    let kc_close: Vec<f64> = (1..=80).map(|x| x as f64).collect();

    // All three migrated today, so they now resolve through `restorer_for`'s trait
    // arm: the parameters come from OPTIONS and the state comes from the chains.
    let keltner = restorer_for("keltnerchannel").expect("missing restorer for keltnerchannel");
    let got = keltner(
        vec![
            ChainState::Wilders {
                value: 1.5,
                period: 20,
            },
            ChainState::Ema {
                value: 40.0,
                period: 20,
            },
        ],
        &[kc_high.as_slice(), kc_low.as_slice(), kc_close.as_slice()],
        &[20.0, 3.5],
    );
    match got {
        IndicatorStateBox::Keltnerchannel(state) => assert!((state.step - 3.5).abs() < 1e-12),
        _ => panic!("wrong variant for option-aware keltnerchannel"),
    }

    // Invalid option payloads should fail fast (proves option-aware path is used,
    // not fallback defaults). `psar` left the replay table when its latches became
    // carried, so this control now runs through the trait arm: a VALID carry with
    // an INVALID option pair (max_af <= af_step) must still be declined.
    let psar = restorer_for("psar").expect("missing restorer for psar");
    let psar_high = [11.0, 12.0, 13.0];
    let psar_low = [9.0, 10.0, 11.0];
    let got = psar(
        vec![ChainState::Psar {
            psar: 10.5,
            extream: 12.0,
            accel: 0.04,
            uptrend: true,
        }],
        &[&psar_high, &psar_low],
        &[0.2, 0.1],
    );
    assert!(matches!(got, IndicatorStateBox::Other));

    let smaenvelope = restorer_for("smaenvelope").expect("missing restorer for smaenvelope");
    let se_real = [
        10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0, 21.0, 22.0, 23.0, 24.0,
        25.0, 26.0, 27.0, 28.0, 29.0, 30.0,
    ];
    let se_chain = || {
        vec![ChainState::Sma {
            sum: 420.0,
            period: 20,
        }]
    };
    // The percentage travels from the options now (it used to be hard-coded 1.0),
    // and a carry whose period contradicts the request is REFUSED rather than
    // re-parameterised — a control the replay arms could not express, because they
    // ignored the chain list entirely.
    let got = smaenvelope(se_chain(), &[&se_real], &[20.0, 3.25]);
    assert!(matches!(got, IndicatorStateBox::Smaenvelope(_)));
    let got = smaenvelope(se_chain(), &[&se_real], &[10.0, 3.25]);
    assert!(matches!(got, IndicatorStateBox::Other));

    let vidya = restorer_for("vidya").expect("missing restorer for vidya");
    let vd_real: Vec<f64> = (1..=30).map(|x| x as f64).collect();
    let vd_chains = || {
        vec![
            ChainState::Stddev {
                sum: 100.0,
                sum_sq: 1000.0,
                period: 10,
            },
            ChainState::Stddev {
                sum: 420.0,
                sum_sq: 9100.0,
                period: 20,
            },
            ChainState::Mirror { value: 21.0 },
        ]
    };
    // `alpha` must sit in (0,1) exactly as the hand's own validator demands — the
    // restorer calls it, so an illegal request is refused rather than answered with
    // a state the next `indicator()` call would reject. The second call keeps a
    // LEGAL alpha and contradicts only the long period against the chains.
    let got = vidya(vd_chains(), &[vd_real.as_slice()], &[10.0, 20.0, 0.8]);
    assert!(matches!(got, IndicatorStateBox::Vidya(_)));
    let got = vidya(vd_chains(), &[vd_real.as_slice()], &[10.0, 25.0, 0.8]);
    assert!(matches!(got, IndicatorStateBox::Other));
    let got = vidya(vd_chains(), &[vd_real.as_slice()], &[10.0, 20.0, 1.2]);
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
    let got = restore(vec![], &[], &[]);
    assert!(matches!(got, IndicatorStateBox::Other));
}
