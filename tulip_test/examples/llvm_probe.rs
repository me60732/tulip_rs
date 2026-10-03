//! Inkwell/LLVM lane-width probe — EXPERIMENT, not the engine (cranelift
//! stays the engine unless this says otherwise).
//!
//! Hand-emits ONE function shaped like `jit_dashboard`'s steady segment —
//! 12 EMA chains in 6 (short, long) pairs, lane-aligned S/L vector groups,
//! ONE vector `fsub` per group, lane extracts feeding POINTER-WALK stores
//! into 6 output rows, chains stepped by `fmul` + explicit `llvm.fma` —
//! through two lane widths, answering the four switch-gating questions in
//! one process:
//!
//!   1. QUALITY   — LLVM's codegen of the w2 shape
//!   2. PRIZE     — w4 = real YMM on this AVX2 box: what lifting the
//!                  cranelift 128-bit SSA ceiling would actually buy
//!   3. COMPILE   — LLVM module+JIT per shape vs cranelift's ~394µs
//!   4. EXACTNESS — lane-wise `llvm.fma` == Rust `mul_add` bit-for-bit
//!                  (if exactness breaks, no port is viable — the
//!                  hand-parity suite would stop being the gate)
//!
//! VERDICT (this box, LLVM 18.1.3 vs the live engine, same-shape):
//!   quality  — llvm-w2 9.12µs vs clif same-shape twin 9.21µs: a DRAW
//!   prize    — w4 (true YMM `vfmadd231pd`, folded SIB stores) buys 1.4%
//!              on the stepping core: this loop is store/latency-bound,
//!              not lane-throughput-bound, and wider lanes also halve the
//!              independent chains hiding fma latency — the two effects
//!              cancel. Exactness holds at every width (explicit fma).
//!   compile  — LLVM 4.3–5.5ms vs cranelift ~1.1ms per shape; amortized
//!              by E4, but the dependency story (system LLVM + llvm-sys
//!              version coupling) starts to matter at 4×.
//! → no backend switch; cranelift + E1/E2-lite/E2b captured the same win.
//!
//! Run: `cargo run -p tulip_test --release --example llvm_probe`
//! (dumps /tmp/probe_w2.s / /tmp/probe_w4.s for inspection)

use inkwell::attributes::AttributeLoc;
use inkwell::context::Context;
use inkwell::intrinsics::Intrinsic;
use inkwell::targets::RelocMode;
use inkwell::targets::{CodeModel, InitializationConfig, Target, TargetMachine};
use inkwell::types::{BasicTypeEnum, VectorType};
use inkwell::values::{BasicValue, BasicValueEnum, PhiValue, ValueKind};
use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};
use std::hint::black_box;
use std::time::Instant;

const BARS: usize = 5000;
const ROUNDS: usize = 3000;
const NPAIRS: usize = 6; // 12 chains

/// short/long periods of the dense dashboard's six lines (ema9 appears as
/// both a long and a short in the real arena; the probe models stepping
/// shape, not sharing — sharing only ever REMOVES work, same for both
/// backends).
const PROBE_S: [usize; NPAIRS] = [5, 6, 9, 10, 11, 12];
const PROBE_L: [usize; NPAIRS] = [8, 9, 25, 26, 23, 24];

type ProbeFn = unsafe extern "C" fn(*mut f64, *const f64, u64, *mut *mut f64);

fn mult_inv(period: usize) -> (f64, f64) {
    let m = 2.0 / (period as f64 + 1.0);
    (m, 1.0 - m)
}

fn series() -> Vec<f64> {
    (0..BARS)
        .map(|b| (b as f64 * 0.7).sin() * 10.0 + 100.0)
        .collect()
}

/// Stepping order: prod = price*mult (exact fmul), next =
/// mul_add(prev, inv, prod) — one rounding, the same IEEE semantics as
/// cranelift `fma.f64x2` and lane-wise `llvm.fma`. Line = short − long.
fn scalar_run(close: &[f64]) -> Vec<Vec<f64>> {
    let mut prev = vec![close[0]; 2 * NPAIRS];
    let mut outs = vec![Vec::with_capacity(close.len()); NPAIRS];
    for &p in close {
        for j in 0..NPAIRS {
            let (ms, invs) = mult_inv(PROBE_S[j]);
            let (ml, invl) = mult_inv(PROBE_L[j]);
            let ns = prev[2 * j].mul_add(invs, p * ms);
            let nl = prev[2 * j + 1].mul_add(invl, p * ml);
            prev[2 * j] = ns;
            prev[2 * j + 1] = nl;
            outs[j].push(ns - nl);
        }
    }
    outs
}

struct Built {
    address: usize,
    compile_us: f64,
}

/// Build + JIT one probe function for lane width W. State words: the 12
/// chain prevs; `outs`: a table of 6 row pointers. w4 carries 6 pairs as
/// two groups — lanes 6,7 pad with mult=0/inv=1 frozen dummies that never
/// store (exactly what a real w4 dense plan would do).
fn build<const W: usize>(ctx: &Context, name: &str) -> Built {
    let t0 = Instant::now();
    let ngroups = NPAIRS.div_ceil(W);

    let module = ctx.create_module("probe");
    let triple = TargetMachine::get_default_triple();
    let tm = Target::from_triple(&triple)
        .unwrap()
        .create_target_machine(
            &triple,
            TargetMachine::get_host_cpu_name().to_str().unwrap(),
            TargetMachine::get_host_cpu_features().to_str().unwrap(),
            OptimizationLevel::Aggressive,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();
    module.set_triple(&tm.get_triple());
    module.set_data_layout(&tm.get_target_data().get_data_layout());

    let f64t = ctx.f64_type();
    let i64t = ctx.i64_type();
    let i32t = ctx.i32_type();
    let i8t = ctx.i8_type();
    let ptr = ctx.ptr_type(AddressSpace::from(0u16));
    let vd = f64t.vec_type(W as u32);

    let fty = ctx
        .void_type()
        .fn_type(&[ptr.into(), ptr.into(), i64t.into(), ptr.into()], false);
    let f = module.add_function(name, fty, None);
    // Pin host vector ISA explicitly (same CPUID truth cranelift-native
    // feeds its ISA): AVX2 + FMA on this box.
    f.add_attribute(
        AttributeLoc::Function,
        ctx.create_string_attribute("target-cpu", "x86-64"),
    );
    f.add_attribute(
        AttributeLoc::Function,
        ctx.create_string_attribute(
            "target-features",
            "+avx,+avx2,+fma,+bmi,+bmi2,+popcnt,+lzcnt,+cx16",
        ),
    );

    let bd = ctx.create_builder();
    let entry = ctx.append_basic_block(f, "entry");
    let header = ctx.append_basic_block(f, "loop");
    let body = ctx.append_basic_block(f, "body");
    let exit = ctx.append_basic_block(f, "exit");

    let state = f.get_nth_param(0).unwrap().into_pointer_value();
    let price = f.get_nth_param(1).unwrap().into_pointer_value();
    let len = f.get_nth_param(2).unwrap().into_int_value();
    let outs = f.get_nth_param(3).unwrap().into_pointer_value();

    macro_rules! byte_gep {
        ($base:expr, $off:expr) => {
            unsafe { bd.build_in_bounds_gep(i8t, $base, &[$off], "p").unwrap() }
        };
    }
    let c8 = i64t.const_int(8, false);

    // ---- entry: prev vectors from state, row ptrs from outs, i = 0 ----
    bd.position_at_end(entry);
    let mut init: Vec<BasicValueEnum> = Vec::new();
    for g in 0..ngroups {
        for half in 0..2 {
            // runtime insert chain (a const_vector of load instructions is
            // illegal IR — only compile-time values may form constants)
            let mut v = vd.get_poison();
            for l in 0..W {
                let pair = g * W + l;
                let elem = if pair < NPAIRS {
                    let idx = i64t.const_int((pair * 2 + half) as u64 * 8, false);
                    bd.build_load(f64t, byte_gep!(state, idx), "prev").unwrap()
                } else {
                    f64t.const_zero().as_basic_value_enum() // pad, frozen below
                };
                v = bd
                    .build_insert_element(v, elem, i32t.const_int(l as u64, false), "pins")
                    .unwrap();
            }
            init.push(v.as_basic_value_enum());
        }
    }
    for j in 0..NPAIRS {
        init.push(
            bd.build_load(
                ptr,
                byte_gep!(outs, i64t.const_int(j as u64 * 8, false)),
                "row",
            )
            .unwrap(),
        );
    }
    init.push(i64t.const_zero().as_basic_value_enum());
    bd.build_unconditional_branch(header).unwrap();

    // ---- header: one phi per carried value [vecs.., rows.., i] ----
    bd.position_at_end(header);
    let nvec = 2 * ngroups;
    let mut phi_vals: Vec<PhiValue> = Vec::with_capacity(init.len());
    let mut phis: Vec<BasicValueEnum> = Vec::with_capacity(init.len());
    for k in 0..init.len() {
        let phi = if k < nvec {
            bd.build_phi(vd, "vphi").unwrap()
        } else if k < nvec + NPAIRS {
            bd.build_phi(ptr, "rphi").unwrap()
        } else {
            bd.build_phi(i64t, "iphi").unwrap()
        };
        phis.push(phi.as_basic_value());
        phi_vals.push(phi);
    }
    let ivar = phis.last().unwrap().into_int_value();
    let done = bd
        .build_int_compare(IntPredicate::UGE, ivar, len, "done")
        .unwrap();
    bd.build_conditional_branch(done, exit, body).unwrap();

    // ---- body ----
    bd.position_at_end(body);
    let poff = bd.build_int_mul(ivar, c8, "off").unwrap();
    let pv_scalar = bd
        .build_load(f64t, byte_gep!(price, poff), "price")
        .unwrap()
        .into_float_value();
    // splat = insert lane 0 + shuffle broadcast
    let t0v = bd
        .build_insert_element(vd.get_poison(), pv_scalar, i32t.const_zero(), "ins0")
        .unwrap();
    let zeros: Vec<_> = (0..W).map(|_| i32t.const_zero()).collect();
    let mask = VectorType::const_vector(&zeros);
    let pv = bd.build_shuffle_vector(t0v, t0v, mask, "splat").unwrap();

    let fma = Intrinsic::find("llvm.fma")
        .unwrap()
        .get_declaration(&module, &[BasicTypeEnum::from(vd)])
        .unwrap();

    let side_const = |g: usize, half: usize, which: usize| -> Vec<inkwell::values::FloatValue> {
        (0..W)
            .map(|l| {
                let pair = g * W + l;
                let v = if pair >= NPAIRS {
                    if which == 0 {
                        0.0 // pad: mult 0
                    } else {
                        1.0 // pad: inv 1  (prev*1 + price*0 = prev, frozen)
                    }
                } else {
                    let per = if half == 0 {
                        PROBE_S[pair]
                    } else {
                        PROBE_L[pair]
                    };
                    let (m, i) = mult_inv(per);
                    if which == 0 {
                        m
                    } else {
                        i
                    }
                };
                f64t.const_float(v)
            })
            .collect()
    };

    let mut latch: Vec<BasicValueEnum> = Vec::with_capacity(init.len());
    let mut rows_next: Vec<Option<BasicValueEnum>> = vec![None; NPAIRS];
    for g in 0..ngroups {
        let ms = VectorType::const_vector(&side_const(g, 0, 0));
        let invs = VectorType::const_vector(&side_const(g, 0, 1));
        let ml = VectorType::const_vector(&side_const(g, 1, 0));
        let invl = VectorType::const_vector(&side_const(g, 1, 1));
        let sv = phis[2 * g].into_vector_value();
        let lv = phis[2 * g + 1].into_vector_value();
        let prods = bd.build_float_mul(pv, ms, "prods").unwrap();
        let snew = call_fma(&bd, fma, sv, invs, prods);
        let prodl = bd.build_float_mul(pv, ml, "prodl").unwrap();
        let lnew = call_fma(&bd, fma, lv, invl, prodl);
        let line = bd.build_float_sub(snew, lnew, "line").unwrap();
        latch.push(snew.as_basic_value_enum());
        latch.push(lnew.as_basic_value_enum());
        for l in 0..W {
            let pair = g * W + l;
            if pair >= NPAIRS {
                continue;
            }
            let v = bd
                .build_extract_element(line, i32t.const_int(l as u64, false), "ext")
                .unwrap();
            let row = phis[nvec + pair].into_pointer_value();
            bd.build_store(row, v).unwrap();
            rows_next[pair] = Some(byte_gep!(row, c8).as_basic_value_enum());
        }
    }
    latch.extend(rows_next.into_iter().flatten());
    latch.push(
        bd.build_int_add(ivar, i64t.const_int(1, false), "i1")
            .unwrap()
            .as_basic_value_enum(),
    );
    bd.build_unconditional_branch(header).unwrap();

    for (k, phi) in phi_vals.iter().enumerate() {
        phi.add_incoming(&[(&init[k], entry), (&latch[k], body)]);
    }

    // ---- exit: store final chain vectors back to state ----
    bd.position_at_end(exit);
    for g in 0..ngroups {
        for half in 0..2 {
            let v = phis[2 * g + half].into_vector_value();
            for l in 0..W {
                let pair = g * W + l;
                if pair >= NPAIRS {
                    continue;
                }
                let word = bd
                    .build_extract_element(v, i32t.const_int(l as u64, false), "fe")
                    .unwrap();
                let idx = i64t.const_int((pair * 2 + half) as u64 * 8, false);
                bd.build_store(byte_gep!(state, idx), word).unwrap();
            }
        }
    }
    bd.build_return(None).unwrap();

    module.verify().expect("probe module invalid");
    let asm_path = format!("/tmp/probe_w{W}.s");
    tm.write_to_file(
        &module,
        inkwell::targets::FileType::Assembly,
        std::path::Path::new(&asm_path),
    )
    .expect("emit asm");
    let ee = module
        .create_jit_execution_engine(OptimizationLevel::Aggressive)
        .expect("jit engine");
    let address = ee.get_function_address(name).unwrap();
    std::mem::forget(ee); // keep the code alive for the process
    Built {
        address,
        compile_us: t0.elapsed().as_secs_f64() * 1e6,
    }
}

fn call_fma<'ctx>(
    bd: &inkwell::builder::Builder<'ctx>,
    fma: inkwell::values::FunctionValue<'ctx>,
    a: inkwell::values::VectorValue<'ctx>,
    b: inkwell::values::VectorValue<'ctx>,
    c: inkwell::values::VectorValue<'ctx>,
) -> inkwell::values::VectorValue<'ctx> {
    let args = [
        a.as_basic_value_enum().into(),
        b.as_basic_value_enum().into(),
        c.as_basic_value_enum().into(),
    ];
    match bd
        .build_call(fma, &args, "fma")
        .unwrap()
        .try_as_basic_value()
    {
        ValueKind::Basic(b) => b.into_vector_value(),
        _ => panic!("fma call must produce a value"),
    }
}

/// Scratch buffers reused across rounds — timing measures the LOOP, not
/// allocator traffic (the first harness revision buried ~30µs of fresh
/// 240KB row allocation inside every measured iteration).
struct Scratch {
    state: Vec<f64>,
    rows: Vec<Vec<f64>>,
}

impl Scratch {
    fn new(close: &[f64]) -> Self {
        Self {
            state: vec![close[0]; 2 * NPAIRS],
            rows: (0..NPAIRS).map(|_| vec![0.0; close.len()]).collect(),
        }
    }
}

/// One cold run into fresh buffers (verification path).
fn run_once(address: usize, close: &[f64]) -> Vec<Vec<f64>> {
    let f: ProbeFn = unsafe { std::mem::transmute(address) };
    let mut state: Vec<f64> = vec![close[0]; 2 * NPAIRS];
    let mut rows: Vec<Vec<f64>> = (0..NPAIRS).map(|_| vec![0.0; close.len()]).collect();
    let mut tab: Vec<*mut f64> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    unsafe {
        f(
            state.as_mut_ptr(),
            close.as_ptr(),
            close.len() as u64,
            tab.as_mut_ptr(),
        )
    };
    black_box(&tab);
    rows
}

/// Repeated run into REUSED buffers (timing path). The table rebuild is
/// 6 pointer moves — noise vs the 5000-bar call, and keeps borrowck honest.
fn run_reuse(address: usize, close: &[f64], sc: &mut Scratch) {
    let f: ProbeFn = unsafe { std::mem::transmute(address) };
    let mut tab: Vec<*mut f64> = sc.rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    unsafe {
        f(
            sc.state.as_mut_ptr(),
            close.as_ptr(),
            close.len() as u64,
            tab.as_mut_ptr(),
        )
    };
    black_box((
        sc.state[0],
        sc.rows[0][0],
        sc.rows[NPAIRS - 1][close.len() - 1],
    ));
}

fn main() {
    Target::initialize_all(&InitializationConfig::default());
    let ctx = Context::create();
    let close = series();
    let reference = scalar_run(&close);

    println!(
        "{:<10}{:>13}{:>12}{:>10}{:>12}",
        "variant", "compile", "run/5000", "ns/bar", "bit-exact"
    );
    let mut results = Vec::new();
    for (label, w) in [("llvm-w2", 2usize), ("llvm-w4", 4)] {
        let built = match w {
            2 => build::<2>(&ctx, "probe2"),
            _ => build::<4>(&ctx, "probe4"),
        };
        let rows = run_once(built.address, &close);
        let exact = rows.iter().zip(&reference).all(|(a, b)| {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
        });
        let mut sc = Scratch::new(&close);
        for _ in 0..50 {
            run_reuse(built.address, &close, &mut sc);
        }
        let t = Instant::now();
        for _ in 0..ROUNDS {
            run_reuse(built.address, &close, &mut sc);
        }
        let us = t.elapsed().as_secs_f64() * 1e6 / ROUNDS as f64;
        results.push((label, us));
        println!(
            "{:<10}{:>12.0}µs{:>10.2}µs{:>9.2}{:>12}",
            label,
            built.compile_us,
            us,
            us * 1000.0 / BARS as f64,
            if exact { "YES" } else { "NO !" }
        );
    }
    let w2 = results[0].1;
    let w4 = results[1].1;
    println!(
        "\nprize of 4 lanes: {:.1}% on the probe loop",
        (1.0 - w4 / w2) * 100.0
    );

    // live cranelift comparison, same process, same series
    use tulip_rs::cse::{Planable, Request};
    use tulip_rs::indicators::apo::Apo;
    use tulip_rs::indicators::macd::Macd;
    use tulip_rs::indicators::ppo::Ppo;

    // SAME-SHAPE cranelift twin of the probe loop: six APO lines = 12 price
    // chains, 6 (paired) subs, 6 walking stores — op-for-op the probe's job
    let twin = Request::new()
        .add(Apo::recipe(&[5.0, 8.0]), None)
        .add(Apo::recipe(&[6.0, 9.0]), None)
        .add(Apo::recipe(&[11.0, 23.0]), None)
        .add(Apo::recipe(&[10.0, 26.0]), None)
        .add(Apo::recipe(&[9.0, 25.0]), None)
        .add(Apo::recipe(&[12.0, 24.0]), None);
    let t = Instant::now();
    let twin_plan = twin.build();
    let twin_compile = t.elapsed().as_secs_f64() * 1e6;
    let s: [&[f64]; 1] = [close.as_slice()];
    for _ in 0..50 {
        let (r, q) = twin_plan.run(&s);
        black_box((r.outputs[0][0].len(), q.next_bar()));
    }
    let t = Instant::now();
    for _ in 0..ROUNDS {
        let (r, q) = twin_plan.run(&s);
        black_box((r.outputs[0][0].len(), q.next_bar()));
    }
    let twin_us = t.elapsed().as_secs_f64() * 1e6 / ROUNDS as f64;
    println!(
        "{:<10}{:>12.0}µs{:>10.2}µs{:>9.2}{:>12}",
        "clif-twin",
        twin_compile,
        twin_us,
        twin_us * 1000.0 / BARS as f64,
        "suite ✅"
    );
    let dense = Request::new()
        .add(Apo::recipe(&[5.0, 8.0]), None)
        .add(Apo::recipe(&[6.0, 9.0]), None)
        .add(Ppo::recipe(&[9.0, 25.0]), None)
        .add(Ppo::recipe(&[10.0, 26.0]), None)
        .add(Macd::recipe(&[11.0, 23.0, 17.0]), None)
        .add(Macd::recipe(&[12.0, 24.0, 9.0]), None);
    let t = Instant::now();
    let plan = dense.build();
    let clif_compile = t.elapsed().as_secs_f64() * 1e6;
    let s: [&[f64]; 1] = [close.as_slice()];
    for _ in 0..50 {
        let (r, q) = plan.run(&s);
        black_box((r.outputs[0][0].len(), q.next_bar()));
    }
    let t = Instant::now();
    for _ in 0..ROUNDS {
        let (r, q) = plan.run(&s);
        black_box((r.outputs[0][0].len(), q.next_bar()));
    }
    let us = t.elapsed().as_secs_f64() * 1e6 / ROUNDS as f64;
    println!(
        "{:<10}{:>12.0}µs{:>10.2}µs{:>9.2}{:>12}",
        "cranelift",
        clif_compile,
        us,
        us * 1000.0 / BARS as f64,
        "suite ✅"
    );
    println!("(cranelift row = the FULL dense body incl. div/max/sigs — probe is the leaner");
    println!(" stepping-only core for both; compare compile + the w2→w4 prize)");
}
