//! Macro-fused-kernel bench — hand vs generated, classic
//! sharing graph (macd+apo+ppo) + apo+ppo pair + ATR family + wide-pairing
//! proof shape, ONE binary (the layout rule: only same-run ratios are
//! quotable). Single asset, primary-only (the bench conventions:
//! `add(recipe, None)`, genuine cross-request sharing).
//!
//! The old Cranelift JIT rows retired with it to `../old_jit/` (2026-10);
//! its last measured numbers live in `proposals/Macro_Cse_Engine/ROADMAP.md`
//! and Handover §6.42, re-measurable by restoring the archive if ever needed.
//!
//! Naming convention — `<engine>_<scope>_<shape>`:
//!   engine   hand | generated (the `kernel!` macro's straight-line kernel;
//!            its warm `[0, gate)` prologue runs the const-generic `exec`
//!            inside every such call). Interpreter-as-engine rows retired
//!            2026-10-12 — nothing may run a whole series through `exec`;
//!            parity is gated against the hand.
//!   scope    runonly = plan resolved OUTSIDE the timed loop, the timed call
//!            is dispatch + kernel over the FULL series [0, n) — cold run
//!            every iteration (the warm phase is included; this is NOT
//!            resume — resume seeds chains from serialized state and starts
//!            mid-series, which M1 builds)
//!            endtoend = merge + erase + dispatch + kernel timed together,
//!            the honest "a fresh dashboard request arrives" cost
//!            planonly = merge + erase only (no execution) — the delta
//!            between runonly and endtoend
//!            scan (group `fused_assets`) = ONE plan, every timed iteration
//!            loops all 4 DB assets through it (≈ runonly × 4, plan once)
//!
//! Groups: `fused` (classic shapes), `fused_assets` (plan-reuse scans),
//! `fused_atr` (ATR/NATR/SuperTrend family), `fused_rolling` (MA pair
//! Sma+Wma, BBands, and the six-SMA scalar-vs-packed wide shape),
//! `fused_wide` (six distinct price-EMA chains: scalar vs
//! `Simd<f64,4>`+`Simd<f64,2>` packed twins — the twin fns are called
//! DIRECTLY because they share one Signature; `try_run` always reaches the
//! scalar first).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tulip_rs::cse::{erase_request, try_run, try_run_assets, Planable, Request};
use tulip_rs::indicator_types::{Indicator, IndicatorByOptions};
use tulip_rs::indicators::apo::Apo;
use tulip_rs::indicators::atr::Atr;
use tulip_rs::indicators::ema::Ema;
use tulip_rs::indicators::macd::Macd;
use tulip_rs::indicators::natr::Natr;
use tulip_rs::indicators::ppo::Ppo;
use tulip_rs::indicators::supertrend::SuperTrend;
use tulip_test::database::{
    get_all_stock_data, get_all_stock_data_8, init_database_data, init_database_data_8, EodData,
};

const APO: [f64; 2] = [12.0, 26.0];
const MACD: [f64; 3] = [12.0, 26.0, 9.0];
const PPO: [f64; 2] = [12.0, 26.0];

fn stocks() -> &'static Vec<(String, Vec<EodData>)> {
    init_database_data();
    get_all_stock_data().expect("stock data initialised")
}
fn col(sym: usize, f: impl Fn(&EodData) -> f64) -> Vec<f64> {
    stocks()[sym].1.iter().map(|d| f(d)).collect()
}
fn close(sym: usize) -> Vec<f64> {
    col(sym, |d| d.close)
}
fn hlc(sym: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    (
        col(sym, |d| d.high),
        col(sym, |d| d.low),
        col(sym, |d| d.close),
    )
}

fn apo_ppo_macd_shared_sl_req() -> Request {
    Request::new()
        .add(Apo::recipe(&APO), None)
        .add(Ppo::recipe(&PPO), None)
        .add(Macd::recipe(&MACD), None)
}
fn apo_ppo_shared_sl_req() -> Request {
    Request::new()
        .add(Apo::recipe(&APO), None)
        .add(Ppo::recipe(&PPO), None)
}
fn atr_natr_shared_p_req() -> Request {
    Request::new()
        .add(Atr::recipe(&[14.0]), None)
        .add(Natr::recipe(&[14.0]), None)
}
fn natr_supertrend_shared_p_req() -> Request {
    Request::new()
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None)
}
fn ema_six_req() -> Request {
    Request::new()
        .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
        .add(Apo::recipe(&[10.0, 20.0]), None)
        .add(Ppo::recipe(&[6.0, 14.0]), None)
}

fn bench_fused(c: &mut Criterion) {
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    // Plan resolved OUTSIDE every `runonly` loop — the kernel's per-request
    // cost lives in `*_planonly` and `*_endtoend` rows, honestly separated.
    let (_a, sig, vals, _o) = erase_request(&apo_ppo_macd_shared_sl_req()).unwrap();
    let mut g = c.benchmark_group("fused");
    g.sample_size(20);

    // BASELINE: three separate hand passes over [0, n), each with its own
    // rows + allocations (the same work the fused engines must beat).
    g.bench_function("hand_runonly_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let a = Apo::indicator(&inputs, &APO, None).unwrap();
            let p = Ppo::indicator(&inputs, &PPO, None).unwrap();
            let m = Macd::indicator(&inputs, &MACD, None).unwrap();
            black_box((a.0[0].len(), p.0[0].len(), m.0[0].len()))
        })
    });
    // GENERATED KERNEL, plan hoisted: `try_run` structural compare + kernel:
    // 1 arena malloc + warm exec + steady loop (4 deduped primary stores).
    g.bench_function("generated_runonly_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let rows = try_run(&sig, &vals, &inputs).expect("apo_ppo_macd template");
            // sink layout (primaries+optionals fixed list): line at 0, ppo at 3
            black_box(rows[0].len() + rows[3].len())
        })
    });
    // GENERATED, nothing hoisted: per iter = Request::new + 3 recipes +
    // merge (HashMap interning) + leads + erase (Signature/Values) +
    // dispatch + kernel. The honest fresh-request cost. Delta vs
    // `generated_runonly_*` ≈ the `*_planonly` row below.
    g.bench_function("generated_endtoend_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let out = tulip_rs::cse::run_registered(&apo_ppo_macd_shared_sl_req(), &inputs)
                .expect("apo_ppo_macd");
            black_box(out.rows[0].len() + out.rows[3].len())
        })
    });
    // Pair shapes (apo+ppo): 2 chains, 2 distinct sink nodes.
    g.bench_function("hand_runonly_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let a = Apo::indicator(&inputs, &APO, None).unwrap();
            let p = Ppo::indicator(&inputs, &PPO, None).unwrap();
            black_box((a.0[0].len(), p.0[0].len()))
        })
    });
    g.bench_function("generated_endtoend_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let out =
                tulip_rs::cse::run_registered(&apo_ppo_shared_sl_req(), &inputs).expect("apo_ppo");
            black_box(out.rows[0].len() + out.rows[3].len())
        })
    });

    // THE PLAN PHASE ALONE: merge + leads + erase, no execution. The
    // per-fresh-request tax endtoend pays and runonly hoists — and the
    // number M1's `Prepared` handle removes for streaming.
    g.bench_function("generated_planonly_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let (_a, sig, _v, _o) = erase_request(&apo_ppo_macd_shared_sl_req()).unwrap();
            black_box(sig.hash())
        })
    });

    // ── ring shapes, SINGLE asset (user gap 2026-10-13: every other pattern
    // has its runonly pair here; the ring shapes had only fused_assets8
    // rows). The lone shapes are DIAGNOSTIC denominators (hand's home turf
    // per the merging doctrine); the headline pair is the fused arena: the
    // engine's ONE pass vs the hand's TWO. ────────────────────────────────
    use tulip_rs::indicators::ao::Ao;
    use tulip_rs::indicators::fisher::Fisher;
    use tulip_rs::indicators::hma::Hma;
    let (h0, l0, c0) = hlc(0);
    let hl: [&[f64]; 2] = [h0.as_slice(), l0.as_slice()];
    let cl: [&[f64]; 1] = [c0.as_slice()];

    let (_ra, sig_ra, vals_ra, _oa) =
        erase_request(&Request::new().add(Ao::recipe(&[]), None)).unwrap();
    g.bench_function("generated_runonly_ao_s_m", |b| {
        b.iter(|| {
            let rows = try_run(&sig_ra, &vals_ra, &hl).expect("ao template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_ao_s_m", |b| {
        b.iter(|| {
            let a = Ao::indicator(&hl, &[], None).unwrap();
            black_box(a.0[0].len())
        })
    });

    let (_rh, sig_rh, vals_rh, _oh) =
        erase_request(&Request::new().add(Hma::recipe(&[9.0]), None)).unwrap();
    g.bench_function("generated_runonly_hma_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_rh, &vals_rh, &cl).expect("hma template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_hma_n", |b| {
        b.iter(|| {
            let h = Hma::indicator(&cl, &[9.0], None).unwrap();
            black_box(h.0[0].len())
        })
    });

    let (_rf, sig_rf, vals_rf, _of) =
        erase_request(&Request::new().add(Fisher::recipe(&[9.0]), None)).unwrap();
    g.bench_function("generated_runonly_fisher_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_rf, &vals_rf, &hl).expect("fisher template");
            black_box(rows[0].len() + rows[1].len())
        })
    });
    g.bench_function("hand_runonly_fisher_n", |b| {
        b.iter(|| {
            let f = Fisher::indicator(&hl, &[9.0], None).unwrap();
            black_box(f.0[0].len() + f.0[1].len())
        })
    });

    // THE fused single-asset pair: one medprice + one ring feed serve all
    // four rolls; the hand rival runs Ao and Fisher as TWO full passes.
    let (_raf, sig_raf, vals_raf, _oaf) = erase_request(
        &Request::new()
            .add(Ao::recipe(&[]), None)
            .add(Fisher::recipe(&[9.0]), None),
    )
    .unwrap();
    g.bench_function("generated_runonly_ao_fisher_shared_med", |b| {
        b.iter(|| {
            let rows = try_run(&sig_raf, &vals_raf, &hl).expect("ao+fisher template");
            black_box(rows.iter().map(|r| r.len()).sum::<usize>())
        })
    });
    g.bench_function("hand_runonly_ao_fisher_shared_med", |b| {
        b.iter(|| {
            let a = Ao::indicator(&hl, &[], None).unwrap();
            let f = Fisher::indicator(&hl, &[9.0], None).unwrap();
            black_box(a.0[0].len() + f.0[0].len() + f.0[1].len())
        })
    });

    // ── the shapes that landed after this group existed: ultosc, mass, ad,
    // adosc, mom. The doctrine is one single-asset `runonly` pair per newly
    // landed shape, and these are DIAGNOSTIC denominators — a lone indicator is
    // the hand's home turf, so parity or a small loss here is the floor, not a
    // regression to flag. The HEADLINE of this block is the last pair: mass
    // merged into the apo+ppo+macd hub, four indicators in ONE arena. ──────
    use tulip_rs::indicators::ad::Ad;
    use tulip_rs::indicators::adosc::Adosc;
    use tulip_rs::indicators::mass::Mass;
    use tulip_rs::indicators::mom::Mom;
    use tulip_rs::indicators::ultosc::Ultosc;
    let v0 = col(0, |d| d.volume);
    let hlc3: [&[f64]; 3] = [h0.as_slice(), l0.as_slice(), c0.as_slice()];
    let hlc4: [&[f64]; 4] = [h0.as_slice(), l0.as_slice(), c0.as_slice(), v0.as_slice()];

    let (_ru1, sig_run_ult, vals_run_ult, _ou1) =
        erase_request(&Request::new().add(Ultosc::recipe(&[7.0, 14.0, 28.0]), None)).unwrap();
    g.bench_function("generated_runonly_ultosc_s_m_l", |b| {
        b.iter(|| {
            let rows = try_run(&sig_run_ult, &vals_run_ult, &hlc3).expect("ultosc template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_ultosc_s_m_l", |b| {
        b.iter(|| {
            let u = Ultosc::indicator(&hlc3, &[7.0, 14.0, 28.0], None).unwrap();
            black_box(u.0[0].len())
        })
    });

    // ── THE DIRECTIONAL HUB (dm+di+dx+adx), one DM chain, four indicators ──
    // The first registered pattern that is a realistic dashboard combo AND the
    // first arena carrying both Wilders forms (di's raw-sum ATR + adx's mean ATR
    // over one interned `tr`). The hand rival is FOUR separate passes over the
    // same bars — `dm` on (h,l), the rest on (h,l,c) — so this is the pairing
    // the engine exists for, not a lone-indicator floor. The packed twin shares
    // the Signature, so `try_run` reaches the scalar and the twin is called
    // DIRECTLY (same-binary A/B, per the twin convention).
    use tulip_rs::cse::kernel::registry::dm_di_dx_adx_shared_p_p;
    use tulip_rs::indicators::adx::Adx;
    use tulip_rs::indicators::di::Di;
    use tulip_rs::indicators::dm::Dm;
    use tulip_rs::indicators::dx::Dx;

    let hub_req = Request::new()
        .add(Dm::recipe(&[14.0]), None)
        .add(Di::recipe(&[14.0]), None)
        .add(Dx::recipe(&[14.0]), None)
        .add(Adx::recipe(&[14.0]), None);
    let (_rh, sig_hub, vals_hub, _oh) = erase_request(&hub_req).unwrap();
    g.bench_function("generated_runonly_dm_di_dx_adx_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_hub, &vals_hub, &hlc3).expect("hub scalar template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("generated_runonly_dm_di_dx_adx_shared_p_p", |b| {
        b.iter(|| {
            let rows = dm_di_dx_adx_shared_p_p(&vals_hub, &hlc3);
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_dm_di_dx_adx_shared_p", |b| {
        b.iter(|| {
            let dm = Dm::indicator(&hl, &[14.0], None).unwrap();
            let di = Di::indicator(&hlc3, &[14.0], None).unwrap();
            let dx = Dx::indicator(&hlc3, &[14.0], None).unwrap();
            let adx = Adx::indicator(&hlc3, &[14.0], None).unwrap();
            black_box(dm.0[0].len() + di.0[0].len() + dx.0[0].len() + adx.0[0].len())
        })
    });

    // ── THE FULL DASHBOARD: seven indicators on ONE arena ──────────────────
    // The scaling question the four-indicator hub cannot answer: `dm`, `di`,
    // `adx`, `atr`, `natr` and `supertrend` ALL read the ONE mean-`Wilders(tr)`
    // ATR chain (gated: the seven-indicator arena holds the same four `Wilders`
    // slots as the four-indicator hub — three extra indicators, zero extra
    // accumulators). So `hub7 / hub` measures how the win grows with CONSUMERS OF
    // ONE CHAIN, engine-to-engine, which is the reliable direction on this box.
    // The hand rival is SEVEN separate passes, each recomputing the TR/Wilders
    // work it needs. Primary-only on both sides; every line is a DISTINCT request,
    // so this is not the own-optional-output artifact.
    let hub7_req = Request::new()
        .add(Dm::recipe(&[14.0]), None)
        .add(Di::recipe(&[14.0]), None)
        .add(Dx::recipe(&[14.0]), None)
        .add(Adx::recipe(&[14.0]), None)
        .add(Atr::recipe(&[14.0]), None)
        .add(Natr::recipe(&[14.0]), None)
        .add(SuperTrend::recipe(&[14.0, 3.0]), None);
    let (_r7, sig_hub7, vals_hub7, _o7) = erase_request(&hub7_req).unwrap();
    g.bench_function(
        "generated_runonly_dm_di_dx_adx_atr_natr_supertrend_shared_p",
        |b| {
            b.iter(|| {
                let rows = try_run(&sig_hub7, &vals_hub7, &hlc3).expect("mega-hub template");
                black_box(rows[0].len())
            })
        },
    );
    g.bench_function(
        "hand_runonly_dm_di_dx_adx_atr_natr_supertrend_shared_p",
        |b| {
            b.iter(|| {
                let dm = Dm::indicator(&hl, &[14.0], None).unwrap();
                let di = Di::indicator(&hlc3, &[14.0], None).unwrap();
                let dx = Dx::indicator(&hlc3, &[14.0], None).unwrap();
                let adx = Adx::indicator(&hlc3, &[14.0], None).unwrap();
                let atr = Atr::indicator(&hlc3, &[14.0], None).unwrap();
                let natr = Natr::indicator(&hlc3, &[14.0], None).unwrap();
                let st = SuperTrend::indicator(&hlc3, &[14.0, 3.0], None).unwrap();
                black_box(
                    dm.0[0].len()
                        + di.0[0].len()
                        + dx.0[0].len()
                        + adx.0[0].len()
                        + atr.0[0].len()
                        + natr.0[0].len()
                        + st.0[0].len(),
                )
            })
        },
    );

    // ── THE LONE-SHAPE FLOOR — the denominator the hub's win is read against ──
    // Each hub member registered and timed ALONE against the same single hand
    // call. These are DIAGNOSTICS, not shipping options (user 2026-10-16: "we can
    // do this for testing, just don't expect it to win, but it shouldn't be too
    // far behind"). Without them the hub's ratio has no context: a lone shape is
    // the hand's home turf, so the only honest reading of a merged win is
    // `sum(lone engine) vs hub`, and `lone engine vs lone hand` tells us how much
    // scalar-axis tax the engine pays before any dedup is counted.
    // NOT to be confused with the `bench_floor` GROUP (`floor_max_only`), which is
    // the MACHINE floor — the cost of the scan itself with no indicator. This is
    // the LONE-SHAPE floor: the same indicator, unmerged.
    // Every row here is gated to actually ROUTE a generated kernel by
    // `lone_dm_di_dx_adx_arenas_match_their_registered_signatures` — an unmatched
    // arena would fall back to the hand and time the hand against itself.
    let (_r1, sig_dm_n, vals_dm_n, _o1) =
        erase_request(&Request::new().add(Dm::recipe(&[14.0]), None)).unwrap();
    g.bench_function("generated_runonly_dm_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_dm_n, &vals_dm_n, &hl).expect("dm_n template");
            black_box(rows[0].len() + rows[1].len())
        })
    });
    g.bench_function("hand_runonly_dm_n", |b| {
        b.iter(|| {
            let d = Dm::indicator(&hl, &[14.0], None).unwrap();
            black_box(d.0[0].len() + d.0[1].len())
        })
    });

    let (_r2, sig_di_n, vals_di_n, _o2) =
        erase_request(&Request::new().add(Di::recipe(&[14.0]), None)).unwrap();
    g.bench_function("generated_runonly_di_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_di_n, &vals_di_n, &hlc3).expect("di_n template");
            black_box(rows[0].len() + rows[1].len())
        })
    });
    g.bench_function("hand_runonly_di_n", |b| {
        b.iter(|| {
            let d = Di::indicator(&hlc3, &[14.0], None).unwrap();
            black_box(d.0[0].len() + d.0[1].len())
        })
    });

    let (_r3, sig_dx_n, vals_dx_n, _o3) =
        erase_request(&Request::new().add(Dx::recipe(&[14.0]), None)).unwrap();
    g.bench_function("generated_runonly_dx_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_dx_n, &vals_dx_n, &hlc3).expect("dx_n template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_dx_n", |b| {
        b.iter(|| {
            let d = Dx::indicator(&hlc3, &[14.0], None).unwrap();
            black_box(d.0[0].len())
        })
    });

    let (_r4, sig_adx_n, vals_adx_n, _o4) =
        erase_request(&Request::new().add(Adx::recipe(&[14.0]), None)).unwrap();
    g.bench_function("generated_runonly_adx_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_adx_n, &vals_adx_n, &hlc3).expect("adx_n template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_adx_n", |b| {
        b.iter(|| {
            let a = Adx::indicator(&hlc3, &[14.0], None).unwrap();
            black_box(a.0[0].len())
        })
    });

    // THE ATR RUNG — the ladder step that makes `di_n` decomposable. DI's arena is
    // (the whole dm chain) + (the mean `Wilders(tr)` ATR chain) + two Mul/Max glue
    // nodes, so `di_n − dm_n − atr_n` is the glue's measured cost. Without this row
    // the di-vs-adx inversion (DI has THREE Wilders chains, ADX has FOUR, yet DI is
    // the slower shape) has no evidence behind it and only a story. Per
    // `fusion-win-decays-with-live-xmm-register-pressure`: ladder the plan first,
    // then compare engine-to-engine — never infer a slow plan from a ratio.
    let (_r5, sig_atr_n, vals_atr_n, _o5) =
        erase_request(&Request::new().add(tulip_rs::indicators::atr::Atr::recipe(&[14.0]), None))
            .unwrap();
    g.bench_function("generated_runonly_atr_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_atr_n, &vals_atr_n, &hlc3).expect("atr_n template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_atr_n", |b| {
        b.iter(|| {
            let a = tulip_rs::indicators::atr::Atr::indicator(&hlc3, &[14.0], None).unwrap();
            black_box(a.0[0].len())
        })
    });

    // THE ATR ABLATION RUNGS, benched in the SAME binary as `atr_n` above so the
    // three numbers share one session (the hybrid-CPU rule: never compare a ratio
    // across runs). `atr` = the tr subgraph + one mean-`Wilders` over it, so
    //     atr_n − tr_n − wilders_n
    // is the measured cost of fusing the two. ANSWERED (one binary, medians): the
    // smoothing is at near-parity -- `wilders_n` 5.029 vs hand 4.748 = 1.06x behind
    // -- so atr's 1.59x deficit is the TR WALK: `tr_n` 1.904 vs hand 1.291 standalone,
    // and INSIDE atr the engine's tr costs 7.668-5.029 = 2.64 us where the hand's costs
    // 4.826-4.748 = 0.08 us, folded into the same pass. This RETRACTS the earlier claim
    // that the locus was "the mean-`Wilders` ATR emitter". Both rungs are bit-exact
    // against their own hands -- gated by
    // `atr_ablation_rungs_bit_exact`, which also proves they ROUTE a generated
    // kernel (`run_registered` succeeding), so neither row can silently time the
    // hand against itself.
    let (_r6, sig_tr_n, vals_tr_n, _o6) =
        erase_request(&Request::new().add(tulip_rs::cse::recipes::tr(&[]), None)).unwrap();
    g.bench_function("generated_runonly_tr_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_tr_n, &vals_tr_n, &hlc3).expect("tr_n template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_tr_n", |b| {
        b.iter(|| {
            let t = tulip_rs::indicators::tr::Tr::indicator(&hlc3, &[], None).unwrap();
            black_box(t.0[0].len())
        })
    });
    let cl1: [&[f64]; 1] = [c0.as_slice()];
    let (_r7, sig_wl_n, vals_wl_n, _o7) =
        erase_request(&Request::new().add(tulip_rs::cse::recipes::wilders(&[14.0]), None)).unwrap();
    g.bench_function("generated_runonly_wilders_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_wl_n, &vals_wl_n, &cl1).expect("wilders_n template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_wilders_n", |b| {
        b.iter(|| {
            let w = tulip_rs::indicators::wilders::Wilders::indicator(&cl1, &[14.0], None).unwrap();
            black_box(w.0[0].len())
        })
    });
    // DIRECT-CALL PAIRS -- the per-call ROUTING cost, isolated. `try_run` dispatches
    // by walking `if *sig == SIG_x` over EVERY registered row (kernel! codegen), and a
    // `Signature` compare is derived `PartialEq` over `len` then fixed arrays
    // (`kinds[64]`, five `[u32;64]`, `sink_src`, `ring_class`, `ring_leader`), so the
    // walk is O(rows) and its size does NOT depend on the shape being run. These rows
    // call the generated kernel directly with the same plan and same inputs, so
    // `runonly_X - runonly_X_direct` IS that row's routing cost.
    //
    // Why this matters for the lone-shape ladder: a stateless row like `tr_n` has
    // almost no per-bar work to hide a fixed per-call cost behind, so a routing delta
    // can be the WHOLE engine-vs-hand gap. `atr_n` sits earlier in the registry than
    // `tr_n`/`wilders_n`, so the three deltas also test the position dependence.
    g.bench_function("generated_runonly_tr_n_direct", |b| {
        b.iter(|| {
            let rows = tulip_rs::cse::kernel::registry::tr_n(&vals_tr_n, &hlc3);
            black_box(rows[0].len())
        })
    });
    g.bench_function("generated_runonly_wilders_n_direct", |b| {
        b.iter(|| {
            let rows = tulip_rs::cse::kernel::registry::wilders_n(&vals_wl_n, &cl1);
            black_box(rows[0].len())
        })
    });
    g.bench_function("generated_runonly_atr_n_direct", |b| {
        b.iter(|| {
            let rows = tulip_rs::cse::kernel::registry::atr_n(&vals_atr_n, &hlc3);
            black_box(rows[0].len())
        })
    });

    let (_ru2, sig_run_mass, vals_run_mass, _ou2) =
        erase_request(&Request::new().add(Mass::recipe(&[40.0]), None)).unwrap();
    g.bench_function("generated_runonly_mass_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_run_mass, &vals_run_mass, &hl).expect("mass template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_mass_n", |b| {
        b.iter(|| {
            let m = Mass::indicator(&hl, &[40.0], None).unwrap();
            black_box(m.0[0].len())
        })
    });

    let (_ru3, sig_run_ad, vals_run_ad, _ou3) =
        erase_request(&Request::new().add(Ad::recipe(&[]), None)).unwrap();
    g.bench_function("generated_runonly_ad", |b| {
        b.iter(|| {
            let rows = try_run(&sig_run_ad, &vals_run_ad, &hlc4).expect("ad template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_ad", |b| {
        b.iter(|| {
            let a = Ad::indicator(&hlc4, &[], None).unwrap();
            black_box(a.0[0].len())
        })
    });

    let (_ru4, sig_run_adosc, vals_run_adosc, _ou4) =
        erase_request(&Request::new().add(Adosc::recipe(&[3.0, 10.0]), None)).unwrap();
    g.bench_function("generated_runonly_adosc_s_l", |b| {
        b.iter(|| {
            let rows = try_run(&sig_run_adosc, &vals_run_adosc, &hlc4).expect("adosc template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_adosc_s_l", |b| {
        b.iter(|| {
            let a = Adosc::indicator(&hlc4, &[3.0, 10.0], None).unwrap();
            black_box(a.0[0].len())
        })
    });

    let (_ru5, sig_run_mom, vals_run_mom, _ou5) =
        erase_request(&Request::new().add(Mom::recipe(&[10.0]), None)).unwrap();
    g.bench_function("generated_runonly_mom_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_run_mom, &vals_run_mom, &cl).expect("mom template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_mom_n", |b| {
        b.iter(|| {
            let m = Mom::indicator(&cl, &[10.0], None).unwrap();
            black_box(m.0[0].len())
        })
    });

    // THE headline: mass merged into the apo+ppo+macd hub. FOUR REAL
    // CO-REQUESTED RECIPES, one arena — the hub's three share the
    // `ema_s`/`ema_l`/`line` triple (apo's line IS macd's line, pinned by
    // `mass_apo_ppo_macd_bit_exact`), mass rides along for the single pass. The
    // hand rival is four separate calls, which is what a dashboard actually does
    // today. This used to be one hand-composed recipe (`recipes::mass_apo_ppo_macd`,
    // now deleted) because `mass` is HL-shaped while the hub is price-shaped and
    // positional slots made that mix silently wrong; `mass` now declares `hl()`,
    // so `merge` ranks {High:0, Low:1, Close:2} and the hub reads CLOSE by
    // derivation. Six primary rows (mass, apo, ppo, macd line/signal/hist) inside
    // twelve sinks, the other six being the recipes' own mirrors with want=false.
    let (_ru6, sig_run_hub, vals_run_hub, _ou6) = erase_request(
        &Request::new()
            .add(Mass::recipe(&[40.0]), None)
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[12.0, 26.0]), None)
            .add(Macd::recipe(&[12.0, 26.0, 9.0]), None),
    )
    .unwrap();
    g.bench_function("generated_runonly_mass_apo_ppo_macd", |b| {
        b.iter(|| {
            let rows = try_run(&sig_run_hub, &vals_run_hub, &hlc3).expect("mass+hub template");
            black_box(rows.iter().map(|r| r.len()).sum::<usize>())
        })
    });
    g.bench_function("hand_runonly_mass_apo_ppo_macd", |b| {
        b.iter(|| {
            let m = Mass::indicator(&hl, &[40.0], None).unwrap();
            let a = Apo::indicator(&cl, &APO, None).unwrap();
            let p = Ppo::indicator(&cl, &PPO, None).unwrap();
            let d = Macd::indicator(&cl, &MACD, None).unwrap();
            black_box(
                m.0[0].len()
                    + a.0[0].len()
                    + p.0[0].len()
                    + d.0[0].len()
                    + d.0[1].len()
                    + d.0[2].len(),
            )
        })
    });

    // ── THE CROSS-CONVENTION ARENA, packed — same-source only ────────────
    // Per-bar budget for this arena's SIX `Ema` chains (four over close, two over
    // the volume-driven AD line), which is the whole question:
    //   scalar  6 chains x (1 mul + 1 fma)                    = 12 vector ops
    //   `_p`    2 groups x (1 mul + 1 fma), source BROADCAST  =  4 vector ops
    //           — (Ema,close)=[15,16,18,19] 4-wide, (Ema,AD)=[12,13] 2-wide
    // MEASURED twice, one binary each, medians: run 1 scalar 12.515 → `_p` 12.031
    // (+4.0%), hand 17.712; run 2 scalar 12.582 → `_p` **11.873** (+6.0%), hand 17.768.
    // A third arm merging BOTH buckets into one 4-wide group that
    // reads a per-lane source vector was built, gated byte-exact, and measured
    // 12.798 µs — slower than `_p` AND slower than not packing — so it is not
    // registered here; the assembly of `<4 x f64>` from two locals per bar costs
    // more than the halved group count saves. Values for the packed arm are pinned
    // byte-equal to the scalar kernel by `adosc_apo_ppo_packed_matches_scalar`.
    // Hand rival: the same three indicators on their own arities (Adosc OHLCV,
    // Apo/Ppo close).
    {
        use tulip_rs::cse::kernel::registry::adosc_apo_ppo_p;
        use tulip_rs::indicators::adosc::Adosc;
        let (_aq, sig_aq, vals_aq, _oq) = erase_request(
            &Request::new()
                .add(Adosc::recipe(&[3.0, 10.0]), None)
                .add(Apo::recipe(&[12.0, 26.0]), None)
                .add(Ppo::recipe(&[6.0, 14.0]), None),
        )
        .unwrap();
        let cl_one: [&[f64]; 1] = [c0.as_slice()];
        g.bench_function("generated_runonly_adosc_apo_ppo", |b| {
            b.iter(|| {
                let rows = try_run(&sig_aq, &vals_aq, &hlc4).expect("scalar arm");
                black_box(rows.iter().map(|r| r.len()).sum::<usize>())
            })
        });
        g.bench_function("generated_runonly_adosc_apo_ppo_p", |b| {
            b.iter(|| {
                let rows = adosc_apo_ppo_p(&vals_aq, &hlc4);
                black_box(rows.iter().map(|r| r.len()).sum::<usize>())
            })
        });
        g.bench_function("hand_runonly_adosc_apo_ppo", |b| {
            b.iter(|| {
                let a = Adosc::indicator(&hlc4, &[3.0, 10.0], None).unwrap();
                let p = Apo::indicator(&cl_one, &[12.0, 26.0], None).unwrap();
                let q = Ppo::indicator(&cl_one, &[6.0, 14.0], None).unwrap();
                black_box(a.0[0].len() + p.0[0].len() + q.0[0].len())
            })
        });
    }

    // ── stoch — the skip/steady_from pair + THE flagship triple ──────────
    use tulip_rs::indicators::donchianchannel::DonchianChannel;
    use tulip_rs::indicators::stoch::Stoch;
    use tulip_rs::indicators::willr::Willr;
    let hlc3: [&[f64]; 3] = [h0.as_slice(), l0.as_slice(), c0.as_slice()];

    let (_rsk, sig_rsk, vals_rsk, _osk) =
        erase_request(&Request::new().add(Stoch::recipe(&[14.0, 3.0, 3.0]), None)).unwrap();
    g.bench_function("generated_runonly_stoch_kd", |b| {
        b.iter(|| {
            let rows = try_run(&sig_rsk, &vals_rsk, &hlc3).expect("stoch template");
            black_box(rows[0].len() + rows[1].len())
        })
    });
    g.bench_function("hand_runonly_stoch_kd", |b| {
        b.iter(|| {
            let s = Stoch::indicator(&hlc3, &[14.0, 3.0, 3.0], None).unwrap();
            black_box(s.0[0].len() + s.0[1].len())
        })
    });

    // Flagship single-asset pair: ONE pass of the engine vs the hand's THREE
    // — stoch's (14,13) roll pair IS willr's and donchian's.
    let (_rsd, sig_rsd, vals_rsd, _osd) = erase_request(
        &Request::new()
            .add(Stoch::recipe(&[14.0, 3.0, 3.0]), None)
            .add(Willr::recipe(&[14.0]), None)
            .add(DonchianChannel::recipe(&[14.0]), None),
    )
    .unwrap();
    g.bench_function("generated_runonly_stoch_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_rsd, &vals_rsd, &hlc3).expect("triple template");
            black_box(rows.iter().map(|r| r.len()).sum::<usize>())
        })
    });
    g.bench_function("hand_runonly_stoch_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let s = Stoch::indicator(&hlc3, &[14.0, 3.0, 3.0], None).unwrap();
            let w = Willr::indicator(&hlc3, &[14.0], None).unwrap();
            let d = DonchianChannel::indicator(&hl, &[14.0], None).unwrap();
            black_box(
                s.0[0].len()
                    + s.0[1].len()
                    + w.0[0].len()
                    + d.0[0].len()
                    + d.0[1].len()
                    + d.0[2].len(),
            )
        })
    });

    // ── rsi_n — dual mean-Wilders chains over the Max-gated movement arms.
    // Lone-shape diagnostic pair (hand `Rsi::indicator` is the oracle); the
    // `prev_real` Mirror rides the arena as a dead identity on both axes.
    use tulip_rs::indicators::rsi::Rsi;
    let (_r1, sig_r1, vals_r1, _o1) =
        erase_request(&Request::new().add(Rsi::recipe(&[14.0]), None)).unwrap();
    g.bench_function("generated_runonly_rsi_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_r1, &vals_r1, &cl).expect("rsi template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_rsi_n", |b| {
        b.iter(|| {
            let r = Rsi::indicator(&cl, &[14.0], None).unwrap();
            black_box(r.0[0].len())
        })
    });
    // THE grafted-chain flagship: Rsi + StochRsi co-requested — the entire
    // movement chain + both Wilders chains + division intern once, the
    // optional rsi row aliases the primary. Hand rival runs the two full
    // passes (and recomputes the shared chain twice).
    use tulip_rs::indicators::stochrsi::StochRsi;
    let (_rp, sig_rp, vals_rp, _op) = erase_request(
        &Request::new()
            .add(Rsi::recipe(&[14.0]), None)
            .add(StochRsi::recipe(&[14.0]), Some(&[true])),
    )
    .unwrap();
    g.bench_function("generated_runonly_rsi_stochrsi_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_rp, &vals_rp, &cl).expect("rsi+stochrsi template");
            black_box(rows[0].len() + rows[1].len() + rows[2].len())
        })
    });
    g.bench_function("hand_runonly_rsi_stochrsi_shared_p", |b| {
        b.iter(|| {
            let ri = Rsi::indicator(&cl, &[14.0], None).unwrap();
            let sr = StochRsi::indicator(&cl, &[14.0], Some(&[true])).unwrap();
            black_box(ri.0[0].len() + sr.0[0].len() + sr.0[1].len())
        })
    });

    // ── cmo_n — twin ring sums over the Max-gated movement arms (lone
    // diagnostic pair) + THE movement-sharing flagship vs the hand's TWO
    // full passes (Rsi::indicator + Cmo::indicator).
    use tulip_rs::indicators::cmo::Cmo;
    let (_c1, sig_c1, vals_c1, _d1) =
        erase_request(&Request::new().add(Cmo::recipe(&[14.0]), None)).unwrap();
    g.bench_function("generated_runonly_cmo_n", |b| {
        b.iter(|| {
            let rows = try_run(&sig_c1, &vals_c1, &cl).expect("cmo template");
            black_box(rows[0].len())
        })
    });
    g.bench_function("hand_runonly_cmo_n", |b| {
        b.iter(|| {
            let c = Cmo::indicator(&cl, &[14.0], None).unwrap();
            black_box(c.0[0].len())
        })
    });
    let (_rc, sig_rc, vals_rc, _oc) = erase_request(
        &Request::new()
            .add(Rsi::recipe(&[14.0]), None)
            .add(Cmo::recipe(&[14.0]), None),
    )
    .unwrap();
    g.bench_function("generated_runonly_rsi_cmo_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_rc, &vals_rc, &cl).expect("rsi+cmo template");
            black_box(rows[0].len() + rows[1].len())
        })
    });
    g.bench_function("hand_runonly_rsi_cmo_shared_p", |b| {
        b.iter(|| {
            let ri = Rsi::indicator(&cl, &[14.0], None).unwrap();
            let cm = Cmo::indicator(&cl, &[14.0], None).unwrap();
            black_box(ri.0[0].len() + cm.0[0].len())
        })
    });
    g.finish();

    // ── PLAN REUSE ACROSS ASSETS ──────────────────────────────────────────
    // The scanner contract: the plan (Request → merge → erase → Signature +
    // Values + resolved dispatch) is ASSET-INVARIANT, resolved ONCE at
    // setup; every timed iteration runs all 4 assets through it — one
    // kernel call per asset, zero planning inside the loop.
    let all: Vec<Vec<f64>> = (0..stocks().len()).map(close).collect();
    let all_inputs: Vec<[&[f64]; 1]> = all.iter().map(|v| [v.as_slice()]).collect();
    let (_a2, sig2, vals2, _o2) = erase_request(&apo_ppo_shared_sl_req()).unwrap();
    let mut ga = c.benchmark_group("fused_assets");
    ga.sample_size(20);

    ga.bench_function("hand_scalar_scan_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_inputs {
                let a = Apo::indicator(inputs, &APO, None).unwrap();
                let p = Ppo::indicator(inputs, &PPO, None).unwrap();
                let m = Macd::indicator(inputs, &MACD, None).unwrap();
                acc += a.0[0].len() + p.0[0].len() + m.0[0].len();
            }
            black_box(acc)
        })
    });
    // ONE (sig, vals) from setup; four cold kernel runs per iteration.
    ga.bench_function("generated_scalar_scan_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_inputs {
                let rows = try_run(&sig, &vals, inputs).expect("apo_ppo_macd template");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    // Pair shapes over the same 4-asset scan.
    ga.bench_function("hand_scalar_scan_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_inputs {
                let a = Apo::indicator(inputs, &APO, None).unwrap();
                let p = Ppo::indicator(inputs, &PPO, None).unwrap();
                acc += a.0[0].len() + p.0[0].len();
            }
            black_box(acc)
        })
    });
    ga.bench_function("generated_scalar_scan_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_inputs {
                let rows = try_run(&sig2, &vals2, inputs).expect("pair template");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    // ── by_assets rivals: the hand's own 4-wide SIMD across symbols. ──
    let by_assets_1: [&[&[f64]; 1]; 4] = [
        &[all[0].as_slice()],
        &[all[1].as_slice()],
        &[all[2].as_slice()],
        &[all[3].as_slice()],
    ];
    ga.bench_function("hand_byassets_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let a = Apo::indicator_by_assets::<4>(&by_assets_1, &APO, None).unwrap();
            let p = Ppo::indicator_by_assets::<4>(&by_assets_1, &PPO, None).unwrap();
            let m = Macd::indicator_by_assets::<4>(&by_assets_1, &MACD, None).unwrap();
            black_box(a.0[0][0].len() + p.0[0][0].len() + m.0[0][0].len())
        })
    });
    ga.bench_function("hand_byassets_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let a = Apo::indicator_by_assets::<4>(&by_assets_1, &APO, None).unwrap();
            let p = Ppo::indicator_by_assets::<4>(&by_assets_1, &PPO, None).unwrap();
            black_box(a.0[0][0].len() + p.0[0][0].len())
        })
    });
    // M2 asset lanes: ONE plan, 4 assets in Simd<f64, 4> lanes.
    let la_g1: [&[&[f64]]; 4] = [
        by_assets_1[0],
        by_assets_1[1],
        by_assets_1[2],
        by_assets_1[3],
    ];
    ga.bench_function("generated_assets4_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig, &vals, &la_g1).expect("trio lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    ga.bench_function("generated_assets4_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig2, &vals2, &la_g1).expect("pair lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    ga.finish();
}

/// ATR-family shapes in the SAME binary as their hand comparisons (the
/// layout rule). `atr_natr_shared_p:` Atr+Natr sharing the Wilders(tr)
/// chain; `natr_supertrend_shared_p:` Natr+SuperTrend sharing it — the shape
/// the (now archived) JIT LOST to hand at 1.42× and the macro engine was
/// first to beat. No st-SOLO generated row: that pattern is not registered
/// (M1 open item).
fn bench_fused_atr(c: &mut Criterion) {
    let (hi, lo, cl) = hlc(0);
    let three: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
    let (_a, sig_ap, vals_ap, _o) = erase_request(&atr_natr_shared_p_req()).unwrap();
    let (_b, sig_vt, vals_vt, _o) = erase_request(&natr_supertrend_shared_p_req()).unwrap();
    let mut g = c.benchmark_group("fused_atr");
    g.sample_size(20);

    g.bench_function("hand_runonly_atr_natr_shared_p", |b| {
        b.iter(|| {
            let a = Atr::indicator(&three, &[14.0], None).unwrap();
            let n = Natr::indicator(&three, &[14.0], None).unwrap();
            black_box((a.0[0].len(), n.0[0].len()))
        })
    });
    g.bench_function("generated_runonly_atr_natr_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_ap, &vals_ap, &three).expect("atr template");
            // full-sink layout: primaries at 0 (atr) and 2 (natr)
            black_box(rows[0].len() + rows[2].len())
        })
    });
    // supertrend solo: hand only (no generated template registered).
    g.bench_function("hand_runonly_supertrend", |b| {
        b.iter(|| {
            let s = SuperTrend::indicator(&three, &[14.0, 3.0], None).unwrap();
            black_box(s.0[0].len())
        })
    });

    // THE headline: natr + supertrend sharing one Wilders(tr) chain.
    g.bench_function("hand_runonly_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let n = Natr::indicator(&three, &[14.0], None).unwrap();
            let s = SuperTrend::indicator(&three, &[14.0, 3.0], None).unwrap();
            black_box((n.0[0].len(), s.0[0].len()))
        })
    });
    g.bench_function("generated_runonly_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_vt, &vals_vt, &three).expect("natr_supertrend hit");
            // full-sink layout: primaries at 0 (natr) and 3 (supertrend)
            black_box(rows[0].len() + rows[3].len())
        })
    });
    // SAME template, all optionals ON — want bits differ, not the machine
    // code; this row prices the 3 extra stores the ON path adds.
    let opt_req = Request::new()
        .add(Natr::recipe(&[14.0]), Some(&[true, true]))
        .add(SuperTrend::recipe(&[14.0, 3.0]), Some(&[true, true, true]));
    let (_c, sig_opt, vals_opt, _d) = erase_request(&opt_req).unwrap();
    g.bench_function("generated_runonly_natr_supertrend_shared_p_opton", |b| {
        b.iter(|| {
            let rows = try_run(&sig_opt, &vals_opt, &three).expect("opton must HIT");
            black_box(rows[0].len() + rows[6].len())
        })
    });

    // 4-asset scans off ONE plan each (the plan-reuse contract, ATR family).
    let all: Vec<(Vec<f64>, Vec<f64>, Vec<f64>)> = (0..stocks().len()).map(hlc).collect();
    let all_three: Vec<[&[f64]; 3]> = all
        .iter()
        .map(|(h, l, c)| [h.as_slice(), l.as_slice(), c.as_slice()])
        .collect();
    g.bench_function("hand_scalar_scan_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_three {
                let n = Natr::indicator(inputs, &[14.0], None).unwrap();
                let s = SuperTrend::indicator(inputs, &[14.0, 3.0], None).unwrap();
                acc += n.0[0].len() + s.0[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_scalar_scan_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_three {
                let rows = try_run(&sig_vt, &vals_vt, inputs).expect("natr_supertrend hit");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    // ── by_assets rivals (hand SIMD: 4 assets in ONE call, N-lane state) ──
    let by_assets_3: [&[&[f64]; 3]; 4] = [
        &[
            all[0].0.as_slice(),
            all[0].1.as_slice(),
            all[0].2.as_slice(),
        ],
        &[
            all[1].0.as_slice(),
            all[1].1.as_slice(),
            all[1].2.as_slice(),
        ],
        &[
            all[2].0.as_slice(),
            all[2].1.as_slice(),
            all[2].2.as_slice(),
        ],
        &[
            all[3].0.as_slice(),
            all[3].1.as_slice(),
            all[3].2.as_slice(),
        ],
    ];
    g.bench_function("hand_byassets_atr_natr_shared_p", |b| {
        b.iter(|| {
            let a = Atr::indicator_by_assets::<4>(&by_assets_3, &[14.0], None).unwrap();
            let n = Natr::indicator_by_assets::<4>(&by_assets_3, &[14.0], None).unwrap();
            black_box(a.0[0][0].len() + n.0[0][0].len())
        })
    });
    g.bench_function("hand_byassets_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let n = Natr::indicator_by_assets::<4>(&by_assets_3, &[14.0], None).unwrap();
            let s = SuperTrend::indicator_by_assets::<4>(&by_assets_3, &[14.0, 3.0], None).unwrap();
            black_box(n.0[0][0].len() + s.0[0][0].len())
        })
    });
    let la_a3: [&[&[f64]]; 4] = [
        by_assets_3[0],
        by_assets_3[1],
        by_assets_3[2],
        by_assets_3[3],
    ];
    g.bench_function("generated_assets4_atr_natr_shared_p", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_ap, &vals_ap, &la_a3).expect("atr_natr lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    g.bench_function("generated_assets4_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_vt, &vals_vt, &la_a3).expect("natr_st lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    // ATR scan off one plan, scalar hand for the shape not yet scanned:
    g.bench_function("hand_scalar_scan_atr_natr_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_three {
                let a = Atr::indicator(inputs, &[14.0], None).unwrap();
                let n = Natr::indicator(inputs, &[14.0], None).unwrap();
                acc += a.0[0].len() + n.0[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_scalar_scan_atr_natr_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_three {
                let rows = try_run(&sig_ap, &vals_ap, inputs).expect("atr template");
                acc += rows[0].len() + rows[2].len();
            }
            black_box(acc)
        })
    });

    // ── atr_multi: THREE ATRs, distinct periods, ONE shared tr subgraph.
    // Matrix per family: shared periods (above), unshared periods PACKED
    // (below), by_options, across assets (scan), by_assets.
    let atr_multi = || {
        Request::new()
            .add(Atr::recipe(&[9.0]), None)
            .add(Atr::recipe(&[14.0]), None)
            .add(Atr::recipe(&[21.0]), None)
    };
    let (_m, sig_am, vals_am, _o) = erase_request(&atr_multi()).unwrap();
    // Hand scalar baseline: three separate ATR passes.
    g.bench_function("hand_runonly_atr_multi", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [9.0, 14.0, 21.0] {
                let a = Atr::indicator(&three, &[p], None).unwrap();
                acc += a.0[0].len();
            }
            black_box(acc)
        })
    });
    // Hand SIMD across OPTIONS — N=4 BUCKET ONLY: `::<3>` is silently
    // WRONG (road_train buckets at 8/4/2, pinned by `by_options_bucket_test`),
    // so the rival carries a 4th period (28) and is GENEROUS to the hand by
    // one whole ATR. black_box the ARGS or LLVM hoists the call (it is
    // loop-invariant) out of the timed region.
    g.bench_function("hand_byoptions_atr_multi", |b| {
        b.iter(|| {
            let o: [[f64; 1]; 4] = [[9.0], [14.0], [21.0], [28.0]];
            let orefs: [&[f64; 1]; 4] = [&o[0], &o[1], &o[2], &o[3]];
            let a =
                Atr::indicator_by_options::<4>(black_box(&three), black_box(&orefs), None).unwrap();
            black_box(&a);
        })
    });
    g.bench_function("generated_runonly_atr_multi", |b| {
        b.iter(|| {
            let rows = try_run(&sig_am, &vals_am, &three).expect("atr_multi template");
            // sinks [w,tr ×3], primaries at 0/2/4 (flags None)
            black_box(rows[0].len() + rows[2].len() + rows[4].len())
        })
    });
    g.bench_function("generated_runonly_atr_multi_packed", |b| {
        b.iter(|| {
            let rows = tulip_rs::cse::atr_multi_p(&vals_am, &three);
            black_box(rows[0].len() + rows[2].len() + rows[4].len())
        })
    });
    g.bench_function("generated_scalar_scan_atr_multi", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_three {
                let rows = try_run(&sig_am, &vals_am, inputs).expect("atr_multi template");
                acc += rows[0].len() + rows[2].len() + rows[4].len();
            }
            black_box(acc)
        })
    });
    // Packed Wilders lanes across assets too (the by_assets comparison
    // needs the engine's BEST merged shape, not its scalar one).
    g.bench_function("generated_scalar_scan_atr_multi_packed", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &all_three {
                let rows = tulip_rs::cse::atr_multi_p(&vals_am, inputs);
                acc += rows[0].len() + rows[2].len() + rows[4].len();
            }
            black_box(acc)
        })
    });
    // by_assets rival for the multi shape: three 4-asset SIMD calls.
    g.bench_function("hand_byassets_atr_multi", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [9.0, 14.0, 21.0] {
                let a = Atr::indicator_by_assets::<4>(&by_assets_3, &[p], None).unwrap();
                acc += a.0[0][0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_assets4_atr_multi", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_am, &vals_am, &la_a3).expect("atr_multi lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    g.finish();
}

/// The wide-pairing proof shape: six price-sourced Ema chains with NO dedup
/// (macd 12/24, apo 10/20, ppo 6/14 all distinct) + the chained signal.
/// Scalar twin = 7 scalar chains; packed twin = `Simd<f64,4>` lanes
/// {ema12, ema24, ema10, ema20} + `Simd<f64,2>` {ema6, ema14} + scalar
/// signal. The twins share ONE Signature, so `try_run` always reaches the
/// scalar — the packed kernel is called DIRECTLY (pub re-export) with the
/// identical (vals, streams) to keep the A/B same-binary and same-bindings.
fn bench_fused_wide(c: &mut Criterion) {
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let (_a, sig, vals, _o) = erase_request(&ema_six_req()).unwrap();
    let mut g = c.benchmark_group("fused_wide");
    g.sample_size(20);

    g.bench_function("hand_runonly_ema_six", |b| {
        b.iter(|| {
            let a = Apo::indicator(&inputs, &[10.0, 20.0], None).unwrap();
            let p = Ppo::indicator(&inputs, &[6.0, 14.0], None).unwrap();
            let m = Macd::indicator(&inputs, &[12.0, 24.0, 9.0], None).unwrap();
            black_box((a.0[0].len(), p.0[0].len(), m.0[0].len()))
        })
    });
    // Scalar twin kernel (via dispatch — the template try_run reaches).
    g.bench_function("generated_runonly_ema_six", |b| {
        b.iter(|| {
            let rows = try_run(&sig, &vals, &inputs).expect("ema_six scalar twin");
            // sinks: macd [d,sig,hist,es,el], apo [d..], ppo [q..] — line@0, apo line@5
            black_box(rows[0].len() + rows[5].len())
        })
    });
    // Packed twin kernel — DIRECT call; identical (vals, streams), so the
    // only difference vs the row above is Simd<f64,4>+Simd<f64,2> stepping.
    g.bench_function("generated_runonly_ema_six_packed", |b| {
        b.iter(|| {
            let rows = tulip_rs::cse::ema_six_p(&vals, &inputs);
            black_box(rows[0].len() + rows[5].len())
        })
    });

    // ── ema_chains: the PURE six-chain shape (no glue) so the by_options
    // rival measures the same work: six distinct periods, one column. ──
    let ema_chains = || {
        let mut r = Request::new();
        for p in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
            r = r.add(Ema::recipe(&[p]), None);
        }
        r
    };
    let (_e, sig_ec, vals_ec, _o) = erase_request(&ema_chains()).unwrap();
    g.bench_function("hand_runonly_ema_chains", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
                let a = Ema::indicator(&inputs, &[p], None).unwrap();
                acc += a.0[0].len();
            }
            black_box(acc)
        })
    });
    // Hand SIMD across OPTIONS (bucket widths 4+2 — see
    // `by_options_bucket_test`; odd widths are silently wrong).
    g.bench_function("hand_byoptions_ema_chains", |b| {
        b.iter(|| {
            let o: [[f64; 1]; 4] = [[12.0], [24.0], [10.0], [20.0]];
            let orefs: [&[f64; 1]; 4] = [&o[0], &o[1], &o[2], &o[3]];
            let o2: [[f64; 1]; 2] = [[6.0], [14.0]];
            let orefs2: [&[f64; 1]; 2] = [&o2[0], &o2[1]];
            let a = Ema::indicator_by_options::<4>(black_box(&inputs), black_box(&orefs), None)
                .unwrap();
            let b2 = Ema::indicator_by_options::<2>(black_box(&inputs), black_box(&orefs2), None)
                .unwrap();
            black_box((&a, &b2));
        })
    });
    g.bench_function("generated_runonly_ema_chains", |b| {
        b.iter(|| {
            let rows = try_run(&sig_ec, &vals_ec, &inputs).expect("ema_chains template");
            black_box(rows.iter().map(|r| r.len()).sum::<usize>())
        })
    });
    g.bench_function("generated_runonly_ema_chains_packed", |b| {
        b.iter(|| {
            let rows = tulip_rs::cse::ema_chains_p(&vals_ec, &inputs);
            black_box(rows.iter().map(|r| r.len()).sum::<usize>())
        })
    });
    // 4-asset scan off one plan + the by_assets rival (six 4-asset calls).
    let allc: Vec<Vec<f64>> = (0..4).map(close).collect();
    let allc_inputs: Vec<[&[f64]; 1]> = allc.iter().map(|v| [v.as_slice()]).collect();
    let by_assets_c1: [&[&[f64]; 1]; 4] = [
        &[allc[0].as_slice()],
        &[allc[1].as_slice()],
        &[allc[2].as_slice()],
        &[allc[3].as_slice()],
    ];
    g.bench_function("generated_scalar_scan_ema_chains", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &allc_inputs {
                let rows = try_run(&sig_ec, &vals_ec, inputs).expect("ema_chains template");
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_scalar_scan_ema_chains_packed", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &allc_inputs {
                let rows = tulip_rs::cse::ema_chains_p(&vals_ec, inputs);
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("hand_byassets_ema_chains", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
                let a = Ema::indicator_by_assets::<4>(&by_assets_c1, &[p], None).unwrap();
                acc += a.0[0][0].len();
            }
            black_box(acc)
        })
    });
    let la_ec1: [&[&[f64]]; 4] = [
        by_assets_c1[0],
        by_assets_c1[1],
        by_assets_c1[2],
        by_assets_c1[3],
    ];
    g.bench_function("generated_assets4_ema_chains", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_ec, &vals_ec, &la_ec1).expect("ema_chains lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    g.finish();
}

/// Rolling family: the honest MA pair `Sma(14)+Wma(14)` (two co-requested
/// indicators, the hand pays two full passes; the standalone Sma row
/// interns onto the very `Sma` node wma mirrors), `Bbands(20, 2)`, and the
/// WIDE shape `sma_six` — six distinct periods on one column (nothing
/// dedups but the price), run scalar vs the `Simd<f64,4>`+`Simd<f64,2>`
/// packed twin called DIRECTLY (both twins share one Signature). Params
/// stay runtime; primary-only flags per the bench conventions.
fn bench_fused_rolling(c: &mut Criterion) {
    use tulip_rs::indicators::bbands::BBands;
    use tulip_rs::indicators::sma::Sma;
    use tulip_rs::indicators::wma::Wma;
    let series = close(0);
    let inputs: [&[f64]; 1] = [series.as_slice()];
    let sma_wma = || {
        Request::new()
            .add(Sma::recipe(&[14.0]), None)
            .add(Wma::recipe(&[14.0]), None)
    };
    let bb = || {
        Request::new()
            .add(BBands::recipe(&[20.0, 2.0]), None)
            .add(Wma::recipe(&[20.0]), None)
    };
    let sma_six = || {
        let mut r = Request::new();
        for p in [3.0, 7.0, 14.0, 20.0, 50.0, 100.0] {
            r = r.add(Sma::recipe(&[p]), None);
        }
        r
    };
    let (_a, sig_ma, vals_ma, _o) = erase_request(&sma_wma()).unwrap();
    let (_b, sig_bb, vals_bb, _o) = erase_request(&bb()).unwrap();
    let (_c, sig_6, vals_6, _o) = erase_request(&sma_six()).unwrap();
    let mut g = c.benchmark_group("fused_rolling");
    g.sample_size(20);

    g.bench_function("hand_runonly_sma_wma_shared_p", |b| {
        b.iter(|| {
            let s = Sma::indicator(&inputs, &[14.0], None).unwrap();
            let w = Wma::indicator(&inputs, &[14.0], None).unwrap();
            black_box(s.0[0].len() + w.0[0].len())
        })
    });
    g.bench_function("generated_runonly_sma_wma_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_ma, &vals_ma, &inputs).expect("sma_wma template");
            black_box(rows[0].len() + rows[1].len())
        })
    });
    g.bench_function("generated_endtoend_sma_wma_shared_p", |b| {
        b.iter(|| {
            let out = tulip_rs::cse::run_registered(&sma_wma(), &inputs).expect("sma_wma");
            black_box(out.rows[0].len() + out.rows[1].len())
        })
    });
    // The MERGED shape (user doctrine: the engine is for merging — a lone
    // bbands row is the hand's home turf): bbands + wma on one period, the
    // `Sma(p)` node interned across the middle row and the mirror.
    g.bench_function("hand_runonly_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let r = BBands::indicator(&inputs, &[20.0, 2.0], None).unwrap();
            let w = Wma::indicator(&inputs, &[20.0], None).unwrap();
            black_box(r.0[0].len() + r.0[1].len() + r.0[2].len() + w.0[0].len())
        })
    });
    g.bench_function("generated_runonly_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_bb, &vals_bb, &inputs).expect("bbands_wma template");
            // sinks [lo, mid, up, wma, mirror(disabled)] — lower + wma
            black_box(rows[0].len() + rows[3].len())
        })
    });
    g.bench_function("generated_endtoend_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let out = tulip_rs::cse::run_registered(&bb(), &inputs).expect("bbands_wma");
            black_box(out.rows[0].len() + out.rows[3].len())
        })
    });
    // ── the 6-SMA wide shape: is vectorized stepping real here? ──
    g.bench_function("hand_runonly_sma_six", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [3.0, 7.0, 14.0, 20.0, 50.0, 100.0] {
                let s = Sma::indicator(&inputs, &[p], None).unwrap();
                acc += s.0[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_runonly_sma_six", |b| {
        b.iter(|| {
            let rows = try_run(&sig_6, &vals_6, &inputs).expect("sma_six scalar template");
            black_box(rows.iter().map(|r| r.len()).sum::<usize>())
        })
    });
    // Packed twin — DIRECT call (same Signature, try_run reaches the
    // scalar first), so the only delta vs the row above is SIMD stepping.
    g.bench_function("generated_runonly_sma_six_packed", |b| {
        b.iter(|| {
            let rows = tulip_rs::cse::sma_six_p(&vals_6, &inputs);
            black_box(rows.iter().map(|r| r.len()).sum::<usize>())
        })
    });

    // ── matrix completion: by_options rival + 4-asset scans + by_assets ──
    // Hand SIMD across OPTIONS (bucket widths only — 4+2, see
    // `by_options_bucket_test`): the honest rival for six sets.
    g.bench_function("hand_byoptions_sma_six", |b| {
        b.iter(|| {
            let o: [[f64; 1]; 4] = [[3.0], [7.0], [14.0], [20.0]];
            let orefs: [&[f64; 1]; 4] = [&o[0], &o[1], &o[2], &o[3]];
            let o2: [[f64; 1]; 2] = [[50.0], [100.0]];
            let orefs2: [&[f64; 1]; 2] = [&o2[0], &o2[1]];
            let a = Sma::indicator_by_options::<4>(black_box(&inputs), black_box(&orefs), None)
                .unwrap();
            let b2 = Sma::indicator_by_options::<2>(black_box(&inputs), black_box(&orefs2), None)
                .unwrap();
            black_box((&a, &b2));
        })
    });
    let allc: Vec<Vec<f64>> = (0..4).map(close).collect();
    let allc_inputs: Vec<[&[f64]; 1]> = allc.iter().map(|v| [v.as_slice()]).collect();
    let by_assets_1: [&[&[f64]; 1]; 4] = [
        &[allc[0].as_slice()],
        &[allc[1].as_slice()],
        &[allc[2].as_slice()],
        &[allc[3].as_slice()],
    ];
    // MA pair across assets off ONE plan, vs the hand's two 4-asset calls.
    g.bench_function("hand_scalar_scan_sma_wma_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &allc_inputs {
                let s = Sma::indicator(inputs, &[14.0], None).unwrap();
                let w = Wma::indicator(inputs, &[14.0], None).unwrap();
                acc += s.0[0].len() + w.0[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_scalar_scan_sma_wma_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &allc_inputs {
                let rows = try_run(&sig_ma, &vals_ma, inputs).expect("sma_wma template");
                acc += rows[0].len() + rows[1].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("hand_byassets_sma_wma_shared_p", |b| {
        b.iter(|| {
            let s = Sma::indicator_by_assets::<4>(&by_assets_1, &[14.0], None).unwrap();
            let w = Wma::indicator_by_assets::<4>(&by_assets_1, &[14.0], None).unwrap();
            black_box(s.0[0][0].len() + w.0[0][0].len())
        })
    });
    let la_r1: [&[&[f64]]; 4] = [
        by_assets_1[0],
        by_assets_1[1],
        by_assets_1[2],
        by_assets_1[3],
    ];
    g.bench_function("generated_assets4_sma_wma_shared_p", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_ma, &vals_ma, &la_r1).expect("sma_wma lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    // BBands+Wma scan + by_assets rival.
    g.bench_function("generated_scalar_scan_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &allc_inputs {
                let rows = try_run(&sig_bb, &vals_bb, inputs).expect("bbands_wma template");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("hand_byassets_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let r = BBands::indicator_by_assets::<4>(&by_assets_1, &[20.0, 2.0], None).unwrap();
            let w = Wma::indicator_by_assets::<4>(&by_assets_1, &[20.0], None).unwrap();
            black_box(r.0[0][0].len() + w.0[0][0].len())
        })
    });
    g.bench_function("generated_assets4_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_bb, &vals_bb, &la_r1).expect("bbands_wma lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    // Six-smas scan (scalar + packed) + the 6×4-asset by_assets rival.
    g.bench_function("generated_scalar_scan_sma_six", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &allc_inputs {
                let rows = try_run(&sig_6, &vals_6, inputs).expect("sma_six template");
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_scalar_scan_sma_six_packed", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for inputs in &allc_inputs {
                let rows = tulip_rs::cse::sma_six_p(&vals_6, inputs);
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("hand_byassets_sma_six", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [3.0, 7.0, 14.0, 20.0, 50.0, 100.0] {
                let s = Sma::indicator_by_assets::<4>(&by_assets_1, &[p], None).unwrap();
                acc += s.0[0][0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_assets4_sma_six", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_6, &vals_6, &la_r1).expect("sma_six lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    g.finish();
}

/// The trail-rescan hub. NO packed rows on purpose: a roll's per-bar work
/// is GPR-side trail bookkeeping (add/compare/select), its RESCAN is
/// already the hand's 4-wide SIMD helper, and merging two rolls' rescans
/// into one walk is the measured anti-pattern (extreme rescans are
/// disjoint in time — a merged walk scans MORE). So the hub's matrix rows
/// are shared/runonly/endtoend/scan/by_assets.
fn bench_fused_roll(c: &mut Criterion) {
    use tulip_rs::indicators::aroon::Aroon;
    use tulip_rs::indicators::aroonosc::AroonOsc;
    use tulip_rs::indicators::donchianchannel::DonchianChannel;
    use tulip_rs::indicators::ichimoku::Ichimoku;
    use tulip_rs::indicators::max::Max;
    use tulip_rs::indicators::min::Min;
    use tulip_rs::indicators::willr::Willr;
    let (hi, lo, cl) = hlc(0);
    let s1: [&[f64]; 1] = [cl.as_slice()];
    let s2: [&[f64]; 2] = [hi.as_slice(), lo.as_slice()];
    let s3: [&[f64]; 3] = [hi.as_slice(), lo.as_slice(), cl.as_slice()];
    let max_min = || {
        Request::new()
            .add(Max::recipe(&[14.0]), None)
            .add(Min::recipe(&[14.0]), None)
    };
    let aroon_osc = || {
        Request::new()
            .add(Aroon::recipe(&[14.0]), None)
            .add(AroonOsc::recipe(&[14.0]), None)
    };
    let willr_donch = || {
        Request::new()
            .add(Willr::recipe(&[14.0]), None)
            .add(DonchianChannel::recipe(&[14.0]), None)
    };
    let ichimoku = || Request::new().add(Ichimoku::recipe(&[9.0, 26.0]), None);
    let (_a, sig_mm, vals_mm, _o) = erase_request(&max_min()).unwrap();
    let (_b, sig_ao, vals_ao, _o) = erase_request(&aroon_osc()).unwrap();
    let (_c, sig_wd, vals_wd, _o) = erase_request(&willr_donch()).unwrap();
    let (_d, sig_ic, vals_ic, _o) = erase_request(&ichimoku()).unwrap();
    let mut g = c.benchmark_group("fused_roll");
    g.sample_size(20);

    g.bench_function("hand_runonly_max_min_shared_p", |b| {
        b.iter(|| {
            let x = Max::indicator(&s1, &[14.0], None).unwrap();
            let n = Min::indicator(&s1, &[14.0], None).unwrap();
            black_box(x.0[0].len() + n.0[0].len())
        })
    });
    g.bench_function("generated_runonly_max_min_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_mm, &vals_mm, &s1).expect("max_min template");
            black_box(rows[0].len() + rows[1].len())
        })
    });
    g.bench_function("hand_runonly_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let a = Aroon::indicator(&s2, &[14.0], None).unwrap();
            let o = AroonOsc::indicator(&s2, &[14.0], None).unwrap();
            black_box(a.0[0].len() + o.0[0].len())
        })
    });
    g.bench_function("generated_runonly_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_ao, &vals_ao, &s2).expect("aroon_osc template");
            black_box(rows[0].len() + rows[2].len())
        })
    });
    g.bench_function("generated_endtoend_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let out = tulip_rs::cse::run_registered(&aroon_osc(), &s2).expect("aroon_osc");
            black_box(out.rows[0].len() + out.rows[2].len())
        })
    });
    g.bench_function("hand_runonly_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let w = Willr::indicator(&s3, &[14.0], None).unwrap();
            let d = DonchianChannel::indicator(&s2, &[14.0], None).unwrap();
            black_box(w.0[0].len() + d.0[0].len())
        })
    });
    g.bench_function("generated_runonly_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let rows = try_run(&sig_wd, &vals_wd, &s3).expect("willr_donch template");
            // sinks [wr, mn, mx, mn, mid, mx]; optionals disabled → primaries 0/3/4/5
            black_box(rows[0].len() + rows[3].len() + rows[4].len() + rows[5].len())
        })
    });
    g.bench_function("hand_runonly_ichimoku_s_m", |b| {
        b.iter(|| {
            let i = Ichimoku::indicator(&s3, &[9.0, 26.0], None).unwrap();
            black_box(i.0[0].len() + i.0[3].len())
        })
    });
    g.bench_function("generated_runonly_ichimoku_s_m", |b| {
        b.iter(|| {
            let rows = try_run(&sig_ic, &vals_ic, &s2).expect("ichimoku template");
            black_box(rows[0].len() + rows[3].len())
        })
    });

    // ── 4-asset scans off ONE plan + the by_assets rivals ──
    let all: Vec<(Vec<f64>, Vec<f64>, Vec<f64>)> = (0..4).map(hlc).collect();
    let sc1: Vec<[&[f64]; 1]> = all.iter().map(|(_, _, c)| [c.as_slice()]).collect();
    let sc2: Vec<[&[f64]; 2]> = all
        .iter()
        .map(|(h, l, _)| [h.as_slice(), l.as_slice()])
        .collect();
    let sc3: Vec<[&[f64]; 3]> = all
        .iter()
        .map(|(h, l, c)| [h.as_slice(), l.as_slice(), c.as_slice()])
        .collect();
    let ba1: [&[&[f64]; 1]; 4] = [
        &[all[0].2.as_slice()],
        &[all[1].2.as_slice()],
        &[all[2].2.as_slice()],
        &[all[3].2.as_slice()],
    ];
    let ba2: [&[&[f64]; 2]; 4] = [
        &[all[0].0.as_slice(), all[0].1.as_slice()],
        &[all[1].0.as_slice(), all[1].1.as_slice()],
        &[all[2].0.as_slice(), all[2].1.as_slice()],
        &[all[3].0.as_slice(), all[3].1.as_slice()],
    ];
    let ba3: [&[&[f64]; 3]; 4] = [
        &[
            all[0].0.as_slice(),
            all[0].1.as_slice(),
            all[0].2.as_slice(),
        ],
        &[
            all[1].0.as_slice(),
            all[1].1.as_slice(),
            all[1].2.as_slice(),
        ],
        &[
            all[2].0.as_slice(),
            all[2].1.as_slice(),
            all[2].2.as_slice(),
        ],
        &[
            all[3].0.as_slice(),
            all[3].1.as_slice(),
            all[3].2.as_slice(),
        ],
    ];
    let la_h1: [&[&[f64]]; 4] = [ba1[0], ba1[1], ba1[2], ba1[3]];
    let la_h2: [&[&[f64]]; 4] = [ba2[0], ba2[1], ba2[2], ba2[3]];
    let la_h3: [&[&[f64]]; 4] = [ba3[0], ba3[1], ba3[2], ba3[3]];
    // scans: hand (two scalar calls per asset) vs generated vs by_assets.
    g.bench_function("hand_scalar_scan_max_min_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1 {
                acc += Max::indicator(s, &[14.0], None).unwrap().0[0].len();
                acc += Min::indicator(s, &[14.0], None).unwrap().0[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_scalar_scan_max_min_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1 {
                let rows = try_run(&sig_mm, &vals_mm, s).expect("max_min template");
                acc += rows[0].len() + rows[1].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("hand_byassets_max_min_shared_p", |b| {
        b.iter(|| {
            let x = Max::indicator_by_assets::<4>(&ba1, &[14.0], None).unwrap();
            let n = Min::indicator_by_assets::<4>(&ba1, &[14.0], None).unwrap();
            black_box(x.0[0][0].len() + n.0[0][0].len())
        })
    });
    g.bench_function("generated_assets4_max_min_shared_p", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_mm, &vals_mm, &la_h1).expect("max_min lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    g.bench_function("hand_scalar_scan_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc2 {
                acc += Aroon::indicator(s, &[14.0], None).unwrap().0[0].len();
                acc += AroonOsc::indicator(s, &[14.0], None).unwrap().0[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_scalar_scan_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc2 {
                let rows = try_run(&sig_ao, &vals_ao, s).expect("aroon_osc template");
                acc += rows[0].len() + rows[2].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("hand_byassets_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let a = Aroon::indicator_by_assets::<4>(&ba2, &[14.0], None).unwrap();
            let o = AroonOsc::indicator_by_assets::<4>(&ba2, &[14.0], None).unwrap();
            black_box(a.0[0][0].len() + o.0[0][0].len())
        })
    });
    g.bench_function("generated_assets4_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_ao, &vals_ao, &la_h2).expect("aroon lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    g.bench_function("hand_scalar_scan_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc3 {
                acc += Willr::indicator(s, &[14.0], None).unwrap().0[0].len();
                let s2 = [s[0], s[1]];
                acc += DonchianChannel::indicator(&s2, &[14.0], None).unwrap().0[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_scalar_scan_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc3 {
                let rows = try_run(&sig_wd, &vals_wd, s).expect("willr_donch template");
                acc += rows[0].len() + rows[3].len() + rows[5].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("hand_byassets_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let w = Willr::indicator_by_assets::<4>(&ba3, &[14.0], None).unwrap();
            let d = DonchianChannel::indicator_by_assets::<4>(&ba2, &[14.0], None).unwrap();
            black_box(w.0[0][0].len() + d.0[0][0].len())
        })
    });
    g.bench_function("generated_assets4_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_wd, &vals_wd, &la_h3).expect("willr lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    g.bench_function("generated_scalar_scan_ichimoku_s_m", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc2 {
                let rows = try_run(&sig_ic, &vals_ic, s).expect("ichimoku template");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("hand_byassets_ichimoku_s_m", |b| {
        b.iter(|| {
            let i = Ichimoku::indicator_by_assets::<4>(&ba3, &[9.0, 26.0], None).unwrap();
            black_box(i.0[0][0].len())
        })
    });
    g.bench_function("generated_assets4_ichimoku_s_m", |b| {
        b.iter(|| {
            let out = try_run_assets::<4>(&sig_ic, &vals_ic, &la_h2).expect("ichimoku lanes");
            black_box(out.iter().flatten().map(|r| r.len()).sum::<usize>())
        })
    });
    g.finish();
}

/// Machine floor reference: the minimum a 6705-bar scan + one store-free
/// fold can cost. Every `*_runonly` row is this plus its real work.
fn bench_floor(c: &mut Criterion) {
    let s = close(0);
    let n = s.len();
    c.bench_function("floor_max_only", |b| {
        b.iter(|| {
            let mut m = f64::MIN;
            for i in 0..n {
                m = m.max(s[i]);
            }
            black_box(m)
        })
    });
}

/// The 8-asset asset-lane kernel: plan resolved ONCE, two batches of 4
/// assets each driven by the SAME erased (sig, vals) via `try_run_assets::<4>`
/// × 2 — the headline row. Plus scalar scan and hand rivals over all 8.
fn bench_fused_assets8(c: &mut Criterion) {
    init_database_data_8();
    let s8 = get_all_stock_data_8().unwrap();
    if s8.len() < 8 {
        eprintln!("fused_assets8: SKIPPED (only {} assets)", s8.len());
        return;
    }

    // ── Build per-asset H/L/C vectors for all 8 assets ─────────────────────
    let all_8: Vec<(Vec<f64>, Vec<f64>, Vec<f64>)> = (0..8)
        .map(|k| {
            let hi = s8[k].1.iter().map(|d| d.high).collect();
            let lo = s8[k].1.iter().map(|d| d.low).collect();
            let cl = s8[k].1.iter().map(|d| d.close).collect();
            (hi, lo, cl)
        })
        .collect();

    // ── Stream-count arrays: baS[i] = [&[&[f64]; S]; 8], laS[i] = [&[&[f64]]; 8] ──
    let ba1_8: [&[&[f64]; 1]; 8] = [
        &[all_8[0].2.as_slice()],
        &[all_8[1].2.as_slice()],
        &[all_8[2].2.as_slice()],
        &[all_8[3].2.as_slice()],
        &[all_8[4].2.as_slice()],
        &[all_8[5].2.as_slice()],
        &[all_8[6].2.as_slice()],
        &[all_8[7].2.as_slice()],
    ];
    let ba2_8: [&[&[f64]; 2]; 8] = [
        &[all_8[0].0.as_slice(), all_8[0].1.as_slice()],
        &[all_8[1].0.as_slice(), all_8[1].1.as_slice()],
        &[all_8[2].0.as_slice(), all_8[2].1.as_slice()],
        &[all_8[3].0.as_slice(), all_8[3].1.as_slice()],
        &[all_8[4].0.as_slice(), all_8[4].1.as_slice()],
        &[all_8[5].0.as_slice(), all_8[5].1.as_slice()],
        &[all_8[6].0.as_slice(), all_8[6].1.as_slice()],
        &[all_8[7].0.as_slice(), all_8[7].1.as_slice()],
    ];
    let ba3_8: [&[&[f64]; 3]; 8] = [
        &[
            all_8[0].0.as_slice(),
            all_8[0].1.as_slice(),
            all_8[0].2.as_slice(),
        ],
        &[
            all_8[1].0.as_slice(),
            all_8[1].1.as_slice(),
            all_8[1].2.as_slice(),
        ],
        &[
            all_8[2].0.as_slice(),
            all_8[2].1.as_slice(),
            all_8[2].2.as_slice(),
        ],
        &[
            all_8[3].0.as_slice(),
            all_8[3].1.as_slice(),
            all_8[3].2.as_slice(),
        ],
        &[
            all_8[4].0.as_slice(),
            all_8[4].1.as_slice(),
            all_8[4].2.as_slice(),
        ],
        &[
            all_8[5].0.as_slice(),
            all_8[5].1.as_slice(),
            all_8[5].2.as_slice(),
        ],
        &[
            all_8[6].0.as_slice(),
            all_8[6].1.as_slice(),
            all_8[6].2.as_slice(),
        ],
        &[
            all_8[7].0.as_slice(),
            all_8[7].1.as_slice(),
            all_8[7].2.as_slice(),
        ],
    ];
    // ── Split into two batches of 4 for the headline row ───────────────────
    // The VOLUME column, collected separately so `all_8` keeps its (hi, lo, cl)
    // arity everywhere it is already destructured. Only the OHLCV patterns
    // (`ad`, `adosc`) read it — hand `ad.rs::INPUTS = 4`.
    let vol_8: Vec<Vec<f64>> = (0..8)
        .map(|k| s8[k].1.iter().map(|d| d.volume).collect())
        .collect();
    let ba4_8: [&[&[f64]; 4]; 8] = [
        &[
            all_8[0].0.as_slice(),
            all_8[0].1.as_slice(),
            all_8[0].2.as_slice(),
            vol_8[0].as_slice(),
        ],
        &[
            all_8[1].0.as_slice(),
            all_8[1].1.as_slice(),
            all_8[1].2.as_slice(),
            vol_8[1].as_slice(),
        ],
        &[
            all_8[2].0.as_slice(),
            all_8[2].1.as_slice(),
            all_8[2].2.as_slice(),
            vol_8[2].as_slice(),
        ],
        &[
            all_8[3].0.as_slice(),
            all_8[3].1.as_slice(),
            all_8[3].2.as_slice(),
            vol_8[3].as_slice(),
        ],
        &[
            all_8[4].0.as_slice(),
            all_8[4].1.as_slice(),
            all_8[4].2.as_slice(),
            vol_8[4].as_slice(),
        ],
        &[
            all_8[5].0.as_slice(),
            all_8[5].1.as_slice(),
            all_8[5].2.as_slice(),
            vol_8[5].as_slice(),
        ],
        &[
            all_8[6].0.as_slice(),
            all_8[6].1.as_slice(),
            all_8[6].2.as_slice(),
            vol_8[6].as_slice(),
        ],
        &[
            all_8[7].0.as_slice(),
            all_8[7].1.as_slice(),
            all_8[7].2.as_slice(),
            vol_8[7].as_slice(),
        ],
    ];
    let la1a: [&[&[f64]]; 4] = [ba1_8[0], ba1_8[1], ba1_8[2], ba1_8[3]];
    let la1b: [&[&[f64]]; 4] = [ba1_8[4], ba1_8[5], ba1_8[6], ba1_8[7]];
    let la2a: [&[&[f64]]; 4] = [ba2_8[0], ba2_8[1], ba2_8[2], ba2_8[3]];
    let la2b: [&[&[f64]]; 4] = [ba2_8[4], ba2_8[5], ba2_8[6], ba2_8[7]];
    let la3a: [&[&[f64]]; 4] = [ba3_8[0], ba3_8[1], ba3_8[2], ba3_8[3]];
    let la3b: [&[&[f64]]; 4] = [ba3_8[4], ba3_8[5], ba3_8[6], ba3_8[7]];
    let la4a: [&[&[f64]]; 4] = [ba4_8[0], ba4_8[1], ba4_8[2], ba4_8[3]];
    let la4b: [&[&[f64]]; 4] = [ba4_8[4], ba4_8[5], ba4_8[6], ba4_8[7]];

    // ── Hand rival in the SAME shape: `by_assets::<4>` twice, never `::<8>`.
    // This box has no 8-wide f64 (AVX2 max), so a `::<8>` call is just the
    // driver's private split (user, 2026-10-12) — the batches are made
    // explicit here so engine-vs-hand is 2×4 vs 2×4, batching visible on
    // both sides.
    let bh1a: [&[&[f64]; 1]; 4] = [ba1_8[0], ba1_8[1], ba1_8[2], ba1_8[3]];
    let bh1b: [&[&[f64]; 1]; 4] = [ba1_8[4], ba1_8[5], ba1_8[6], ba1_8[7]];
    let bh2a: [&[&[f64]; 2]; 4] = [ba2_8[0], ba2_8[1], ba2_8[2], ba2_8[3]];
    let bh2b: [&[&[f64]; 2]; 4] = [ba2_8[4], ba2_8[5], ba2_8[6], ba2_8[7]];
    let bh3a: [&[&[f64]; 3]; 4] = [ba3_8[0], ba3_8[1], ba3_8[2], ba3_8[3]];
    let bh3b: [&[&[f64]; 3]; 4] = [ba3_8[4], ba3_8[5], ba3_8[6], ba3_8[7]];
    let bh4a: [&[&[f64]; 4]; 4] = [ba4_8[0], ba4_8[1], ba4_8[2], ba4_8[3]];
    let bh4b: [&[&[f64]; 4]; 4] = [ba4_8[4], ba4_8[5], ba4_8[6], ba4_8[7]];

    // ── Build scan vectors scN8 (Vec<[&[f64]; S]>) for each stream count ───
    let sc1_8: Vec<[&[f64]; 1]> = all_8.iter().map(|(_, _, c)| [c.as_slice()]).collect();
    let sc2_8: Vec<[&[f64]; 2]> = all_8
        .iter()
        .map(|(h, l, _)| [h.as_slice(), l.as_slice()])
        .collect();
    let sc3_8: Vec<[&[f64]; 3]> = all_8
        .iter()
        .map(|(h, l, c)| [h.as_slice(), l.as_slice(), c.as_slice()])
        .collect();

    // ── Patterns and their erased (sig, vals) ───────────────────────────────
    // apo_ppo_macd_shared_sl (S=1)
    let apo_ppo_macd = || {
        Request::new()
            .add(Apo::recipe(&APO), None)
            .add(Ppo::recipe(&PPO), None)
            .add(Macd::recipe(&MACD), None)
    };
    let (_a8, sig_apm, vals_apm, _o8) = erase_request(&apo_ppo_macd()).unwrap();

    // apo_ppo_shared_sl (S=1)
    let apo_ppo = || {
        Request::new()
            .add(Apo::recipe(&APO), None)
            .add(Ppo::recipe(&PPO), None)
    };
    let (_b8, sig_ap, vals_ap, _o8) = erase_request(&apo_ppo()).unwrap();

    // atr_natr_shared_p (S=3)
    let atr_natr = || {
        Request::new()
            .add(Atr::recipe(&[14.0]), None)
            .add(Natr::recipe(&[14.0]), None)
    };
    let (_c8, sig_atn, vals_atn, _o8) = erase_request(&atr_natr()).unwrap();

    // natr_supertrend_shared_p (S=3)
    let natr_st = || {
        Request::new()
            .add(Natr::recipe(&[14.0]), None)
            .add(SuperTrend::recipe(&[14.0, 3.0]), None)
    };
    let (_d8, sig_nst, vals_nst, _o8) = erase_request(&natr_st()).unwrap();

    // ema_six_req (S=1) — macd(12/24), apo(10/20), ppo(6/14)
    let ema_six = || {
        Request::new()
            .add(Macd::recipe(&[12.0, 24.0, 9.0]), None)
            .add(Apo::recipe(&[10.0, 20.0]), None)
            .add(Ppo::recipe(&[6.0, 14.0]), None)
    };
    let (_e8, sig_esx, vals_esx, _o8) = erase_request(&ema_six()).unwrap();

    // ema_chains (S=1) — six distinct periods
    let ema_chains = || {
        let mut r = Request::new();
        for p in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
            r = r.add(Ema::recipe(&[p]), None);
        }
        r
    };
    let (_f8, sig_ec, vals_ec, _o8) = erase_request(&ema_chains()).unwrap();

    // sma_six (S=1)
    let sma_six = || {
        let mut r = Request::new();
        for p in [3.0, 7.0, 14.0, 20.0, 50.0, 100.0] {
            r = r.add(Sma::recipe(&[p]), None);
        }
        r
    };
    use tulip_rs::indicators::sma::Sma;
    let (_g8, sig_s6, vals_s6, _o8) = erase_request(&sma_six()).unwrap();

    // max_min_shared_p (S=1)
    use tulip_rs::indicators::max::Max;
    use tulip_rs::indicators::min::Min;
    let max_min = || {
        Request::new()
            .add(Max::recipe(&[14.0]), None)
            .add(Min::recipe(&[14.0]), None)
    };
    let (_h8, sig_mm, vals_mm, _o8) = erase_request(&max_min()).unwrap();

    // aroon_osc_shared_p (S=2)
    use tulip_rs::indicators::aroon::Aroon;
    use tulip_rs::indicators::aroonosc::AroonOsc;
    let aroon_osc = || {
        Request::new()
            .add(Aroon::recipe(&[14.0]), None)
            .add(AroonOsc::recipe(&[14.0]), None)
    };
    let (_i8, sig_aro, vals_aro, _o8) = erase_request(&aroon_osc()).unwrap();

    // willr_donchian_shared_p (S=3)
    use tulip_rs::indicators::donchianchannel::DonchianChannel;
    use tulip_rs::indicators::willr::Willr;
    let willr_donch = || {
        Request::new()
            .add(Willr::recipe(&[14.0]), None)
            .add(DonchianChannel::recipe(&[14.0]), None)
    };
    let (_j8, sig_wd, vals_wd, _o8) = erase_request(&willr_donch()).unwrap();

    // ichimoku_s_m (S=2)
    use tulip_rs::indicators::ichimoku::Ichimoku;
    let ichimoku = || Request::new().add(Ichimoku::recipe(&[9.0, 26.0]), None);
    let (_k8, sig_ic, vals_ic, _o8) = erase_request(&ichimoku()).unwrap();

    // ── Build group and add rows ────────────────────────────────────────────
    let mut g = c.benchmark_group("fused_assets8");
    g.sample_size(20);

    // ── apo_ppo_macd_shared_sl (S=1) ────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_apm, &vals_apm, s).expect("apo_ppo_macd template");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_apm, &vals_apm, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_apm, &vals_apm, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_apo_ppo_macd_shared_sl", |b| {
        b.iter(|| {
            let a0 = Apo::indicator_by_assets::<4>(&bh1a, &APO, None).unwrap();
            let p0 = Ppo::indicator_by_assets::<4>(&bh1a, &PPO, None).unwrap();
            let m0 = Macd::indicator_by_assets::<4>(&bh1a, &MACD, None).unwrap();
            let a1 = Apo::indicator_by_assets::<4>(&bh1b, &APO, None).unwrap();
            let p1 = Ppo::indicator_by_assets::<4>(&bh1b, &PPO, None).unwrap();
            let m1 = Macd::indicator_by_assets::<4>(&bh1b, &MACD, None).unwrap();
            black_box(
                a0.0[0][0].len()
                    + p0.0[0][0].len()
                    + m0.0[0][0].len()
                    + a1.0[0][0].len()
                    + p1.0[0][0].len()
                    + m1.0[0][0].len(),
            )
        })
    });

    // ── apo_ppo_shared_sl (S=1) ─────────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_ap, &vals_ap, s).expect("pair template");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_ap, &vals_ap, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_ap, &vals_ap, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_apo_ppo_shared_sl", |b| {
        b.iter(|| {
            let a0 = Apo::indicator_by_assets::<4>(&bh1a, &APO, None).unwrap();
            let p0 = Ppo::indicator_by_assets::<4>(&bh1a, &PPO, None).unwrap();
            let a1 = Apo::indicator_by_assets::<4>(&bh1b, &APO, None).unwrap();
            let p1 = Ppo::indicator_by_assets::<4>(&bh1b, &PPO, None).unwrap();
            black_box(a0.0[0][0].len() + p0.0[0][0].len() + a1.0[0][0].len() + p1.0[0][0].len())
        })
    });

    // ── atr_natr_shared_p (S=3) ─────────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_atr_natr_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc3_8 {
                let rows = try_run(&sig_atn, &vals_atn, s).expect("atr template");
                acc += rows[0].len() + rows[2].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_atr_natr_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_atn, &vals_atn, &la3a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_atn, &vals_atn, &la3b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_atr_natr_shared_p", |b| {
        b.iter(|| {
            let x0 = Atr::indicator_by_assets::<4>(&bh3a, &[14.0], None).unwrap();
            let n0 = Natr::indicator_by_assets::<4>(&bh3a, &[14.0], None).unwrap();
            let x1 = Atr::indicator_by_assets::<4>(&bh3b, &[14.0], None).unwrap();
            let n1 = Natr::indicator_by_assets::<4>(&bh3b, &[14.0], None).unwrap();
            black_box(x0.0[0][0].len() + n0.0[0][0].len() + x1.0[0][0].len() + n1.0[0][0].len())
        })
    });

    // ── natr_supertrend_shared_p (S=3) ──────────────────────────────────────
    g.bench_function("generated_scalar_scan8_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc3_8 {
                let rows = try_run(&sig_nst, &vals_nst, s).expect("natr_st hit");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_nst, &vals_nst, &la3a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_nst, &vals_nst, &la3b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_natr_supertrend_shared_p", |b| {
        b.iter(|| {
            let n0 = Natr::indicator_by_assets::<4>(&bh3a, &[14.0], None).unwrap();
            let s0 = SuperTrend::indicator_by_assets::<4>(&bh3a, &[14.0, 3.0], None).unwrap();
            let n1 = Natr::indicator_by_assets::<4>(&bh3b, &[14.0], None).unwrap();
            let s1 = SuperTrend::indicator_by_assets::<4>(&bh3b, &[14.0, 3.0], None).unwrap();
            black_box(n0.0[0][0].len() + s0.0[0][0].len() + n1.0[0][0].len() + s1.0[0][0].len())
        })
    });

    // ── ema_six (S=1) ───────────────────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_ema_six", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_esx, &vals_esx, s).expect("ema_six scalar twin");
                acc += rows[0].len() + rows[5].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_ema_six", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_esx, &vals_esx, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_esx, &vals_esx, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_ema_six", |b| {
        b.iter(|| {
            let a0 = Apo::indicator_by_assets::<4>(&bh1a, &[10.0, 20.0], None).unwrap();
            let p0 = Ppo::indicator_by_assets::<4>(&bh1a, &[6.0, 14.0], None).unwrap();
            let m0 = Macd::indicator_by_assets::<4>(&bh1a, &[12.0, 24.0, 9.0], None).unwrap();
            let a1 = Apo::indicator_by_assets::<4>(&bh1b, &[10.0, 20.0], None).unwrap();
            let p1 = Ppo::indicator_by_assets::<4>(&bh1b, &[6.0, 14.0], None).unwrap();
            let m1 = Macd::indicator_by_assets::<4>(&bh1b, &[12.0, 24.0, 9.0], None).unwrap();
            black_box(
                a0.0[0][0].len()
                    + p0.0[0][0].len()
                    + m0.0[0][0].len()
                    + a1.0[0][0].len()
                    + p1.0[0][0].len()
                    + m1.0[0][0].len(),
            )
        })
    });

    // ── ema_chains (S=1) ────────────────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_ema_chains", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_ec, &vals_ec, s).expect("ema_chains template");
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_ema_chains", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_ec, &vals_ec, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_ec, &vals_ec, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_ema_chains", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [12.0, 24.0, 10.0, 20.0, 6.0, 14.0] {
                acc += Ema::indicator_by_assets::<4>(&bh1a, &[p], None).unwrap().0[0][0].len();
                acc += Ema::indicator_by_assets::<4>(&bh1b, &[p], None).unwrap().0[0][0].len();
            }
            black_box(acc)
        })
    });

    // ── sma_six (S=1) ───────────────────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_sma_six", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_s6, &vals_s6, s).expect("sma_six scalar template");
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_sma_six", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_s6, &vals_s6, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_s6, &vals_s6, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_sma_six", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [3.0, 7.0, 14.0, 20.0, 50.0, 100.0] {
                acc += Sma::indicator_by_assets::<4>(&bh1a, &[p], None).unwrap().0[0][0].len();
                acc += Sma::indicator_by_assets::<4>(&bh1b, &[p], None).unwrap().0[0][0].len();
            }
            black_box(acc)
        })
    });

    // ── max_min_shared_p (S=1) ──────────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_max_min_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_mm, &vals_mm, s).expect("max_min template");
                acc += rows[0].len() + rows[1].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_max_min_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_mm, &vals_mm, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_mm, &vals_mm, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_max_min_shared_p", |b| {
        b.iter(|| {
            let x0 = Max::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let n0 = Min::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let x1 = Max::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            let n1 = Min::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            black_box(x0.0[0][0].len() + n0.0[0][0].len() + x1.0[0][0].len() + n1.0[0][0].len())
        })
    });

    // ── aroon_osc_shared_p (S=2) ────────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc2_8 {
                let rows = try_run(&sig_aro, &vals_aro, s).expect("aroon_osc template");
                acc += rows[0].len() + rows[2].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_aro, &vals_aro, &la2a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_aro, &vals_aro, &la2b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_aroon_osc_shared_p", |b| {
        b.iter(|| {
            let r0 = Aroon::indicator_by_assets::<4>(&bh2a, &[14.0], None).unwrap();
            let o0 = AroonOsc::indicator_by_assets::<4>(&bh2a, &[14.0], None).unwrap();
            let r1 = Aroon::indicator_by_assets::<4>(&bh2b, &[14.0], None).unwrap();
            let o1 = AroonOsc::indicator_by_assets::<4>(&bh2b, &[14.0], None).unwrap();
            black_box(r0.0[0][0].len() + o0.0[0][0].len() + r1.0[0][0].len() + o1.0[0][0].len())
        })
    });

    // ── willr_donchian_shared_p (S=3) ───────────────────────────────────────
    g.bench_function("generated_scalar_scan8_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc3_8 {
                let rows = try_run(&sig_wd, &vals_wd, s).expect("willr_donch template");
                acc += rows[0].len() + rows[3].len() + rows[5].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_wd, &vals_wd, &la3a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_wd, &vals_wd, &la3b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let w0 = Willr::indicator_by_assets::<4>(&bh3a, &[14.0], None).unwrap();
            let d0 = DonchianChannel::indicator_by_assets::<4>(&bh2a, &[14.0], None).unwrap();
            let w1 = Willr::indicator_by_assets::<4>(&bh3b, &[14.0], None).unwrap();
            let d1 = DonchianChannel::indicator_by_assets::<4>(&bh2b, &[14.0], None).unwrap();
            black_box(w0.0[0][0].len() + d0.0[0][0].len() + w1.0[0][0].len() + d1.0[0][0].len())
        })
    });

    // ── stoch_kd (S=3, 2026-10-14 — TWO ring groups: slow %K over the
    // division column with the skip/steady_from fold, %D over the %K line;
    // warmed per-asset, ONE batched feed + leave read per group per bar) ────
    use tulip_rs::indicators::stoch::Stoch;
    let (_sk8, sig_sk, vals_sk, _sk9) =
        erase_request(&Request::new().add(Stoch::recipe(&[14.0, 3.0, 3.0]), None)).unwrap();
    g.bench_function("generated_scalar_scan8_stoch_kd", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc3_8 {
                let rows = try_run(&sig_sk, &vals_sk, s).expect("stoch template");
                acc += rows[0].len() + rows[1].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_stoch_kd", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_sk, &vals_sk, &la3a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_sk, &vals_sk, &la3b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_stoch_kd", |b| {
        b.iter(|| {
            let s0 = Stoch::indicator_by_assets::<4>(&bh3a, &[14.0, 3.0, 3.0], None).unwrap();
            let s1 = Stoch::indicator_by_assets::<4>(&bh3b, &[14.0, 3.0, 3.0], None).unwrap();
            black_box(
                s0.0.iter().flatten().map(|v| v.len()).sum::<usize>()
                    + s1.0.iter().flatten().map(|v| v.len()).sum::<usize>(),
            )
        })
    });

    // ── stoch_willr_donchian_shared_p (S=3) — THE roll-interning flagship:
    // stoch's (14,13) extrema pair IS willr's and donchian's, plus the two
    // ring groups — EIGHT rows off six roll/sum engines in ONE pass. Rival:
    // the hand trio (three full passes per batch).
    let (_sw8, sig_swd, vals_swd, _sw9) = erase_request(
        &Request::new()
            .add(Stoch::recipe(&[14.0, 3.0, 3.0]), None)
            .add(Willr::recipe(&[14.0]), None)
            .add(DonchianChannel::recipe(&[14.0]), None),
    )
    .unwrap();
    g.bench_function(
        "generated_scalar_scan8_stoch_willr_donchian_shared_p",
        |b| {
            b.iter(|| {
                let mut acc = 0usize;
                for s in &sc3_8 {
                    let rows = try_run(&sig_swd, &vals_swd, s).expect("stoch_wd template");
                    acc += rows.iter().map(|r| r.len()).sum::<usize>();
                }
                black_box(acc)
            })
        },
    );
    g.bench_function("generated_lanes8_2x4_stoch_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_swd, &vals_swd, &la3a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_swd, &vals_swd, &la3b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_stoch_willr_donchian_shared_p", |b| {
        b.iter(|| {
            let s0 = Stoch::indicator_by_assets::<4>(&bh3a, &[14.0, 3.0, 3.0], None).unwrap();
            let w0 = Willr::indicator_by_assets::<4>(&bh3a, &[14.0], None).unwrap();
            let d0 = DonchianChannel::indicator_by_assets::<4>(&bh2a, &[14.0], None).unwrap();
            let s1 = Stoch::indicator_by_assets::<4>(&bh3b, &[14.0, 3.0, 3.0], None).unwrap();
            let w1 = Willr::indicator_by_assets::<4>(&bh3b, &[14.0], None).unwrap();
            let d1 = DonchianChannel::indicator_by_assets::<4>(&bh2b, &[14.0], None).unwrap();
            black_box(
                s0.0.iter().flatten().map(|v| v.len()).sum::<usize>()
                    + w0.0[0][0].len()
                    + d0.0.iter().flatten().map(|v| v.len()).sum::<usize>()
                    + s1.0.iter().flatten().map(|v| v.len()).sum::<usize>()
                    + w1.0[0][0].len()
                    + d1.0.iter().flatten().map(|v| v.len()).sum::<usize>(),
            )
        })
    });

    // ── rsi_n (S=1: dual mean-Wilders chains over the Max-gated movement
    // arms; first pattern whose lane lowering carries a K_MIRROR identity —
    // dead-copy, DCE'd, the lane hit proves the kind emits clean) ──────────
    use tulip_rs::indicators::rsi::Rsi;
    let (_r8a, sig_r8, vals_r8, _r8b) =
        erase_request(&Request::new().add(Rsi::recipe(&[14.0]), None)).unwrap();
    g.bench_function("generated_scalar_scan8_rsi_n", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_r8, &vals_r8, s).expect("rsi template");
                acc += rows[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_rsi_n", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_r8, &vals_r8, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_r8, &vals_r8, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_rsi_n", |b| {
        b.iter(|| {
            let r0 = Rsi::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let r1 = Rsi::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            black_box(r0.0[0][0].len() + r1.0[0][0].len())
        })
    });

    // ── rsi_stochrsi_shared_p (S=1) — THE grafted-chain flagship: one pass
    // interns the whole movement chain + both Wilders chains + the division,
    // and the optional rsi row is the SAME stored stream as the primary.
    // Rival: the hand's two SIMD indicators per batch (each recomputes the
    // shared chain in full — the old JIT's exact loss, now the engine's win).
    use tulip_rs::indicators::stochrsi::StochRsi;
    let (_rp8, sig_rp8, vals_rp8, _op8) = erase_request(
        &Request::new()
            .add(Rsi::recipe(&[14.0]), None)
            .add(StochRsi::recipe(&[14.0]), Some(&[true])),
    )
    .unwrap();
    g.bench_function("generated_scalar_scan8_rsi_stochrsi_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_rp8, &vals_rp8, s).expect("rsi+stochrsi template");
                acc += rows[0].len() + rows[1].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_rsi_stochrsi_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_rp8, &vals_rp8, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_rp8, &vals_rp8, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_rsi_stochrsi_shared_p", |b| {
        b.iter(|| {
            let ri0 = Rsi::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let sr0 = StochRsi::indicator_by_assets::<4>(&bh1a, &[14.0], Some(&[true])).unwrap();
            let ri1 = Rsi::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            let sr1 = StochRsi::indicator_by_assets::<4>(&bh1b, &[14.0], Some(&[true])).unwrap();
            black_box(ri0.0[0][0].len() + ri1.0[0][0].len() + sr0.0[0][0].len() + sr1.0[0][0].len())
        })
    });

    // ── ultosc / mass — the FUSED-CONTAINER axis ───────────────────────────
    // `ultosc_s_m_l` is the shape that finally prices the fused container's
    // read cost: TWO independent pure-sum groups of THREE windows each, so the
    // planner names ONE `MultiBuffer<2>` and a bar costs two stores plus ONE
    // cursor advance for six folds (the hand answers it with `MultiBuffer<2>`
    // too — `ultosc.rs:9` — so this pair is a like-for-like container contest,
    // not a lone-shape race). `mass_n` is the DECLINES-TO-FUSE control: one
    // group, nothing independent to pair with.
    // `ultosc_natr_shared_pc` asks the timing version of a question the arena
    // already answered structurally: co-requested, ultosc and natr share the
    // `Lag{close,1}` anchor and NOTHING else (`K_MIN` stays 1, `K_ABS` stays 2
    // — their two `tr` spellings are value-equal but not interning-equal, pinned
    // by `ultosc_natr_shared_pc_sig_derived`), so this row prices what a
    // one-node share buys.
    use tulip_rs::indicators::mass::Mass;
    use tulip_rs::indicators::ultosc::Ultosc;
    let (_u8, sig_u8, vals_u8, _un8) =
        erase_request(&Request::new().add(Ultosc::recipe(&[7.0, 14.0, 28.0]), None)).unwrap();
    let (_m8, sig_m8, vals_m8, _mn8) =
        erase_request(&Request::new().add(Mass::recipe(&[25.0]), None)).unwrap();
    let (_up8, sig_up8, vals_up8, _unp8) = erase_request(
        &Request::new()
            .add(Ultosc::recipe(&[7.0, 14.0, 28.0]), None)
            .add(Natr::recipe(&[14.0]), None),
    )
    .unwrap();

    g.bench_function("generated_lanes8_2x4_ultosc_s_m_l", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_u8, &vals_u8, &la3a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_u8, &vals_u8, &la3b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_ultosc_s_m_l", |b| {
        b.iter(|| {
            let a = Ultosc::indicator_by_assets::<4>(&bh3a, &[7.0, 14.0, 28.0], None).unwrap();
            let bb = Ultosc::indicator_by_assets::<4>(&bh3b, &[7.0, 14.0, 28.0], None).unwrap();
            black_box(a.0[0][0].len() + bb.0[0][0].len())
        })
    });

    g.bench_function("generated_lanes8_2x4_mass_n", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_m8, &vals_m8, &la2a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_m8, &vals_m8, &la2b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_mass_n", |b| {
        b.iter(|| {
            let a = Mass::indicator_by_assets::<4>(&bh2a, &[25.0], None).unwrap();
            let bb = Mass::indicator_by_assets::<4>(&bh2b, &[25.0], None).unwrap();
            black_box(a.0[0][0].len() + bb.0[0][0].len())
        })
    });

    g.bench_function("generated_lanes8_2x4_ultosc_natr_shared_pc", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_up8, &vals_up8, &la3a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_up8, &vals_up8, &la3b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_ultosc_natr_shared_pc", |b| {
        b.iter(|| {
            let u0 = Ultosc::indicator_by_assets::<4>(&bh3a, &[7.0, 14.0, 28.0], None).unwrap();
            let n0 = Natr::indicator_by_assets::<4>(&bh3a, &[14.0], None).unwrap();
            let u1 = Ultosc::indicator_by_assets::<4>(&bh3b, &[7.0, 14.0, 28.0], None).unwrap();
            let n1 = Natr::indicator_by_assets::<4>(&bh3b, &[14.0], None).unwrap();
            black_box(u0.0[0][0].len() + n0.0[0][0].len() + u1.0[0][0].len() + n1.0[0][0].len())
        })
    });

    // ── the MIXED-INPUT arena — the experiment the old JIT lost ────────────
    // The user's standing question: `adosc + apo + ppo` was tried under the old
    // Cranelift JIT (apo/ppo EMAs read PRICE, adosc's read the AD line) and
    // "failed missurably in speed", blamed on cranelift's 2-lane ceiling. This
    // arena is the same workload on the macro engine, where a derived-source
    // `Ema` is literally the same arm a price-source one uses — no new
    // machinery, so any difference here is the engine, not a special case.
    //
    // THREE REAL CO-REQUESTS, not one composing recipe. This arena used to be
    // hand-authored (`recipes::adosc_apo_ppo`, now deleted) because stream slots
    // were positional: `Apo`/`Ppo` bind price to `Input{0}` while this arena's
    // slot 0 is HIGH, so a naive co-request silently computed APO/PPO over the
    // high column and every `SIG_*` check stayed green. That hazard is closed by
    // `merge`'s column-declared path — all three recipes declare their columns,
    // so apo's slot 0 is remapped onto the arena's CLOSE node — proven by
    // `adosc_apo_ppo_bit_exact`, which calls each hand on its own input array.
    //
    // Periods are all DISTINCT (adosc 3/10, apo 12/26, ppo 6/14) so nothing
    // dedups by accident: `K_EMA` is 6 (pinned by `adosc_apo_ppo_sig_derived`),
    // and the hand rival therefore does six EMA chains too. This row prices ONE
    // pass over four columns with a non-price recurrence in it, against three
    // separate hand passes.
    use tulip_rs::indicators::ad::Ad;
    use tulip_rs::indicators::adosc::Adosc;
    let (_ax8, sig_ax8, vals_ax8, _an8) = erase_request(
        &Request::new()
            .add(Adosc::recipe(&[3.0, 10.0]), None)
            .add(Apo::recipe(&[12.0, 26.0]), None)
            .add(Ppo::recipe(&[6.0, 14.0]), None),
    )
    .unwrap();
    let (_d8, sig_d8, vals_d8, _dn8) =
        erase_request(&Request::new().add(Adosc::recipe(&[3.0, 10.0]), None)).unwrap();
    let (_ad8, sig_ad8, vals_ad8, _ann8) =
        erase_request(&Request::new().add(Ad::recipe(&[]), None)).unwrap();

    g.bench_function("generated_lanes8_2x4_adosc_apo_ppo", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_ax8, &vals_ax8, &la4a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_ax8, &vals_ax8, &la4b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_adosc_apo_ppo", |b| {
        b.iter(|| {
            // Same six EMA chains, same three indicators — hand `adosc` takes
            // OHLCV, hand `apo`/`ppo` take close only.
            let a0 = Adosc::indicator_by_assets::<4>(&bh4a, &[3.0, 10.0], None).unwrap();
            let p0 = Apo::indicator_by_assets::<4>(&bh1a, &[12.0, 26.0], None).unwrap();
            let q0 = Ppo::indicator_by_assets::<4>(&bh1a, &[6.0, 14.0], None).unwrap();
            let a1 = Adosc::indicator_by_assets::<4>(&bh4b, &[3.0, 10.0], None).unwrap();
            let p1 = Apo::indicator_by_assets::<4>(&bh1b, &[12.0, 26.0], None).unwrap();
            let q1 = Ppo::indicator_by_assets::<4>(&bh1b, &[6.0, 14.0], None).unwrap();
            black_box(
                a0.0[0][0].len()
                    + p0.0[0][0].len()
                    + q0.0[0][0].len()
                    + a1.0[0][0].len()
                    + p1.0[0][0].len()
                    + q1.0[0][0].len(),
            )
        })
    });

    // Isolation rows, so the headline number is interpretable: `adosc_s_l` is
    // the same non-price recurrence WITHOUT the two price indicators (one pass
    // either way, no dedup to claim), and `ad` is the accumulator alone — the
    // cheapest possible `Accum` cost. A lone shape is the hand's home turf, so
    // these are diagnostics, not win claims.
    g.bench_function("generated_lanes8_2x4_adosc_s_l", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_d8, &vals_d8, &la4a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_d8, &vals_d8, &la4b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_adosc_s_l", |b| {
        b.iter(|| {
            let a = Adosc::indicator_by_assets::<4>(&bh4a, &[3.0, 10.0], None).unwrap();
            let bb = Adosc::indicator_by_assets::<4>(&bh4b, &[3.0, 10.0], None).unwrap();
            black_box(a.0[0][0].len() + bb.0[0][0].len())
        })
    });

    // NOTE the `_ohlcv` suffix: a row named `..._ad` is a PREFIX of
    // `..._adosc_s_l`, so a Criterion substring filter on it silently matches
    // three rows and the first-measured one is whoever ran first. The longer
    // name keeps every filter unambiguous — one filter, one row, one number.
    g.bench_function("generated_lanes8_2x4_ad_ohlcv", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_ad8, &vals_ad8, &la4a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_ad8, &vals_ad8, &la4b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_ad_ohlcv", |b| {
        b.iter(|| {
            let a = Ad::indicator_by_assets::<4>(&bh4a, &[], None).unwrap();
            let bb = Ad::indicator_by_assets::<4>(&bh4b, &[], None).unwrap();
            black_box(a.0[0][0].len() + bb.0[0][0].len())
        })
    });

    // ── cmo_n (S=1) — twin ring groups over the gain/loss columns; the
    // batched lane engine steps both with ONE feed + one eviction read per
    // group per bar. ────────────────────────────────────────────────────────
    use tulip_rs::indicators::cmo::Cmo;
    let (_cm8, sig_cm8, vals_cm8, _cn8) =
        erase_request(&Request::new().add(Cmo::recipe(&[14.0]), None)).unwrap();
    g.bench_function("generated_scalar_scan8_cmo_n", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_cm8, &vals_cm8, s).expect("cmo template");
                acc += rows[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_cmo_n", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_cm8, &vals_cm8, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_cm8, &vals_cm8, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_cmo_n", |b| {
        b.iter(|| {
            let c0 = Cmo::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let c1 = Cmo::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            black_box(c0.0[0][0].len() + c1.0[0][0].len())
        })
    });

    // ── rsi_cmo_shared_p (S=1) — THE movement-sharing flagship: one pass
    // computes Lag/diff/Max-gates once, Wilders pair AND ring sums consume
    // them. Rival: the hand's two SIMD indicators per batch. ───────────────
    let (_rc8, sig_rc8, vals_rc8, _rd8) = erase_request(
        &Request::new()
            .add(Rsi::recipe(&[14.0]), None)
            .add(Cmo::recipe(&[14.0]), None),
    )
    .unwrap();
    g.bench_function("generated_scalar_scan8_rsi_cmo_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_rc8, &vals_rc8, s).expect("rsi+cmo template");
                acc += rows[0].len() + rows[1].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_rsi_cmo_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_rc8, &vals_rc8, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_rc8, &vals_rc8, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_rsi_cmo_shared_p", |b| {
        b.iter(|| {
            let ri0 = Rsi::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let cm0 = Cmo::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let ri1 = Rsi::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            let cm1 = Cmo::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            black_box(ri0.0[0][0].len() + ri1.0[0][0].len() + cm0.0[0][0].len() + cm1.0[0][0].len())
        })
    });

    // ── ichimoku_s_m (S=2) ──────────────────────────────────────────────────
    g.bench_function("generated_scalar_scan8_ichimoku_s_m", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc2_8 {
                let rows = try_run(&sig_ic, &vals_ic, s).expect("ichimoku template");
                acc += rows[0].len() + rows[3].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_ichimoku_s_m", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_ic, &vals_ic, &la2a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_ic, &vals_ic, &la2b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_ichimoku_s_m", |b| {
        b.iter(|| {
            let i0 = Ichimoku::indicator_by_assets::<4>(&bh3a, &[9.0, 26.0], None).unwrap();
            let i1 = Ichimoku::indicator_by_assets::<4>(&bh3b, &[9.0, 26.0], None).unwrap();
            black_box(i0.0[0][0].len() + i1.0[0][0].len())
        })
    });

    // ── rolling pair: sma_wma / bbands_wma (S=1) ────────────────────────────
    // (sma_stddev deliberately NOT benched here — `Sma` is an optional
    // output of `StdDev`, so the pair measures nothing; user rule 2026-10-12.)
    use tulip_rs::indicators::bbands::BBands;
    use tulip_rs::indicators::wma::Wma;
    let sma_wma = || {
        Request::new()
            .add(Sma::recipe(&[14.0]), None)
            .add(Wma::recipe(&[14.0]), None)
    };
    let (_l8, sig_sw, vals_sw, _o8) = erase_request(&sma_wma()).unwrap();
    let bbands_wma = || {
        Request::new()
            .add(BBands::recipe(&[20.0, 2.0]), None)
            .add(Wma::recipe(&[20.0]), None)
    };
    let (_n8, sig_bw, vals_bw, _o8) = erase_request(&bbands_wma()).unwrap();
    g.bench_function("generated_scalar_scan8_sma_wma_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_sw, &vals_sw, s).expect("sma_wma template");
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_sma_wma_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_sw, &vals_sw, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_sw, &vals_sw, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_sma_wma_shared_p", |b| {
        b.iter(|| {
            let s0 = Sma::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let w0 = Wma::indicator_by_assets::<4>(&bh1a, &[14.0], None).unwrap();
            let s1 = Sma::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            let w1 = Wma::indicator_by_assets::<4>(&bh1b, &[14.0], None).unwrap();
            black_box(s0.0[0][0].len() + w0.0[0][0].len() + s1.0[0][0].len() + w1.0[0][0].len())
        })
    });
    g.bench_function("generated_scalar_scan8_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_bw, &vals_bw, s).expect("bbands_wma template");
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_bw, &vals_bw, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_bw, &vals_bw, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_bbands_wma_shared_p", |b| {
        b.iter(|| {
            let r0 = BBands::indicator_by_assets::<4>(&bh1a, &[20.0, 2.0], None).unwrap();
            let w0 = Wma::indicator_by_assets::<4>(&bh1a, &[20.0], None).unwrap();
            let r1 = BBands::indicator_by_assets::<4>(&bh1b, &[20.0, 2.0], None).unwrap();
            let w1 = Wma::indicator_by_assets::<4>(&bh1b, &[20.0], None).unwrap();
            black_box(r0.0[0][0].len() + w0.0[0][0].len() + r1.0[0][0].len() + w1.0[0][0].len())
        })
    });

    // ── atr_multi (S=3, three distinct periods) ─────────────────────────────
    let atr_multi = || {
        Request::new()
            .add(Atr::recipe(&[9.0]), None)
            .add(Atr::recipe(&[14.0]), None)
            .add(Atr::recipe(&[21.0]), None)
    };
    let (_o8b, sig_am, vals_am, _p8) = erase_request(&atr_multi()).unwrap();
    g.bench_function("generated_scalar_scan8_atr_multi", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc3_8 {
                let rows = try_run(&sig_am, &vals_am, s).expect("atr_multi template");
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_atr_multi", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_am, &vals_am, &la3a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_am, &vals_am, &la3b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_atr_multi", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for p in [9.0, 14.0, 21.0] {
                acc += Atr::indicator_by_assets::<4>(&bh3a, &[p], None).unwrap().0[0][0].len();
                acc += Atr::indicator_by_assets::<4>(&bh3b, &[p], None).unwrap().0[0][0].len();
            }
            black_box(acc)
        })
    });

    // ── ao_s_m (S=2, ring-backed — the step-C lane shapes) ─────────────────
    use tulip_rs::indicators::ao::Ao;
    let ao_req = || Request::new().add(Ao::recipe(&[]), None);
    let (_q8, sig_ao, vals_ao, _r8) = erase_request(&ao_req()).unwrap();
    g.bench_function("generated_scalar_scan8_ao_s_m", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc2_8 {
                let rows = try_run(&sig_ao, &vals_ao, s).expect("ao template");
                acc += rows[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_ao_s_m", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_ao, &vals_ao, &la2a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_ao, &vals_ao, &la2b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_ao_s_m", |b| {
        b.iter(|| {
            let a0 = Ao::indicator_by_assets::<4>(&bh2a, &[], None).unwrap();
            let a1 = Ao::indicator_by_assets::<4>(&bh2b, &[], None).unwrap();
            black_box(a0.0[0][0].len() + a1.0[0][0].len())
        })
    });

    // ── hma_n (S=1, bespoke machine + per-asset √n ring) ───────────────────
    use tulip_rs::indicators::hma::Hma;
    let hma_req = || Request::new().add(Hma::recipe(&[9.0]), None);
    let (_s8, sig_hm, vals_hm, _t8) = erase_request(&hma_req()).unwrap();
    g.bench_function("generated_scalar_scan8_hma_n", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc1_8 {
                let rows = try_run(&sig_hm, &vals_hm, s).expect("hma template");
                acc += rows[0].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_hma_n", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_hm, &vals_hm, &la1a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_hm, &vals_hm, &la1b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_hma_n", |b| {
        b.iter(|| {
            let h0 = Hma::indicator_by_assets::<4>(&bh1a, &[9.0], None).unwrap();
            let h1 = Hma::indicator_by_assets::<4>(&bh1b, &[9.0], None).unwrap();
            black_box(h0.0[0][0].len() + h1.0[0][0].len())
        })
    });

    // ── fisher_n (S=2: extremum ring + IIR chain; LANES since 2026-10-13,
    // lane `Ln` = the hand's SIMD ln_unchecked → lanes-vs-scalar is 1e-12) ──
    use tulip_rs::indicators::fisher::Fisher;
    let fisher_req = || Request::new().add(Fisher::recipe(&[9.0]), None);
    let (_u8, sig_fi, vals_fi, _v8) = erase_request(&fisher_req()).unwrap();
    g.bench_function("generated_scalar_scan8_fisher_n", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc2_8 {
                let rows = try_run(&sig_fi, &vals_fi, s).expect("fisher template");
                acc += rows[0].len() + rows[1].len();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_fisher_n", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_fi, &vals_fi, &la2a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_fi, &vals_fi, &la2b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_byassets2x4_fisher_n", |b| {
        b.iter(|| {
            let f0 = Fisher::indicator_by_assets::<4>(&bh2a, &[9.0], None).unwrap();
            let f1 = Fisher::indicator_by_assets::<4>(&bh2b, &[9.0], None).unwrap();
            black_box(f0.0[0][0].len() + f1.0[0][0].len())
        })
    });

    // THE sharing thesis (2026-10-13): AO + Fisher co-requested — ONE
    // interned medprice, ONE ring group (2 sums + 2 extrema), one pass per
    // asset. The old JIT re-ran medprice and lost ~3.5x here; the engine
    // adds ZERO arithmetic nodes (canary `ao_fisher_intern_med_and_share_one_ring`).
    // Rival: the hand pair — two full SIMD passes per batch.
    let ao_fisher_req = || {
        Request::new()
            .add(Ao::recipe(&[]), None)
            .add(Fisher::recipe(&[9.0]), None)
    };
    let (_w8, sig_af, vals_af, _x8) = erase_request(&ao_fisher_req()).unwrap();
    g.bench_function("generated_scalar_scan8_ao_fisher_shared_med", |b| {
        b.iter(|| {
            let mut acc = 0usize;
            for s in &sc2_8 {
                let rows = try_run(&sig_af, &vals_af, s).expect("ao+fisher template");
                acc += rows.iter().map(|r| r.len()).sum::<usize>();
            }
            black_box(acc)
        })
    });
    g.bench_function("generated_lanes8_2x4_ao_fisher_shared_med", |b| {
        b.iter(|| {
            let a = try_run_assets::<4>(&sig_af, &vals_af, &la2a).expect("batch A");
            let bb = try_run_assets::<4>(&sig_af, &vals_af, &la2b).expect("batch B");
            black_box(
                a.iter().flatten().map(|r| r.len()).sum::<usize>()
                    + bb.iter().flatten().map(|r| r.len()).sum::<usize>(),
            )
        })
    });
    g.bench_function("hand_pair_byassets2x4_ao_fisher", |b| {
        b.iter(|| {
            let a0 = Ao::indicator_by_assets::<4>(&bh2a, &[], None).unwrap();
            let f0 = Fisher::indicator_by_assets::<4>(&bh2a, &[9.0], None).unwrap();
            let a1 = Ao::indicator_by_assets::<4>(&bh2b, &[], None).unwrap();
            let f1 = Fisher::indicator_by_assets::<4>(&bh2b, &[9.0], None).unwrap();
            black_box(a0.0[0][0].len() + f0.0[0][0].len() + a1.0[0][0].len() + f1.0[0][0].len())
        })
    });

    g.finish();
}

criterion_group!(
    benches,
    bench_fused,
    bench_fused_atr,
    bench_fused_rolling,
    bench_fused_roll,
    bench_fused_wide,
    bench_fused_assets8,
    bench_floor
);
criterion_main!(benches);
