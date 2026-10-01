//! JIT code generation — the CLIF emitter.
//!
//! Split from `jit.rs` (the plan contract + `compile_request` + `JitPlan`
//! runtime) as a pure code move: this child module sees every `jit`-private
//! item (`Op`/`OpE`/`JitPlan`/`JitState`/`SinkSpec`/`View`/`PRICE`/
//! `LEAD_NEVER`/`MAX_LANES`/`W_NEXT`/`W_SEED`/`vec_ty`/`JitFn`) through
//! `use super::*`, so lowering a new *stateful node kind* lands here, not in
//! the engine-contract file. Behavior is unchanged by construction; the
//! hand-parity suite is the gate.

// `use super::*` re-exports the parent's whole import set (a child module
// sees its parent's private items, cranelift `use`s included), so nothing
// here is imported directly.
use super::*;

// ---------------------------------------------------------------------------
// code generation
// ---------------------------------------------------------------------------

fn ba(vals: &[Value]) -> Vec<BlockArg> {
    vals.iter().copied().map(BlockArg::Value).collect()
}

fn add_params(fb: &mut FunctionBuilder, b: Block, tys: &[Type]) {
    for &t in tys {
        fb.append_block_param(b, t);
    }
}

fn ld(fb: &mut FunctionBuilder, base: Value, off: i64, ty: Type) -> Value {
    let offv = fb.ins().iconst(types::I64, off);
    let addr = fb.ins().iadd(base, offv);
    fb.ins().load(ty, MemFlagsData::new(), addr, 0)
}

fn st(fb: &mut FunctionBuilder, base: Value, off: i64, val: Value) {
    let offv = fb.ins().iconst(types::I64, off);
    let addr = fb.ins().iadd(base, offv);
    fb.ins().store(MemFlagsData::new(), val, addr, 0);
}

/// `a.max(b)` where `b` is the guard, matching the hand for **every shipped
/// use**: `di`/`dx` `.max(0.0)`, `ppo`/`stddev` `.max(EPSILON)` all pass a
/// constant `+0.0`/`+ε` as `b` and a possibly-NaN value as `a` (`0·inf`/`0/0`/
/// `sqrt(<0)`); `tr`'s maxes take finite price differences as both. The two-op
/// `select(a > b, a, b)` is `f64::max` for all finite inputs and on the ±0 tie,
/// and still yields the guard on a NaN first operand — `gt(NaN, b) = false ⇒ b`
/// — so `di`/`dx` on a zero ATR print `0.0` and `stddev`'s negative-variance
/// `sqrt` clamps to `EPSILON`. This is also exactly the hand `tr`'s `if x > tr`
/// reduction. It is NOT general IEEE maxNum: it diverges only when `b` could be
/// NaN, and no recipe guards against that (every `b` here is a non-NaN constant
/// or a finite price). The earlier `bor(fcmp.gt, fcmp.uno)` 4-op form bought
/// that generality at a cost on the hot `tr` path for no reachable benefit.
fn max_num(fb: &mut FunctionBuilder, a: Value, b: Value) -> Value {
    let gt = fb.ins().fcmp(FloatCC::GreaterThan, a, b);
    fb.ins().select(gt, a, b)
}

/// Build and define the plan's function; sets `plan.func`.
///
/// Phase-split (the hand `init_state`/sliced-`cycle` discipline as codegen):
/// the chunk's absolute bar line is cut at every gate point the plan has
/// (sink leads + chained feed leads), and ONE specialized loop is emitted
/// per segment. Inside a segment each gate is a compile-time constant:
/// sinks whose lead ≤ the segment start store UNCONDITIONALLY at
/// `i - base`; later sinks emit no gate at all; chained recurrences step
/// only in segments past their feed lead (the segment where feeding
/// begins keeps the one-time seed select). The steady (last) segment is
/// the hand shape: zero `icmp`, zero `brif` beyond the loop condition,
/// zero selects — branch cost paid once per boundary crossing, per
/// lifetime, not per bar.
///
/// E1 lane packing: the loop-carried VALUE SLOTS are `[group vectors..,
/// scalar op values..]` — each price-sourced EMA group (chains reading the
/// same stream, §7.1 pow-2 buckets) steps as ONE vector `fmul`+`fma`
/// (`vfmadd*pd`, lane-wise identical to scalar `mul_add`); consumers get
/// their lane via one cached `extractlane`. State words stay per-chain
/// (pack/unpack happens exactly once per chunk in setup/end, never in the
/// loop body's edges).
pub(super) fn emit(plan: &mut JitPlan, fc: TargetFrontendConfig) {
    let module = &mut plan.module;
    let i64_t = types::I64;
    let f64_t = types::F64;

    let nops = plan.ops.len();
    let nsinks = plan.stores.len();
    let ngroups = plan.groups.len();
    let chained: Vec<usize> = (0..nops)
        .filter(|&j| matches!(plan.ops[j].e, OpE::Ema { chained: true, .. }))
        .collect();
    let nsd = chained.len();
    // E3 rolling ops: cursor params (one i64 per ring) ride the param list
    // after the store pointers; stddev additionally carries its `sum_sq`
    // as an f64 aux param. Orders are op-id order, fixed at emit start.
    let rolls: Vec<usize> = (0..nops)
        .filter(|&j| {
            matches!(
                plan.ops[j].e,
                OpE::Sma { .. } | OpE::Stddev { .. } | OpE::Wma { .. }
            ) && plan.ops[j].roll_leader == j
        })
        .collect();
    let sqs: Vec<usize> = (0..nops)
        .filter(|&j| matches!(plan.ops[j].e, OpE::Stddev { .. } | OpE::Wma { .. }))
        .collect();
    // E6: raw delayed stream reads (prev-close etc.) — each owns its own
    // anchor base (no shared group), built alongside the rolling vbases.
    let lags: Vec<usize> = (0..nops)
        .filter(|&j| matches!(plan.ops[j].e, OpE::Lag { .. }))
        .collect();
    let nlag = lags.len();
    let mut lag_of = vec![usize::MAX; nops];
    for (k, &j) in lags.iter().enumerate() {
        lag_of[j] = k;
    }
    let nroll = rolls.len();
    let nsq = sqs.len();
    let mut roll_of = vec![usize::MAX; nops];
    for (k, &j) in rolls.iter().enumerate() {
        roll_of[j] = k;
        // followers resolve to their leader's cursor slot
    }
    for j in 0..nops {
        if plan.ops[j].roll_leader != usize::MAX {
            roll_of[j] = roll_of[plan.ops[j].roll_leader];
        }
    }
    let mut sq_of = vec![usize::MAX; nops];
    for (k, &j) in sqs.iter().enumerate() {
        sq_of[j] = k;
    }
    // E3 stage 3: HMA ops carry SIX f64 accumulators + one i64 ring
    // cursor as dedicated loop params (after the sq aux run).
    let hmas: Vec<usize> = (0..nops)
        .filter(|&j| matches!(plan.ops[j].e, OpE::Hma { .. }))
        .collect();
    let nh = hmas.len();
    let mut hma_of = vec![usize::MAX; nops];
    for (k, &j) in hmas.iter().enumerate() {
        hma_of[j] = k;
    }
    // E7: supertrend carries THREE f64 band states + one i64-backed bool
    // (trend) — a dedicated loop-param block, exactly like HMA's run.
    let sups: Vec<usize> = (0..nops)
        .filter(|&j| matches!(plan.ops[j].e, OpE::SuperTrend { .. }))
        .collect();
    let nsup = sups.len();
    let mut sup_of = vec![usize::MAX; nops];
    for (k, &j) in sups.iter().enumerate() {
        sup_of[j] = k;
    }
    // E9: a trail-rescan roll carries its extremum in the op's f64 value slot
    // (like an Ema) and its `trail` in a dedicated i64 run. Its rescan needs
    // one anchor base per op (the window's newest bar).
    let mmrolls: Vec<usize> = (0..nops)
        .filter(|&j| matches!(plan.ops[j].e, OpE::Roll { .. }))
        .collect();
    let ntr = mmrolls.len();
    let mut tr_of = vec![usize::MAX; nops];
    for (k, &j) in mmrolls.iter().enumerate() {
        tr_of[j] = k;
    }

    // Value slots: [0..ngroups) = group vectors, then ungrouped ops in op
    // order. `cur[op]` remains the per-op scalar view for consumers.
    let mut slot_of_op = vec![usize::MAX; nops];
    let mut op_of_slot: Vec<usize> = Vec::with_capacity(nops);
    for (g, grp) in plan.groups.iter().enumerate() {
        for &o in &grp.ops {
            slot_of_op[o] = g;
        }
    }
    for j in 0..nops {
        if plan.lane_of[j].0 == usize::MAX
            && (matches!(
                plan.ops[j].e,
                OpE::Ema { .. } | OpE::Wilders { .. } | OpE::Roll { .. }
            ) || (matches!(
                plan.ops[j].e,
                OpE::Sma { .. } | OpE::Stddev { .. } | OpE::Wma { .. }
            ) && plan.ops[j].roll_leader == j))
        {
            slot_of_op[j] = ngroups + op_of_slot.len();
            op_of_slot.push(j);
        }
    }
    let nval = ngroups + op_of_slot.len();

    // Segment bounds: sorted distinct finite gate leads (sink + chained
    // feed); segments are [0,b0), [b0,b1), …, [blast, ∞) in absolute bars.
    let mut bounds: Vec<u64> = plan
        .stores
        .iter()
        .map(|s| s.lead)
        .filter(|&l| l < LEAD_NEVER && l > 0)
        .chain(plan.ops.iter().filter_map(|op| match op.e {
            OpE::Ema {
                chained: true,
                feed,
                ..
            } if feed > 0 => Some(feed),
            // E3: rings fold while filling (bars <= lead) and trick once
            // the oldest slot is real (bars >= lead+1).
            OpE::Sma { .. } | OpE::Stddev { .. } => Some(op.lead + 1),
            // E6: Wilders folds its mean seed over bars 0..lead then decays
            // from bar lead+1 — the fold→steady seam.
            OpE::Wilders { .. } => Some(op.lead + 1),
            // E7: supertrend idles until the atr seam (its `lead`), seeds one
            // `calc_st` there, steady-decays after — the sink store lead
            // (lead+1 = period) cuts the seed→steady seam, so only `lead`
            // (idle→seed) is added here.
            OpE::SuperTrend { .. } => Some(op.lead),
            // E9: a trail-rescan roll's guard (`period <= trail`) is
            // UNREACHABLE below bar `period` (the trail counts bars since bar
            // 0 at most), and reachable from `period` on — so the rescan
            // side-exit is a compile-time decision per segment if the line is
            // cut here. Standalone `max`/`min` (lead = period−1) get a
            // guard-free one-bar segment at their first printed bar; `aroon`
            // (lead = period) starts guarded.
            OpE::Roll { period, .. } => Some(period as u64),
            _ => None,
        }))
        .collect();
    // E3 stage 3 seams per HMA: engine-2 fold start, the fill (trick)
    // start, and the output start. The store's own lead covers the last
    // one while the sink prints, but the plan may hold the HMA value with
    // NO sink (dead node) — the fill→steady switch must still exist.
    for op in plan.ops.iter() {
        if let OpE::Hma {
            period, p2, psqrt, ..
        } = op.e
        {
            let off = period.saturating_sub(p2);
            if off > 0 {
                bounds.push(off as u64);
            }
            bounds.push(period as u64);
            bounds.push(period as u64 + psqrt as u64);
        }
    }
    bounds.sort_unstable();
    bounds.dedup();
    let mut segs: Vec<(u64, Option<u64>)> = Vec::with_capacity(bounds.len() + 1);
    {
        let mut prev = 0u64;
        for &b in &bounds {
            segs.push((prev, Some(b)));
            prev = b;
        }
        segs.push((prev, None));
    }

    let mut sig = module.make_signature();
    for _ in 0..5 {
        sig.params.push(AbiParam::new(i64_t));
    }
    let id = module
        .declare_function("cse_jit_plan", Linkage::Export, &sig)
        .expect("declare");

    let mut ctx = Context::new();
    ctx.func = Function::with_name_signature(UserFuncName::user(0, 0), sig.clone());
    let mut fb_ctx = FunctionBuilderContext::new();
    let mut fb = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);

    // Loop-carried params (IDENTICAL on every segment loop so phase
    // transitions are pass-throughs): [ value slots.., chained seeds.., i ].
    // `i` is the ABSOLUTE bar counter; `sdpos`/`ipos` index into it.
    let mut pt: Vec<Type> = Vec::with_capacity(nval);
    for (g, grp) in plan.groups.iter().enumerate() {
        let _ = g;
        pt.push(vec_ty(grp.ops.len()));
    }
    pt.extend(std::iter::repeat(f64_t).take(op_of_slot.len()));
    pt.extend(std::iter::repeat(i64_t).take(nsd));
    // E2b: one loop-carried WRITE POINTER per sink (int file — separate
    // budget from the xmm lanes). Segments cut at every sink lead, so
    // inside a segment a sink stores EVERY bar or NEVER: the pointer
    // needs no gating, only the unconditional +8 on storing bars.
    pt.extend(std::iter::repeat(i64_t).take(nsinks));
    pt.extend(std::iter::repeat(f64_t).take(nsq * ROLLING_AUX_PARAMS));
    // E3 stage 3: per HMA op — 6 carried f64 accumulators, then the
    // i64 ring cursor run.
    pt.extend(std::iter::repeat(f64_t).take(HMA_F64_PARAMS * nh));
    pt.extend(std::iter::repeat(i64_t).take(HMA_I64_PARAMS * nh));
    // E7: per supertrend op — 3 carried f64 band states (prev_st/prev_ub/
    // prev_lb) + one i64-backed bool (trend), threaded like HMA's run.
    pt.extend(std::iter::repeat(f64_t).take(SUP_F64_PARAMS * nsup));
    pt.extend(std::iter::repeat(i64_t).take(SUP_I64_PARAMS * nsup));
    // E9: per trail-rescan roll — its `trail` bar-count (int file).
    pt.extend(std::iter::repeat(i64_t).take(ntr * ROLL_TRAIL_PARAMS));
    pt.push(i64_t);
    let nparam = pt.len();
    let ipos = nparam - 1;
    let sdpos = nval;
    let ppos = nval + nsd;
    let auxpos = ppos + nsinks;
    let hpos = auxpos + nsq * ROLLING_AUX_PARAMS;
    let cpos = hpos + HMA_F64_PARAMS * nh;
    let spos = cpos + HMA_I64_PARAMS * nh;
    let tpos = spos + SUP_F64_PARAMS * nsup;
    let rpos = tpos + SUP_I64_PARAMS * nsup;

    let entry = fb.create_block();
    let setup = fb.create_block();
    let end_bb = fb.create_block();
    let mut head_bb = Vec::with_capacity(segs.len());
    let mut body_bb = Vec::with_capacity(segs.len());
    for _ in 0..segs.len() {
        head_bb.push(fb.create_block());
        body_bb.push(fb.create_block());
    }

    fb.append_block_params_for_function_params(entry);
    for &h in &head_bb {
        add_params(&mut fb, h, &pt);
    }
    for &b in &body_bb {
        add_params(&mut fb, b, &pt);
    }
    // end: value slots + seeds + the global seeded flag
    let mut et = pt.clone();
    et.pop();
    et.push(i64_t);
    add_params(&mut fb, end_bb, &et);

    // assemble a group vector from per-lane scalar sources
    macro_rules! pack {
        ($g:expr, $src:expr) => {{
            let g = $g;
            let ty = vec_ty(plan.groups[g].ops.len());
            let z = fb.ins().f64const(0.0);
            let mut v = fb.ins().splat(ty, z);
            for (l, &o) in plan.groups[g].ops.iter().enumerate() {
                v = fb.ins().insertlane(v, $src[o], l as u8);
            }
            v
        }};
    }

    // ---- entry: loads, empty-chunk early-out ----
    fb.switch_to_block(entry);
    let ep: Vec<Value> = fb.block_params(entry).to_vec();
    let (state, prices, anchors, len, outs) = (ep[0], ep[1], ep[2], ep[3], ep[4]);
    // E2 asset lanes: param 1 points at a table of `nassets` f64 slice
    // bases. An A = 1 plan loads exactly one pointer (same machine work).
    // E6: physical stream count = field · asset. Field reads (not
    // `n_streams()`) so it coexists with the `&mut plan.module` borrow.
    let na = plan.n_fields * plan.nassets;
    let mut pbase: Vec<Value> = Vec::with_capacity(na);
    for a in 0..na {
        pbase.push(ld(&mut fb, prices, 8 * a as i64, i64_t));
    }
    let zero = fb.ins().iconst(i64_t, 0);
    let one = fb.ins().iconst(i64_t, 1);
    let eight = fb.ins().iconst(i64_t, 8);

    let next_bar = ld(&mut fb, state, 8 * W_NEXT as i64, i64_t);
    let seeded = ld(&mut fb, state, 8 * W_SEED as i64, i64_t);
    let mut loads: Vec<Value> = Vec::with_capacity(nops);
    for j in 0..nops {
        let w = plan.ops[j].ema_word as i64;
        loads.push(ld(&mut fb, state, 8 * w, f64_t));
    }
    let mut sdloads: Vec<Value> = Vec::with_capacity(nsd);
    for &j in &chained {
        let w = plan.ops[j].sd_word as i64;
        sdloads.push(ld(&mut fb, state, 8 * w, i64_t));
    }
    let mut sqloads: Vec<Value> = Vec::with_capacity(nsq);
    for &j in &sqs {
        let w = plan.ops[j].aux_word as i64;
        sqloads.push(ld(&mut fb, state, 8 * w, f64_t));
    }
    // E3 stage 3: per HMA — six accumulator words, then the cursor word.
    let mut hloads: Vec<Value> = Vec::with_capacity(HMA_F64_PARAMS * nh);
    let mut cursloads: Vec<Value> = Vec::with_capacity(nh);
    for &j in &hmas {
        let base = plan.ops[j].aux_word as i64;
        for m in 0..HMA_F64_PARAMS as i64 {
            hloads.push(ld(&mut fb, state, 8 * (base + m), f64_t));
        }
        cursloads.push(ld(
            &mut fb,
            state,
            8 * (base + HMA_F64_PARAMS as i64),
            i64_t,
        ));
    }
    // E7: per supertrend — three band-state words (f64 bits) + the trend word
    // (i64 0/1).
    let mut sloads: Vec<Value> = Vec::with_capacity(SUP_F64_PARAMS * nsup);
    let mut tloads: Vec<Value> = Vec::with_capacity(nsup);
    for &j in &sups {
        let base = plan.ops[j].aux_word as i64;
        for m in 0..SUP_F64_PARAMS as i64 {
            sloads.push(ld(&mut fb, state, 8 * (base + m), f64_t));
        }
        tloads.push(ld(
            &mut fb,
            state,
            8 * (base + SUP_F64_PARAMS as i64),
            i64_t,
        ));
    }
    // E9: per trail-rescan roll — its `trail` word (a bar count, integer).
    let mut trloads: Vec<Value> = Vec::with_capacity(ntr);
    for &j in &mmrolls {
        trloads.push(ld(&mut fb, state, 8 * plan.ops[j].aux_word as i64, i64_t));
    }
    let empty = fb.ins().icmp(IntCC::Equal, len, zero);
    let mut end_empty: Vec<Value> = Vec::with_capacity(nval);
    for g in 0..ngroups {
        let v = pack!(g, loads);
        end_empty.push(v);
    }
    for &o in &op_of_slot {
        end_empty.push(loads[o]);
    }
    end_empty.extend(sdloads.iter().copied());
    end_empty.extend(std::iter::repeat(zero).take(nsinks));
    end_empty.extend(sqloads.iter().copied());
    end_empty.extend(hloads.iter().copied());
    end_empty.extend(cursloads.iter().copied());
    end_empty.extend(sloads.iter().copied());
    end_empty.extend(tloads.iter().copied());
    end_empty.extend(trloads.iter().copied());
    end_empty.push(seeded);
    fb.ins()
        .brif(empty, end_bb, &ba(&end_empty), setup, &ba(&[]));

    // ---- setup: seed selects, constants, out pointers, segment bounds ----
    fb.switch_to_block(setup);
    let is_seeded = fb.ins().icmp(IntCC::NotEqual, seeded, zero);
    let mut p0: Vec<Value> = Vec::with_capacity(na);
    for a in 0..na {
        p0.push(ld(&mut fb, pbase[a], 0, f64_t));
    }
    let end_abs = fb.ins().iadd(next_bar, len);
    let mut init: Vec<Value> = Vec::with_capacity(nops);
    for j in 0..nops {
        let v = match plan.ops[j].e {
            OpE::Ema {
                src,
                chained: false,
                ..
            } => {
                // cold chunk seeds from ITS asset's first bar
                fb.ins().select(is_seeded, loads[j], p0[PRICE - src])
            }
            OpE::Ema { .. } => loads[j],
            // E3: a cold chunk's bar 0 enters the fold exactly as the
            // hand's init loop starts it (sum = p0; sum_sq handled below).
            OpE::Sma { src, .. } | OpE::Stddev { src, .. } | OpE::Wma { src, .. }
                if plan.ops[j].roll_leader == j =>
            {
                fb.ins().select(is_seeded, loads[j], p0[PRICE - src])
            }
            // E6: Wilders bar 0 = the seed of the running sum (fold adds the
            // rest; the ÷period mean lands at the last fold bar).
            OpE::Wilders {
                src,
                chained: false,
                ..
            } => fb.ins().select(is_seeded, loads[j], p0[PRICE - src]),
            // Chained Wilders (atr over tr): no per-stream `p0` to seed from,
            // so the cold chain value is the (zero-initialized) slot and the
            // loop folds the source node-value from bar 0.
            OpE::Wilders { .. } => loads[j],
            // E9: cold chunk — bar 0 seeds the extremum exactly as the hand's
            // `init_state` opens (`max = real[0]` / `min = real[0]`, trail 0).
            OpE::Roll { src, .. } => fb.ins().select(is_seeded, loads[j], p0[PRICE - src]),
            _ => fb.ins().f64const(0.0),
        };
        init.push(v);
    }
    // Lane constants (setup values — they dominate every loop).
    let mut gmvec: Vec<Value> = Vec::with_capacity(ngroups);
    let mut givec: Vec<Value> = Vec::with_capacity(ngroups);
    for g in 0..ngroups {
        let ty = vec_ty(plan.groups[g].ops.len());
        let z = fb.ins().f64const(0.0);
        let mut mv = fb.ins().splat(ty, z);
        let mut iv = fb.ins().splat(ty, z);
        for (l, &o) in plan.groups[g].ops.iter().enumerate() {
            let (mult, inv) = match plan.ops[o].e {
                OpE::Ema { mult, inv, .. } => (mult, inv),
                _ => unreachable!(),
            };
            let mc = fb.ins().f64const(mult);
            let ic = fb.ins().f64const(inv);
            mv = fb.ins().insertlane(mv, mc, l as u8);
            iv = fb.ins().insertlane(iv, ic, l as u8);
        }
        gmvec.push(mv);
        givec.push(iv);
    }
    // Cold chunk: bar `start` (== 0) was consumed by the entry seed ⇒ i0 = 1.
    let start1 = fb.ins().iadd(next_bar, one);
    // A cold chunk skips bar 0 ONLY when some op consumes it as its seed
    // (the Ema/rolling/wilders `select(is_seeded, .., p0)` path). A
    // chain-free plan (medprice/tr: pure arithmetic off price streams) has
    // no seed — bar 0 must be evaluated by the loop, so i0 = next_bar.
    let seeds_bar0 = plan.ops.iter().any(|o| match o.e {
        OpE::Ema {
            src,
            chained: false,
            ..
        } => src > PRICE - na,
        OpE::Sma { .. } | OpE::Stddev { .. } | OpE::Wma { .. } => true,
        // A price Wilders consumes bar 0 as its `p0` seed (so skip it). A
        // CHAINED Wilders (`atr` over the `tr` node) has no per-stream `p0`,
        // so its cold chain value is `0.0` and the loop folds the source
        // node-value from bar 0 — it must NOT trigger the skip.
        OpE::Wilders { chained, .. } => !chained,
        // HMA cold-seeds sum1/ws1 from p0 (bar 0 consumed as its seed).
        OpE::Hma { .. } => true,
        // E9: a trail-rescan roll's cold seed IS bar 0 (hand `init_state`:
        // `max = real[0]`, `trail = 0`), so the loop must start at bar 1.
        OpE::Roll { .. } => true,
        _ => false,
    });
    let i0 = if seeds_bar0 {
        fb.ins().select(is_seeded, next_bar, start1)
    } else {
        next_bar
    };
    let mut optr: Vec<Value> = Vec::with_capacity(nsinks);
    for j in 0..nsinks {
        optr.push(ld(&mut fb, outs, 8 * j as i64, i64_t));
    }
    // Segment runtime upper bounds: hi_s = min(end, e_s). Segments are
    // entered in order and each head's `done` test passes the live set
    // straight through — skipping a fully-past segment costs one compare.
    let mut hi_v: Vec<Value> = Vec::with_capacity(segs.len());
    for (_b, e) in &segs {
        match e {
            Some(eb) => {
                let ec = fb.ins().iconst(i64_t, *eb as i64);
                let above = fb.ins().icmp(IntCC::SignedGreaterThan, end_abs, ec);
                hi_v.push(fb.ins().select(above, ec, end_abs));
            }
            None => hi_v.push(end_abs),
        }
    }
    // E2b: `optr[k]` (row 0 of sink k's buffer) is the pointer's initial
    // value — stores walk it by +8 per written bar, no index math per bar.
    let mut args: Vec<Value> = Vec::with_capacity(nparam);
    for g in 0..ngroups {
        let v = pack!(g, init);
        args.push(v);
    }
    for &o in &op_of_slot {
        args.push(init[o]);
    }
    args.extend(sdloads.iter().copied());
    args.extend(optr.iter().copied());
    for (k, &j) in sqs.iter().enumerate() {
        let (src, is_sd) = match plan.ops[j].e {
            OpE::Stddev { src, .. } => (src, true),
            OpE::Wma { src, .. } => (src, false),
            _ => unreachable!(),
        };
        let p0v = p0[PRICE - src];
        // cold init values mirror the hand's first fold step: stddev's
        // sum_sq gains p0·p0, wma's weighted_sum gains p0·1
        let cold = if is_sd { fb.ins().fmul(p0v, p0v) } else { p0v };
        args.push(fb.ins().select(is_seeded, sqloads[k], cold));
    }
    // E3 stage 3 HMA cold seeds: bar 0 (consumed by the seed select)
    // folds engine 1 only — hand `init_state(real, P)`: sum1 = p0,
    // ws1 = p0·1. Engine 2 starts at bar P − P/2 ≥ 1 and the diff
    // accumulators at bar P, so those four stay 0; cursor 0.
    let hzero = fb.ins().f64const(0.0);
    for (k, &j) in hmas.iter().enumerate() {
        let src = match plan.ops[j].e {
            OpE::Hma { src, .. } => src,
            _ => unreachable!(),
        };
        let p0v = p0[PRICE - src];
        for m in 0..HMA_F64_PARAMS {
            let cold = if m < 2 { p0v } else { hzero };
            let sel = fb
                .ins()
                .select(is_seeded, hloads[HMA_F64_PARAMS * k + m], cold);
            args.push(sel);
        }
    }
    for k in 0..nh {
        let sel = fb.ins().select(is_seeded, cursloads[k], zero);
        args.push(sel);
    }
    // E7 supertrend cold start mirrors hand `init_state`: prev_st = prev_ub =
    // prev_lb = 0.0, trend = false (0). No p0 seeding — the feedback only runs
    // from the atr seam (its `lead`), where the shared atr node already holds
    // the mean seed.
    for k in 0..nsup {
        for m in 0..SUP_F64_PARAMS {
            let sel = fb
                .ins()
                .select(is_seeded, sloads[SUP_F64_PARAMS * k + m], hzero);
            args.push(sel);
        }
    }
    for k in 0..nsup {
        let sel = fb.ins().select(is_seeded, tloads[k], zero);
        args.push(sel);
    }
    // E9: a cold roll's trail is 0 (the hand's `init_state` sets `trail = 0`
    // when it seeds the value from bar 0); a resumed chunk keeps its count.
    for (k, &_j) in mmrolls.iter().enumerate() {
        let sel = fb.ins().select(is_seeded, trloads[k], zero);
        args.push(sel);
    }
    args.push(i0);
    // E3 anchor bases (setup values — the per-bar cost is one iadd + one
    // load): per-stream virtual price array from the host anchor table,
    // with a fixed per-group byte offset `(pmax − P)·8` folded in, so
    // prev(bar i−P) is `load(vbase_g + poff)` sharing the price load's
    // `poff` — no predicate, no ring store, no cursor param.
    let mut abases: Vec<Value> = Vec::with_capacity(na);
    if nroll > 0 || nh > 0 || nlag > 0 || ntr > 0 {
        for a_idx in 0..na {
            abases.push(ld(&mut fb, anchors, 8 * a_idx as i64, i64_t));
        }
    }
    // E9: a trail-rescan roll walks its window `[i − look_back ..= i]` through
    // the same anchor. Its base is the anchor of THIS bar
    // (`abases[src] + 8·pmax`), so window slot k is `base + poff − 8k` — one
    // iadd per window element, no cursor and no ring store (the rescan is the
    // rare bar; the common bar is a compare and two selects).
    let mut mmroll_base: Vec<Value> = Vec::with_capacity(ntr);
    if ntr > 0 {
        let pmax8 = fb.ins().iconst(i64_t, 8 * plan.pmax as i64);
        for &j in &mmrolls {
            let src = match plan.ops[j].e {
                OpE::Roll { src, .. } => src,
                _ => unreachable!(),
            };
            mmroll_base.push(fb.ins().iadd(abases[PRICE - src], pmax8));
        }
    }
    let mut vbases: Vec<Value> = Vec::with_capacity(nroll);
    for &j in &rolls {
        let (src, period) = match plan.ops[j].e {
            OpE::Sma { src, period, .. } => (src, period),
            OpE::Stddev { src, period, .. } => (src, period),
            OpE::Wma { src, period, .. } => (src, period),
            _ => unreachable!(),
        };
        let c = fb.ins().iconst(
            i64_t,
            8 * (plan.pmax.max(period as usize) - period as usize) as i64,
        );
        let vb = fb.ins().iadd(abases[PRICE - src], c);
        vbases.push(vb);
    }
    // E6: a raw `Lag{src,n}` reads its stream `n` bars back off the anchor.
    // Same base form as the group leader (`pmax − n` folded in), so the
    // per-bar read is `lag_base + poff` (the current-bar offset).
    let mut lag_base: Vec<Value> = Vec::with_capacity(nlag);
    for &j in &lags {
        let (src, n) = match plan.ops[j].e {
            OpE::Lag { src, n } => (src, n),
            _ => unreachable!(),
        };
        let c = fb
            .ins()
            .iconst(i64_t, 8 * (plan.pmax.max(n as usize) - n as usize) as i64);
        lag_base.push(fb.ins().iadd(abases[PRICE - src], c));
    }
    // The `ivar < n` fold value is the stream's FIRST sample (global
    // `price[0]`), NOT `0.0`: `Atr` folds its `Wilders` seed from bar 0 and
    // needs `tr[0] = high[0] − low[0]`, which falls straight out of
    // `max(h−l, |h−price[0]|, |l−price[0]|)` because `l0 <= c0 <= h0` — the
    // hand's bar-0 term. Addressed through the anchor at
    // `lag_base + 8·(n − next_bar)` (= `index_of(0)`), so it is right cold
    // AND after a roll (a resumed chunk whose first bar is still `< n`).
    let nb8 = fb.ins().imul(next_bar, eight);
    let mut lag_clamp: Vec<Value> = Vec::with_capacity(nlag);
    for (k, &j) in lags.iter().enumerate() {
        let n = match plan.ops[j].e {
            OpE::Lag { n, .. } => n,
            _ => unreachable!(),
        };
        let off = fb.ins().iconst(i64_t, 8 * n as i64);
        let sub = fb.ins().isub(off, nb8);
        let addr = fb.ins().iadd(lag_base[k], sub);
        lag_clamp.push(fb.ins().load(f64_t, MemFlagsData::new(), addr, 0));
    }
    // E3 stage 3: HMA's two PRIVATE engines anchor-read at P and P/2;
    // rbases is the state-image DIFF ring's base address (the fill's
    // `first_diff` is read from ring[0] there, in-loop).
    let mut hma_vbase: Vec<[usize; 2]> = Vec::with_capacity(nh);
    let mut rbases: Vec<Value> = Vec::with_capacity(nh);
    for &j in &hmas {
        let (src, period, p2) = match plan.ops[j].e {
            OpE::Hma {
                src, period, p2, ..
            } => (src, period, p2),
            _ => unreachable!(),
        };
        macro_rules! hma_vb {
            ($per:expr) => {{
                let c = fb
                    .ins()
                    .iconst(i64_t, 8 * (plan.pmax.max($per) - $per) as i64);
                fb.ins().iadd(abases[PRICE - src], c)
            }};
        }
        let v1 = hma_vb!(period as usize);
        let v2 = hma_vb!(p2 as usize);
        let i1 = vbases.len();
        vbases.push(v1);
        vbases.push(v2);
        hma_vbase.push([i1, i1 + 1]);
        let base = plan.ops[j].aux_word as i64;
        let rbc = fb.ins().iconst(i64_t, 8 * (base + 7));
        let rb = fb.ins().iadd(state, rbc);
        rbases.push(rb);
    }
    fb.ins().jump(head_bb[0], &ba(&args));

    // ---- segment loops ----
    for (s, (b, _e)) in segs.iter().enumerate() {
        let b = *b as i64;
        let is_last = s + 1 == segs.len();
        let succ = if is_last { end_bb } else { head_bb[s + 1] };

        fb.switch_to_block(head_bb[s]);
        let hp: Vec<Value> = fb.block_params(head_bb[s]).to_vec();
        let done = fb
            .ins()
            .icmp(IntCC::SignedGreaterThanOrEqual, hp[ipos], hi_v[s]);
        if is_last {
            // everything but `i` (which end_bb reads as the SEEDED flag —
            // the last head param becomes the seed word on the exit edge)
            let mut end_args = hp[..ipos].to_vec();
            end_args.push(one);
            fb.ins()
                .brif(done, end_bb, &ba(&end_args), body_bb[s], &ba(&hp));
        } else {
            fb.ins().brif(done, succ, &ba(&hp), body_bb[s], &ba(&hp));
        }

        fb.switch_to_block(body_bb[s]);
        let bp: Vec<Value> = fb.block_params(body_bb[s]).to_vec();
        let ivar = bp[ipos];
        let roff = fb.ins().isub(ivar, next_bar);
        let poff = fb.ins().imul(roff, eight);
        // this bar's price, per stream (one load per asset — the same work
        // the A = 1 plan did with its single pointer)
        let mut pv: Vec<Value> = Vec::with_capacity(na);
        for a in 0..na {
            let paddr = fb.ins().iadd(pbase[a], poff);
            pv.push(fb.ins().load(f64_t, MemFlagsData::new(), paddr, 0));
        }

        // E3 anchor reads (price-sourced rolls only — asserted at build).
        // Warm (fold) bars use a literal 0 prev — the hand init fold, and
        // bars < 0 are never addressed through the anchor anyway.
        macro_rules! roll_prev {
            ($j:expr, $src:expr) => {{
                if b <= plan.ops[$j].lead as i64 {
                    fb.ins().f64const(0.0)
                } else {
                    let g = roll_of[$j];
                    let addr = fb.ins().iadd(vbases[g], poff);
                    fb.ins().load(f64_t, MemFlagsData::new(), addr, 0)
                }
            }};
        }
        // E3 rolling groups: the leader op emits the ring access + sum
        // roll once per bar; the sibling kind reads the same three lines
        // (one authority per (src, period) — a sma+stddev+bbands query
        // advances ONE sum, ONE ring for all of them).
        let mut grp: Vec<Option<(Value, Value, Value)>> = vec![None; nops];
        // packed groups: ONE vector step per group. Lane values are NOT
        // extracted here — `scalarize` materializes an extract only where
        // a scalar consumer (sink store, arith operand, chained seed)
        // actually needs one; vector subs consume whole group vectors.
        // The group's price vector depends only on its lane-asset pattern
        // (e.g. (0,0) = plain broadcast, (0,1) = cross-asset E2 pair), so
        // identical patterns SHARE one packed value per bar.
        let mut stepv: Vec<Value> = Vec::with_capacity(ngroups);
        let mut price_packs: Vec<(Vec<usize>, Value)> = Vec::with_capacity(ngroups);
        for g in 0..ngroups {
            let ty = vec_ty(plan.groups[g].ops.len());
            let assets: Vec<usize> = plan.groups[g]
                .ops
                .iter()
                .map(|&o| {
                    PRICE
                        - match plan.ops[o].e {
                            OpE::Ema { src, .. } => src,
                            _ => unreachable!(),
                        }
                })
                .collect();
            if let Some((_, v)) = price_packs.iter().find(|(k, _)| *k == assets) {
                let v = *v;
                let prev = bp[g];
                let prod = fb.ins().fmul(v, gmvec[g]);
                stepv.push(fb.ins().fma(prev, givec[g], prod));
                continue;
            }
            let mut it = assets.iter().enumerate();
            let (_, &a0) = it.next().unwrap();
            let mut v = fb.ins().splat(ty, pv[a0]);
            for (l, &a) in it {
                // same-asset lanes are already broadcast by the splat
                if a != a0 {
                    v = fb.ins().insertlane(v, pv[a], l as u8);
                }
            }
            price_packs.push((assets, v));
            let prev = bp[g];
            let prod = fb.ins().fmul(v, gmvec[g]);
            stepv.push(fb.ins().fma(prev, givec[g], prod));
        }
        // lazily-materialized scalar views (Eager values: ungrouped Ema /
        // arith / const; grouped chains + aliased subs start None)
        let mut cur: Vec<Option<Value>> = vec![None; nops];
        // E3: rolling ops carry their ACCUMULATOR to the backedge (the
        // published value lives in `cur`); stddev's sum_sq and the ring
        // cursors ride dedicated param blocks.
        let mut carry: Vec<Option<Value>> = vec![None; nops];
        let mut sq_lat: Vec<Option<Value>> = vec![None; nsq];
        // E3 stage 3: HMA's six accumulators + ring cursor are carried
        // by EVERY segment arm (each phase updates a subset, but the
        // param run must stay uniform across phase transitions).
        let mut h_lat: Vec<Option<Value>> = vec![None; HMA_F64_PARAMS * nh];
        let mut c_lat: Vec<Option<Value>> = vec![None; nh];
        // E7: supertrend's three band states + trend latch, likewise carried
        // uniformly across every segment arm.
        let mut s_lat: Vec<Option<Value>> = vec![None; SUP_F64_PARAMS * nsup];
        let mut t_lat: Vec<Option<Value>> = vec![None; nsup];
        // E9: each trail-rescan roll's `trail` (i64), carried by every
        // segment arm like the HMA/supertrend runs. `trail_cur` is that same
        // per-bar value keyed by ROLL OP ID, for `OpE::Trail` consumers.
        let mut tr_lat: Vec<Option<Value>> = vec![None; ntr];
        let mut trail_cur: Vec<Option<Value>> = vec![None; nops];
        // pair-vector results (vec Arith ops): consumed by lane via `alias`
        let mut vvec: Vec<Option<Value>> = vec![None; nops];

        // Resolve any op id to its scalar value for this bar, extracting a
        // vector lane only now (E2-lite's lazy split). SSA order makes the
        // memoized fill safe: producers precede consumers.
        macro_rules! scalarize {
            ($o:expr) => {{
                let o = $o;
                if o > PRICE - na {
                    // E6: a raw price-stream operand (high/low/close of
                    // this asset) — read the bar's value, not an op slot.
                    // Multi-input arith (medprice/tr) feeds streams directly;
                    // classic never does (its arith operands are chains).
                    pv[PRICE - o]
                } else if let Some(v) = cur[o] {
                    v
                } else {
                    let (tgt, lane) = plan.alias[o];
                    let (vecv, l) = if tgt != usize::MAX {
                        (vvec[tgt].expect("pair-vector produced first"), lane)
                    } else if plan.lane_of[o].0 != usize::MAX {
                        (stepv[plan.lane_of[o].0], plan.lane_of[o].1 as u8)
                    } else {
                        (
                            vvec[o].expect("scalarize: value-producing op computed first"),
                            0,
                        )
                    };
                    let v = fb.ins().extractlane(vecv, l);
                    cur[o] = Some(v);
                    v
                }
            }};
        }
        // E9 TWO-PHASE emission order: ROLLS FIRST (sorted by stream, direction,
        // ascending `look_back`), then every other op in arena order. A roll reads
        // only its price stream and its own carried state — never another op — so
        // hoisting them breaks no dependency, while guaranteeing each roll's SEED
        // is already computed no matter what order the caller listed requests in
        // (previously a wide roll listed first found no earlier narrower partner
        // and walked its whole window). Everything else the loop looks up is
        // keyed by op id (`slot_of_op`, `roll_of`, `hma_of`, `sup_of`, `lag_of`,
        // `tr_of`), and `ci` advances only for chained `Ema`s, which stay in
        // phase B — so no other bookkeeping moves. Grouped lane members are
        // already stepped above, hence the existing `lane_of` skip.
        let is_roll = |j: usize| matches!(plan.ops[j].e, OpE::Roll { .. });
        let mut roll_order = mmrolls.clone();
        roll_order.sort_by_key(|&j| match plan.ops[j].e {
            OpE::Roll {
                src,
                want_max,
                look_back,
                ..
            } => (src, !want_max, look_back),
            _ => unreachable!(),
        });
        let rest = (0..nops).filter(|j| !is_roll(*j));
        let mut sd: Vec<Value> = Vec::with_capacity(nsd);
        let mut ci = 0usize;
        for j in roll_order.into_iter().chain(rest) {
            if plan.lane_of[j].0 != usize::MAX {
                continue;
            }
            if plan.alias[j].0 != usize::MAX {
                // absorbed pair-sub: its value IS a lane of another op's
                // pair-vector; `scalarize` extracts there — compute nothing
                // (and leave `cur[j]` None so consumers take the alias).
                continue;
            }
            let newv = match plan.ops[j].e {
                OpE::Ema {
                    src,
                    mult,
                    inv,
                    feed,
                    chained,
                } => {
                    let slot = slot_of_op[j];
                    let prev = bp[slot];
                    macro_rules! step_ema {
                        () => {{
                            let src_v = if src > PRICE - na {
                                pv[PRICE - src]
                            } else {
                                scalarize!(src)
                            };
                            let m = fb.ins().f64const(mult);
                            let iv = fb.ins().f64const(inv);
                            let prod = fb.ins().fmul(src_v, m);
                            fb.ins().fma(prev, iv, prod)
                        }};
                    }
                    if !chained {
                        step_ema!()
                    } else {
                        let sdj = bp[sdpos + ci];
                        ci += 1;
                        if b < feed as i64 {
                            prev // source reports no bars of this segment
                        } else if b == feed as i64 {
                            // feeding starts here: first bar seeds
                            let stepped = step_ema!();
                            let is_sd = fb.ins().icmp(IntCC::NotEqual, sdj, zero);
                            let src_v = if src > PRICE - na {
                                pv[PRICE - src]
                            } else {
                                scalarize!(src)
                            };
                            fb.ins().select(is_sd, stepped, src_v)
                        } else {
                            step_ema!() // sd proven 1: pure step
                        }
                    }
                }
                OpE::Wilders {
                    src,
                    mult,
                    inv,
                    period,
                    composite,
                    feed,
                    ..
                } => {
                    let slot = slot_of_op[j];
                    let prev = bp[slot];
                    // Chained (derived) sources read the same-bar node value;
                    // price sources read this bar's stream (same test as
                    // `step_ema!`).
                    let src_v = if src > PRICE - na {
                        pv[PRICE - src]
                    } else {
                        scalarize!(src)
                    };
                    let lead = plan.ops[j].lead as i64;
                    if b <= lead {
                        // A `feed` chain (`adx` over `dx`) ignores its source
                        // until the source publishes (bars `< feed` add 0), so
                        // the `period`-wide fold window slides to
                        // `[feed, feed+period-1]`. `feed == 0` folds from the
                        // head — no gate emitted, byte-identical to `atr`.
                        let addend = if feed == 0 {
                            src_v
                        } else {
                            let fc = fb.ins().iconst(i64_t, feed as i64);
                            let early = fb.ins().icmp(IntCC::SignedLessThan, ivar, fc);
                            let z = fb.ins().f64const(0.0);
                            fb.ins().select(early, z, src_v)
                        };
                        // fold bars: running sum. For MEAN mode the LAST fold
                        // bar (`ivar == lead`) divides once to seed the mean
                        // (hand `init_state`'s `sum/period`); for COMPOSITE
                        // mode the raw sum IS the seed (hand `partial_calc`,
                        // the DI/DX running sum) so there is no divide.
                        let sum = fb.ins().fadd(prev, addend);
                        let carried = if composite {
                            sum
                        } else {
                            let seam_c = fb.ins().iconst(i64_t, lead);
                            let at_seam = fb.ins().icmp(IntCC::Equal, ivar, seam_c);
                            let pf = fb.ins().f64const(period as f64);
                            let seeded = fb.ins().fdiv(sum, pf);
                            fb.ins().select(at_seam, seeded, sum)
                        };
                        carry[j] = Some(carried);
                        // The op's VALUE on a fold bar is the running warmup
                        // accumulator, NOT `0.0`: a chained consumer that reads
                        // the source at ITS fold bars (`dx` at `dmup`'s seam =
                        // the hand's `calc_dx()` seed for `adx`) needs the real
                        // partial sum. Harmless for a chain that publishes past
                        // its own seam (standalone wilders/atr never store a
                        // fold bar), and it fixes every `+DM·100/ATR_sum` warmup
                        // consumer (di/dx/adx optional ATR mirrors too).
                        carried
                    } else {
                        // steady: MEAN = `w·mult + v·inv` (hand `calc`);
                        // COMPOSITE = `w·mult + v` (hand `partial_calc`, no
                        // `·(1/p)` — the accumulator keeps growing like a sum).
                        let m = fb.ins().f64const(mult);
                        let prod = if composite {
                            src_v
                        } else {
                            let iv = fb.ins().f64const(inv);
                            fb.ins().fmul(src_v, iv)
                        };
                        fb.ins().fma(prev, m, prod)
                    }
                }
                OpE::Arith { op, a, b, vec } => {
                    if vec {
                        // E2-lite pair-sub: a/b are GROUP ids; one fsub on
                        // THIS bar's stepped lane vectors yields both line
                        // values (`bp` holds the previous bar).
                        let v = fb.ins().fsub(stepv[a], stepv[b]);
                        vvec[j] = Some(v);
                        cur[j] = Some(fb.ins().extractlane(v, 0));
                        continue;
                    }
                    let av = scalarize!(a);
                    let bv = scalarize!(b);
                    match op {
                        ArithKind::Sub => fb.ins().fsub(av, bv),
                        ArithKind::Add => fb.ins().fadd(av, bv),
                        ArithKind::Mul => fb.ins().fmul(av, bv),
                        ArithKind::Div => fb.ins().fdiv(av, bv),
                        // Rust `f64::max`, NOT CLIF `fmax` (NaN-propagating) —
                        // see `max_num`. di/dx guard a possible `0·inf`/`0/0`
                        // NaN with `.max(0.0)`.
                        ArithKind::Max => max_num(&mut fb, av, bv),
                    }
                }
                OpE::Wma {
                    src,
                    period: _,
                    w,
                    wr,
                } => {
                    let leader = plan.ops[j].roll_leader;
                    let ax = sq_of[j];
                    let ws = bp[auxpos + ax];
                    // the trick needs the RAW sum, not the SMA line — the
                    // ×1/P multiply happens only in arms that consume it
                    let sum_old = bp[slot_of_op[leader]];
                    let (src_v, prevv, sum2) = if leader == j {
                        let sum = bp[slot_of_op[j]];
                        let src_v = pv[PRICE - src];
                        let prevv = roll_prev!(j, src);
                        let delta = fb.ins().fsub(src_v, prevv);
                        let sum2 = fb.ins().fadd(sum, delta);
                        carry[j] = Some(sum2);
                        grp[j] = Some((src_v, prevv, sum2));
                        (src_v, prevv, sum2)
                    } else {
                        grp[leader].expect("rolling leader precedes follower")
                    };
                    let _ = (prevv, sum2);
                    // warm: `ws += src·(i+1)` (init_state fold); steady:
                    // the hand's calc, `ws −= sum_old` before `ws += src·P`
                    let ws2 = if b <= plan.ops[j].lead as i64 {
                        let k1 = fb.ins().iadd(ivar, one);
                        let kf = fb.ins().fcvt_from_sint(f64_t, k1);
                        let term = fb.ins().fmul(src_v, kf);
                        fb.ins().fadd(ws, term)
                    } else {
                        let t0 = fb.ins().fsub(ws, sum_old);
                        let pc = fb.ins().f64const(w);
                        let t1 = fb.ins().fmul(src_v, pc);
                        fb.ins().fadd(t0, t1)
                    };
                    sq_lat[ax] = Some(ws2);
                    let wc = fb.ins().f64const(wr);
                    fb.ins().fmul(ws2, wc)
                }
                OpE::Sma { src, period, mult } => {
                    let leader = plan.ops[j].roll_leader;
                    let (src_v, prevv, sum2) = if leader == j {
                        let sum = bp[slot_of_op[j]];
                        let src_v = pv[PRICE - src];
                        let prevv = roll_prev!(j, src);
                        let delta = fb.ins().fsub(src_v, prevv);
                        let sum2 = fb.ins().fadd(sum, delta);
                        carry[j] = Some(sum2);
                        grp[j] = Some((src_v, prevv, sum2));
                        (src_v, prevv, sum2)
                    } else {
                        grp[leader].expect("rolling leader precedes follower")
                    };
                    let _ = (src_v, prevv, period);
                    let mc = fb.ins().f64const(mult);
                    fb.ins().fmul(sum2, mc)
                }
                OpE::Stddev {
                    src,
                    period: _,
                    mult,
                } => {
                    let leader = plan.ops[j].roll_leader;
                    let sk = sq_of[j];
                    let sq = bp[auxpos + sk];
                    let mc = fb.ins().f64const(mult);
                    let (src_v, prevv, sum2) = if leader == j {
                        let sum = bp[slot_of_op[j]];
                        let src_v = pv[PRICE - src];
                        let prevv = roll_prev!(j, src);
                        let delta = fb.ins().fsub(src_v, prevv);
                        let sum2 = fb.ins().fadd(sum, delta);
                        carry[j] = Some(sum2);
                        grp[j] = Some((src_v, prevv, sum2));
                        (src_v, prevv, sum2)
                    } else {
                        grp[leader].expect("rolling leader precedes follower")
                    };
                    // warm: fold exactly like the hand's init (`fma(v,v,sq)`
                    // single-rounded); steady: the double-rounded update the
                    // hand's calc uses — never interchangeable (simulated:
                    // 13% first-value bit divergence if mixed up).
                    let sq2 = if b <= plan.ops[j].lead as i64 {
                        fb.ins().fma(src_v, src_v, sq)
                    } else {
                        let pp = fb.ins().fmul(prevv, prevv);
                        let np = fb.ins().fneg(pp);
                        let add = fb.ins().fma(src_v, src_v, np);
                        fb.ins().fadd(sq, add)
                    };
                    let sma_v = fb.ins().fmul(sum2, mc);
                    let inner = fb.ins().fmul(sma_v, sma_v);
                    let neg = fb.ins().fneg(inner);
                    let sd0 = fb.ins().fma(sq2, mc, neg);
                    // `sqrt(sd0).max(EPSILON)` — a negative variance (catastrophic
                    // cancellation) makes `sd0` NaN, and Rust `.max` yields the
                    // EPSILON guard while CLIF `fmax` would propagate the NaN.
                    let sdr = fb.ins().sqrt(sd0);
                    let eps = fb.ins().f64const(f64::EPSILON);
                    sq_lat[sk] = Some(sq2);
                    max_num(&mut fb, sdr, eps)
                }
                OpE::Roll {
                    src,
                    period,
                    look_back,
                    want_max,
                } => {
                    let slot = slot_of_op[j];
                    let prev_val = bp[slot];
                    let src_v = pv[PRICE - src];
                    // `>=` for max, `<=` for min — the hand's else-branch
                    // compares, and the rescan uses the SAME relation so a tie
                    // always lands on the NEWEST occurrence.
                    let cc = if want_max {
                        FloatCC::GreaterThanOrEqual
                    } else {
                        FloatCC::LessThanOrEqual
                    };
                    let tk = tr_of[j];
                    let nt = fb.ins().iadd(bp[rpos + tk], one);
                    let hit = fb.ins().fcmp(cc, src_v, prev_val);
                    let keep_val = fb.ins().select(hit, src_v, prev_val);
                    let keep_trail = fb.ins().select(hit, zero, nt);
                    if b < period as i64 {
                        // No rescan is reachable in this segment: the trail
                        // counts at most `b` bars, and the guard needs
                        // `period <= trail`. So the else-branch IS the whole
                        // bar — zero extra code over the guard-free shape.
                        tr_lat[tk] = Some(keep_trail);
                        trail_cur[j] = Some(keep_trail);
                        keep_val
                    } else {
                        // Guarded segment. The rescan goes out on a side exit
                        // so an ORDINARY bar keeps its compare + two selects +
                        // one brif; only the rare expired-max bar walks the
                        // window. `merge` re-joins the two paths carrying
                        // (value, trail) as block params, and becomes the
                        // block every later op of this bar emits into.
                        let rescan_bb = fb.create_block();
                        let merge_bb = fb.create_block();
                        add_params(&mut fb, merge_bb, &[f64_t, i64_t]);
                        let pc = fb.ins().iconst(i64_t, period as i64);
                        let due = fb.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, nt, pc);
                        fb.ins().brif(
                            due,
                            rescan_bb,
                            &ba(&[]),
                            merge_bb,
                            &ba(&[keep_val, keep_trail]),
                        );
                        fb.switch_to_block(rescan_bb);
                        // window[k] = bar `i − look_back + k`, so slot k's address is
                        // this bar's anchor address minus `8·(look_back − k)` — one
                        // `iadd` per slot, no cursor, no ring store.
                        let this_bar = fb.ins().iadd(mmroll_base[tk], poff);
                        let back = fb.ins().iconst(i64_t, -8 * look_back as i64);
                        let w0 = fb.ins().iadd(this_bar, back);
                        let lbc = fb.ins().iconst(i64_t, look_back as i64);
                        // ONE dependent chain, OLDEST slot → NEWEST slot: load, `fcmp`,
                        // two `select`s (value and slot), next slot. Walking in age
                        // order with the NON-STRICT compare is what gives the hand's
                        // newest-occurrence rule — its `find_max_scalar` uses `>=` "to
                        // get last position" for the same reason — and it is also what
                        // the register allocator likes: the same walk done in the OTHER
                        // order (newest → oldest, seeded with a running (value, trail)
                        // pair, comparing STRICTLY so the order does the tie-breaking)
                        // emits one FEWER load, one FEWER `isub`, and is bit-exact the
                        // same, yet measured ~1 cycle per step SLOWER: +36% on a
                        // standalone `max(20)` (12.73 → 17.29 µs, hand rows within 1%
                        // of the reference run), +30% on the 2- and 3-link plans. Whether
                        // the chain started from the `src_v` register or from a load of
                        // the newest slot made no difference (18.39 vs 17.29), so it is
                        // the direction, not the seed. Handover §6.13.
                        macro_rules! walk {
                            ($base:expr, $n:expr, $nonstrict:expr) => {{
                                let n: i64 = $n;
                                let base: Value = $base;
                                let mut bv = fb.ins().load(f64_t, MemFlagsData::new(), base, 0);
                                let mut bi = zero;
                                let mut k: i64 = 1;
                                while k < n {
                                    let off = fb.ins().iconst(i64_t, 8 * k);
                                    let a = fb.ins().iadd(base, off);
                                    let v = fb.ins().load(f64_t, MemFlagsData::new(), a, 0);
                                    let c = fb.ins().fcmp($nonstrict, v, bv);
                                    bv = fb.ins().select(c, v, bv);
                                    let kc = fb.ins().iconst(i64_t, k);
                                    bi = fb.ins().select(c, kc, bi);
                                    k += 1;
                                }
                                (bv, bi)
                            }};
                        }
                        // E11 experiment: the same walk, TWO SLOTS PER ITERATION.
                        // One vector load, one `CMPPD`, one `vany_true` and a branch
                        // that is rarely taken; all the scalar bookkeeping (the new
                        // extremum, which lane it came from) happens inside that
                        // branch, and the winning LANE is resolved once after the loop
                        // — which is why no vector select is needed (cranelift 0.136
                        // has none), and why the running value/index pair stays a
                        // single (scalar, scalar) like the hand's `find_max_simd`.
                        //
                        // The improvement test is `UGE` with the operands swapped for
                        // min (`chunk ≤ gmin` ≡ `gmin ≥ chunk`), because x64 lowers the
                        // unordered-or-greater forms and NOT `ULE`.
                        //
                        // NaN is the one place the two walks are NOT interchangeable:
                        // `fmax`/`fmin` drop a NaN operand, while the scalar chain's
                        // `fcmp(ge)`/`fcmp(le)` returns false on NaN and so keeps the
                        // previous best. This matches the engine's standing position on
                        // price data (see `max_num`: bit-identical for finite prices) and
                        // the hand's own scalar-vs-SIMD pair already differs the same
                        // way, but it is why a NaN-tolerant dataset would need the
                        // scalar walk (`CSE_SCALARWALK=1`) to stay hand-exact.
                        macro_rules! walkv {
                            ($base:expr, $n:expr, $nonstrict:expr, $is_max:expr) => {{
                                let n: i64 = $n;
                                let base: Value = $base;
                                let is_max: bool = $is_max;
                                let vty = vec_ty(2);
                                // Slot 0 seeds the running extremum, exactly as the
                                // scalar chain does, so a window nothing later improves
                                // still answers (slot 0, that slot's value).
                                let v0 = fb.ins().load(f64_t, MemFlagsData::new(), base, 0);
                                let mut gv = fb.ins().splat(vty, v0);
                                let mut bc = gv;
                                let neg1 = fb.ins().iconst(i64_t, -1);
                                let mut bs = neg1;
                                // Chunks of two over slots 1..=n-1. At most ONE slot
                                // is left over, so there is no tail loop — the scalar
                                // compare after the loop is the whole remainder path.
                                let nchunks = (n - 1) / 2;
                                for j in 0..nchunks {
                                    let up_bb = fb.create_block();
                                    let mrg_bb = fb.create_block();
                                    add_params(&mut fb, mrg_bb, &[vty, vty, i64_t]);
                                    let off = fb.ins().iconst(i64_t, 8 * (1 + 2 * j));
                                    let a = fb.ins().iadd(base, off);
                                    let chunk = fb.ins().load(vty, MemFlagsData::new(), a, 0);
                                    let (ca, cb) = if is_max { (chunk, gv) } else { (gv, chunk) };
                                    let chg = fb.ins().fcmp(
                                        FloatCC::UnorderedOrGreaterThanOrEqual,
                                        ca,
                                        cb,
                                    );
                                    let any = fb.ins().vany_true(chg);
                                    fb.ins()
                                        .brif(any, up_bb, &ba(&[]), mrg_bb, &ba(&[gv, bc, bs]));
                                    fb.switch_to_block(up_bb);
                                    let l0 = fb.ins().extractlane(chunk, 0);
                                    let l1 = fb.ins().extractlane(chunk, 1);
                                    let nm = if is_max {
                                        fb.ins().fmax(l0, l1)
                                    } else {
                                        fb.ins().fmin(l0, l1)
                                    };
                                    let ng = fb.ins().splat(vty, nm);
                                    let sj = fb.ins().iconst(i64_t, 1 + 2 * j);
                                    fb.ins().jump(mrg_bb, &ba(&[ng, chunk, sj]));
                                    fb.switch_to_block(mrg_bb);
                                    let p = fb.block_params(mrg_bb).to_vec();
                                    gv = p[0];
                                    bc = p[1];
                                    bs = p[2];
                                }
                                let mut gmax = fb.ins().extractlane(gv, 0);
                                // Ties go to the NEWER lane (the hand's
                                // `simd_eq(global_max).to_bitmask().ilog2()` at width 2).
                                let e1 = fb.ins().extractlane(bc, 1);
                                let newer = fb.ins().fcmp(FloatCC::Equal, e1, gmax);
                                let add1 = fb.ins().select(newer, one, zero);
                                let mut bi = fb.ins().iadd(bs, add1);
                                // Sentinel: no chunk ever improved on slot 0, so the
                                // answer is slot 0 (and any later EQUAL value would have
                                // taken the rare path, which is what makes that sound).
                                let sent = fb.ins().icmp(IntCC::Equal, bs, neg1);
                                bi = fb.ins().select(sent, zero, bi);
                                let rem = 1 + 2 * nchunks;
                                if rem < n {
                                    let off = fb.ins().iconst(i64_t, 8 * rem);
                                    let a = fb.ins().iadd(base, off);
                                    let v = fb.ins().load(f64_t, MemFlagsData::new(), a, 0);
                                    let hit = fb.ins().fcmp($nonstrict, v, gmax);
                                    gmax = fb.ins().select(hit, v, gmax);
                                    let rc = fb.ins().iconst(i64_t, rem);
                                    bi = fb.ins().select(hit, rc, bi);
                                }
                                (gmax, bi)
                            }};
                        }
                        let seed = plan.ops[j].seed;
                        // The walk yields a WINDOW slot, so the trail is
                        // `look_back − slot` — the hand's
                        // `trail = i − (search_start + max_idx)`, which is the same
                        // thing because `search_start = i − look_back`. Convert to the
                        // TRAIL inside each branch rather than carrying a slot: the
                        // seeded branch has no slot to carry (the seed's own bar is
                        // `look_back − trail_n` in this roll's coordinates and the narrow
                        // roll publishes no index), so trail is the only unit both
                        // branches agree on.
                        let (bv, bt) = if seed == usize::MAX {
                            let (v, i) = if plan.vecwalk {
                                walkv!(w0, look_back as i64 + 1, cc, want_max)
                            } else {
                                walk!(w0, look_back as i64 + 1, cc)
                            };
                            (v, fb.ins().isub(lbc, i))
                        } else {
                            // SEEDED walk: the narrower roll (same stream, same
                            // direction, smaller `look_back`) already covers
                            // `[i − lb_n ..= i]`, so scan only the older gap and
                            // compare it against the seed STRICTLY — a gap value EQUAL
                            // to the seed must not win, because the newest occurrence
                            // of that value is the seed's own bar, so the wide roll
                            // inherits both its value and its trail.
                            let lb_n = match plan.ops[seed].e {
                                OpE::Roll { look_back, .. } => look_back,
                                _ => unreachable!(),
                            };
                            let sv = scalarize!(seed);
                            let st = trail_cur[seed]
                                .expect("jit: a seed roll's trail comes with its value");
                            let gap = (look_back - lb_n) as i64;
                            let (gv, gi) = if plan.vecwalk {
                                walkv!(w0, gap, cc, want_max)
                            } else {
                                walk!(w0, gap, cc)
                            };
                            let gap_trail = fb.ins().isub(lbc, gi);
                            let strict = if want_max {
                                fb.ins().fcmp(FloatCC::GreaterThan, gv, sv)
                            } else {
                                fb.ins().fcmp(FloatCC::LessThan, gv, sv)
                            };
                            (
                                fb.ins().select(strict, gv, sv),
                                fb.ins().select(strict, gap_trail, st),
                            )
                        };
                        fb.ins().jump(merge_bb, &ba(&[bv, bt]));
                        fb.switch_to_block(merge_bb);
                        let mp: Vec<Value> = fb.block_params(merge_bb).to_vec();
                        tr_lat[tk] = Some(mp[1]);
                        trail_cur[j] = Some(mp[1]);
                        mp[0]
                    }
                }
                // E9: project a roll's per-bar trail (integer bar count → f64,
                // exactly the hand's `trail as f64`).
                OpE::Trail { roll } => {
                    let t = trail_cur[roll].expect("jit: Trail's roll is emitted first");
                    fb.ins().fcvt_from_uint(f64_t, t)
                }
                OpE::Hma {
                    src,
                    period,
                    p2,
                    psqrt,
                    wc1,
                    wr1,
                    wc2,
                    wr2,
                    psqrf,
                    wssq,
                } => {
                    let k = hma_of[j];
                    let hb = hpos + HMA_F64_PARAMS * k;
                    let s1 = bp[hb];
                    let ws1 = bp[hb + 1];
                    let s2 = bp[hb + 2];
                    let ws2 = bp[hb + 3];
                    let wss = bp[hb + 4];
                    let ss = bp[hb + 5];
                    let curs = bp[cpos + k];
                    let src_v = pv[PRICE - src];
                    let fold2 = period as i64 - p2 as i64;
                    let seam = period as i64 + psqrt as i64;
                    // the hand's two private WMA engines, one bar of
                    // FOLD (init_state's loop body) — shared by F1/F12
                    macro_rules! hma_fold1 {
                        () => {{
                            let k1 = fb.ins().iadd(ivar, one);
                            let kf = fb.ins().fcvt_from_sint(f64_t, k1);
                            let term = fb.ins().fmul(src_v, kf);
                            let t1 = fb.ins().fadd(ws1, term);
                            let s1p = fb.ins().fadd(s1, src_v);
                            (s1p, t1)
                        }};
                    }
                    // … and one bar of TRICK (hand `WMAState::calc`, exact
                    // order: `ws −= sum_old` before `sum += src − prev`,
                    // `ws += src·P` after, `wma = ws·2/(P(P+1))`)
                    macro_rules! hma_trick {
                        ($s:expr, $ws:expr, $vb:expr, $wc:expr, $wr:expr) => {{
                            let addr = fb.ins().iadd($vb, poff);
                            let prev = fb.ins().load(f64_t, MemFlagsData::new(), addr, 0);
                            let wc = fb.ins().f64const($wc);
                            let wr = fb.ins().f64const($wr);
                            let t0 = fb.ins().fsub($ws, $s);
                            let t1 = fb.ins().fmul(src_v, wc);
                            let ws2 = fb.ins().fadd(t0, t1);
                            let delta = fb.ins().fsub(src_v, prev);
                            let s2 = fb.ins().fadd($s, delta);
                            let wma = fb.ins().fmul(ws2, wr);
                            (s2, ws2, wma)
                        }};
                    }
                    macro_rules! hma_carry {
                        ($vals:expr, $curs:expr) => {{
                            let vals: [Value; HMA_F64_PARAMS] = $vals;
                            for (m, v) in vals.into_iter().enumerate() {
                                h_lat[HMA_F64_PARAMS * k + m] = Some(v);
                            }
                            c_lat[k] = Some($curs);
                        }};
                    }
                    macro_rules! hma_next_curs {
                        () => {{
                            // `curs` carries the ring BYTE OFFSET (not an
                            // index): +8, wrap to 0 at `8·psqrt`. Kills the
                            // per-bar `imul` — the slot addr is a plain
                            // `add(rbases, curs)`. The word persists the
                            // offset (relative, so safe if `state` moves).
                            let ringend = fb.ins().iconst(i64_t, 8 * psqrt as i64);
                            let c1 = fb.ins().iadd(curs, eight);
                            let wrap = fb.ins().icmp(IntCC::Equal, c1, ringend);
                            fb.ins().select(wrap, zero, c1)
                        }};
                    }
                    if b < fold2 {
                        // F1: engine 1 folds, everything else frozen
                        let (s1p, t1) = hma_fold1!();
                        hma_carry!([s1p, t1, s2, ws2, wss, ss], curs);
                        fb.ins().f64const(0.0)
                    } else if b < period as i64 {
                        // F12: both fold; engine 2's weights restart at
                        // its OFFSET window (bar P−P/2 carries weight 1)
                        let (s1p, t1) = hma_fold1!();
                        let off = fb.ins().iconst(i64_t, 1 - fold2);
                        let k2 = fb.ins().iadd(ivar, off);
                        let kf2 = fb.ins().fcvt_from_sint(f64_t, k2);
                        let term2 = fb.ins().fmul(src_v, kf2);
                        let t2 = fb.ins().fadd(ws2, term2);
                        let s2p = fb.ins().fadd(s2, src_v);
                        hma_carry!([s1p, t1, s2p, t2, wss, ss], curs);
                        fb.ins().f64const(0.0)
                    } else if b < seam {
                        // FILL: both engines trick (anchor prevs are live
                        // from bar P on), diff accumulates, the ring
                        // pushes — the hand's fill body, no output, no
                        // pop, and `wss −= ss` every fill bar. The LAST
                        // fill bar additionally applies the hand's
                        // post-loop `sumsqrt −= first_diff` correction
                        // (ring[0] still holds it; at psqrt == 1 the only
                        // fill diff IS first_diff) — carried in `ss`, so
                        // a chunk that resumes at the seam needs none.
                        let (v1, v2) = (vbases[hma_vbase[k][0]], vbases[hma_vbase[k][1]]);
                        let (s1p, t1, wma1) = hma_trick!(s1, ws1, v1, wc1, wr1);
                        let (s2p, t2, wma2) = hma_trick!(s2, ws2, v2, wc2, wr2);
                        let two = fb.ins().f64const(2.0);
                        let d1 = fb.ins().fmul(wma2, two);
                        let diff = fb.ins().fsub(d1, wma1);
                        let pc = fb.ins().f64const(psqrf);
                        let d2 = fb.ins().fmul(diff, pc);
                        let wssa = fb.ins().fadd(wss, d2);
                        let ssa = fb.ins().fadd(ss, diff);
                        let ra = fb.ins().iadd(rbases[k], curs);
                        let is_lf = fb.ins().iconst(i64_t, seam - 1);
                        let last_fill = fb.ins().icmp(IntCC::Equal, ivar, is_lf);
                        let firstd = if psqrt == 1 {
                            diff
                        } else {
                            ld(&mut fb, rbases[k], 0, f64_t)
                        };
                        let fix = fb.ins().fsub(ssa, firstd);
                        let ssc = fb.ins().select(last_fill, fix, ssa);
                        fb.ins().store(MemFlagsData::new(), diff, ra, 0);
                        let cn = hma_next_curs!();
                        let wssb = fb.ins().fsub(wssa, ssa);
                        hma_carry!([s1p, t1, s2p, t2, wssb, ssc], cn);
                        fb.ins().f64const(0.0)
                    } else {
                        // STEADY: fill body + output + pop. The fill's
                        // one-time correction rode the last FILL bar (in
                        // the carried `ss`) — nothing boundary-specific
                        // happens here at all.
                        let (v1, v2) = (vbases[hma_vbase[k][0]], vbases[hma_vbase[k][1]]);
                        let (s1p, t1, wma1) = hma_trick!(s1, ws1, v1, wc1, wr1);
                        let (s2p, t2, wma2) = hma_trick!(s2, ws2, v2, wc2, wr2);
                        let two = fb.ins().f64const(2.0);
                        let d1 = fb.ins().fmul(wma2, two);
                        let diff = fb.ins().fsub(d1, wma1);
                        let pc = fb.ins().f64const(psqrf);
                        let d2 = fb.ins().fmul(diff, pc);
                        let wssa = fb.ins().fadd(wss, d2);
                        let ssa = fb.ins().fadd(ss, diff);
                        let wc = fb.ins().f64const(wssq);
                        let out = fb.ins().fdiv(wssa, wc);
                        let wssb = fb.ins().fsub(wssa, ssa);
                        // pop = front() AFTER the push: at psqrt ≥ 2 the
                        // NEXT slot (untouched by this bar's push → read
                        // before it); at psqrt == 1 the slot pushed IS
                        // the front → the diff itself.
                        // `curs`/`cn` are ring BYTE OFFSETS now (see
                        // hma_next_curs), so push/pop slots are plain adds.
                        let cn = hma_next_curs!();
                        let old = if psqrt == 1 {
                            diff
                        } else {
                            let ran = fb.ins().iadd(rbases[k], cn);
                            fb.ins().load(f64_t, MemFlagsData::new(), ran, 0)
                        };
                        let ra = fb.ins().iadd(rbases[k], curs);
                        fb.ins().store(MemFlagsData::new(), diff, ra, 0);
                        let ssb = fb.ins().fsub(ssa, old);
                        hma_carry!([s1p, t1, s2p, t2, wssb, ssb], cn);
                        out
                    }
                }
                OpE::Fma { a, b, c } => {
                    let av = scalarize!(a);
                    let bv = scalarize!(b);
                    let cv = scalarize!(c);
                    fb.ins().fma(av, bv, cv)
                }
                OpE::Sqrt { src } => {
                    let v = scalarize!(src);
                    fb.ins().sqrt(v)
                }
                OpE::Abs { src } => {
                    let v = scalarize!(src);
                    fb.ins().fabs(v)
                }
                OpE::WhereGe { a, b, t } => {
                    // `if a >= b { t } else { 0.0 }` — the DM+/DM− mask. The
                    // else arm is a literal `+0.0`, so the losing branch never
                    // yields `-0.0`/NaN (the hand sets it to `0.0` outright).
                    let av = scalarize!(a);
                    let bv = scalarize!(b);
                    let tv = scalarize!(t);
                    let ge = fb.ins().fcmp(FloatCC::GreaterThanOrEqual, av, bv);
                    let z = fb.ins().f64const(0.0);
                    fb.ins().select(ge, tv, z)
                }
                OpE::Lag { n, .. } => {
                    // Anchor read `n` bars back. `ivar < n` (the front bars of
                    // a cold chunk) clamps to the stream's first sample
                    // `price[0]` — the hand's "no prior close" convention, so
                    // `tr[0] = high[0] − low[0]` and `Atr`'s Wilders seed is
                    // bit-exact. Every consumer publishes at lead `>= n`, so a
                    // standalone `tr` never stores a clamped bar.
                    let base = lag_base[lag_of[j]];
                    let addr = fb.ins().iadd(base, poff);
                    let real = fb.ins().load(f64_t, MemFlagsData::new(), addr, 0);
                    let guard = fb.ins().iconst(i64_t, n as i64);
                    let warm = fb.ins().icmp(IntCC::SignedLessThan, ivar, guard);
                    let first = lag_clamp[lag_of[j]];
                    fb.ins().select(warm, first, real)
                }
                OpE::SuperTrend {
                    atr,
                    med,
                    close,
                    step,
                } => {
                    let lead = plan.ops[j].lead as i64;
                    let k = sup_of[j];
                    let sb = SUP_F64_PARAMS * k;
                    if b < lead {
                        // idle before the atr seam — carry the state words
                        // through untouched, publish nothing.
                        s_lat[sb] = Some(bp[spos + sb]);
                        s_lat[sb + 1] = Some(bp[spos + sb + 1]);
                        s_lat[sb + 2] = Some(bp[spos + sb + 2]);
                        t_lat[k] = Some(bp[tpos + k]);
                        fb.ins().f64const(0.0)
                    } else {
                        // `calc_st`, identical at the seed bar (b==lead, prev
                        // all-zero) and every steady bar — mirrors the hand's
                        // State::calc_st exactly.
                        let prev_st = bp[spos + sb];
                        let prev_ub = bp[spos + sb + 1];
                        let prev_lb = bp[spos + sb + 2];
                        let trend0 = fb.ins().icmp(IntCC::NotEqual, bp[tpos + k], zero);
                        let atr_v = scalarize!(atr);
                        let med_v = scalarize!(med);
                        let close_v = scalarize!(close);
                        let stepv = fb.ins().f64const(step);
                        let w = fb.ins().fmul(stepv, atr_v);
                        let ub_raw = fb.ins().fadd(med_v, w);
                        let lb_raw = fb.ins().fsub(med_v, w);
                        let cu = fb.ins().fcmp(FloatCC::GreaterThan, close_v, prev_st);
                        let cd = fb.ins().fcmp(FloatCC::LessThan, close_v, prev_st);
                        let notcd = fb.ins().bnot(cd);
                        let held = fb.ins().band(trend0, notcd);
                        let trend = fb.ins().bor(cu, held);
                        // Band clamps stay raw `fmax`/`fmin` (not `max_num`):
                        // both operands are finite — `med ± step·atr` and the
                        // carried band, over finite OHLC with no division — so
                        // maxNum and the NaN-propagating form are bit-identical
                        // here. The hand-parity bug (§2.1) needs the NaN guard
                        // only where a `0·inf`/`0/0` can appear.
                        let lb_clamp = fb.ins().fmax(prev_lb, lb_raw);
                        let ub_clamp = fb.ins().fmin(prev_ub, ub_raw);
                        let st = fb.ins().select(trend, lb_clamp, ub_clamp);
                        let n_lb = fb.ins().select(trend, lb_clamp, lb_raw);
                        let n_ub = fb.ins().select(trend, ub_raw, ub_clamp);
                        let n_tr = fb.ins().select(trend, one, zero);
                        s_lat[sb] = Some(st);
                        s_lat[sb + 1] = Some(n_ub);
                        s_lat[sb + 2] = Some(n_lb);
                        t_lat[k] = Some(n_tr);
                        st
                    }
                }
                OpE::Const(v) => fb.ins().f64const(v),
            };
            cur[j] = Some(newv);
        }
        // Seed flags for the next edge, in `chained` order.
        for (i, &cj) in chained.iter().enumerate() {
            let feed = match plan.ops[cj].e {
                OpE::Ema { feed, .. } => feed as i64,
                _ => unreachable!(),
            };
            if b < feed {
                sd.push(bp[sdpos + i]);
            } else {
                sd.push(one);
            }
        }

        // E2b: writing sinks store through their carried pointer and hand
        // the +8 bump to the next iteration; non-writing sinks pass their
        // pointer through untouched (compile-time known per segment).
        let mut walked: Vec<Value> = Vec::with_capacity(nsinks);
        for (k, sink) in plan.stores.iter().enumerate() {
            let ptr = bp[ppos + k];
            if sink.lead as i64 > b {
                walked.push(ptr); // not printing during this segment
                continue;
            }
            let v = scalarize!(sink.val);
            fb.ins().store(MemFlagsData::new(), v, ptr, 0);
            walked.push(fb.ins().iadd(ptr, eight));
        }
        let i1 = fb.ins().iadd(ivar, one);
        let mut la: Vec<Value> = Vec::with_capacity(nparam);
        la.extend(stepv.iter().copied());
        for &o in &op_of_slot {
            la.push(carry[o].or(cur[o]).expect("state-carried op has a value"));
        }
        la.extend(sd.iter().copied());
        la.extend(walked.iter().copied());
        la.extend(sq_lat.iter().copied().flatten());
        for m in 0..HMA_F64_PARAMS * nh {
            la.push(h_lat[m].expect("hma accumulators carry every segment"));
        }
        for k in 0..nh {
            la.push(c_lat[k].expect("hma cursors carry every segment"));
        }
        for m in 0..SUP_F64_PARAMS * nsup {
            la.push(s_lat[m].expect("supertrend band states carry every segment"));
        }
        for k in 0..nsup {
            la.push(t_lat[k].expect("supertrend trend latch carries every segment"));
        }
        for k in 0..ntr {
            la.push(tr_lat[k].expect("roll trail carries every segment"));
        }
        la.push(i1);
        fb.ins().jump(head_bb[s], &ba(&la));
    }

    // ---- end: store state once (group vectors extracted back per chain) ----
    fb.switch_to_block(end_bb);
    let endp: Vec<Value> = fb.block_params(end_bb).to_vec();
    for g in 0..ngroups {
        for (l, &o) in plan.groups[g].ops.iter().enumerate() {
            let v = fb.ins().extractlane(endp[g], l as u8);
            st(&mut fb, state, 8 * plan.ops[o].ema_word as i64, v);
        }
    }
    for (k, &o) in sqs.iter().enumerate() {
        st(
            &mut fb,
            state,
            8 * plan.ops[o].aux_word as i64,
            endp[auxpos + k],
        );
    }
    // E3 stage 3: HMA's six accumulators + cursor words (the DIFF ring
    // was written IN the loop body — it lives directly in the state
    // image, no end-block copy).
    for (k, &o) in hmas.iter().enumerate() {
        let base = plan.ops[o].aux_word as i64;
        for m in 0..HMA_F64_PARAMS {
            st(
                &mut fb,
                state,
                8 * (base + m as i64),
                endp[hpos + HMA_F64_PARAMS * k + m],
            );
        }
        st(
            &mut fb,
            state,
            8 * (base + HMA_F64_PARAMS as i64),
            endp[cpos + k],
        );
    }
    // E7: supertrend's three band states + trend latch back to its aux run.
    for (k, &o) in sups.iter().enumerate() {
        let base = plan.ops[o].aux_word as i64;
        for m in 0..SUP_F64_PARAMS {
            st(
                &mut fb,
                state,
                8 * (base + m as i64),
                endp[spos + SUP_F64_PARAMS * k + m],
            );
        }
        st(
            &mut fb,
            state,
            8 * (base + SUP_F64_PARAMS as i64),
            endp[tpos + k],
        );
    }
    // E9: each trail-rescan roll's `trail` count back to its aux word (the
    // extremum itself is persisted by the op-value run below).
    for (k, &o) in mmrolls.iter().enumerate() {
        st(
            &mut fb,
            state,
            8 * plan.ops[o].aux_word as i64,
            endp[rpos + k],
        );
    }
    for (si, &o) in op_of_slot.iter().enumerate() {
        if matches!(
            plan.ops[o].e,
            OpE::Ema { .. }
                | OpE::Sma { .. }
                | OpE::Stddev { .. }
                | OpE::Wma { .. }
                | OpE::Roll { .. }
                | OpE::Wilders { .. }
        ) {
            st(
                &mut fb,
                state,
                8 * plan.ops[o].ema_word as i64,
                endp[ngroups + si],
            );
        }
    }
    for (i, &j) in chained.iter().enumerate() {
        st(
            &mut fb,
            state,
            8 * plan.ops[j].sd_word as i64,
            endp[nval + i],
        );
    }
    st(&mut fb, state, 8 * W_SEED as i64, endp[ipos]);
    let nb = fb.ins().iadd(next_bar, len);
    st(&mut fb, state, 8 * W_NEXT as i64, nb);
    fb.ins().return_(&[]);

    fb.seal_all_blocks();
    fb.finalize(fc);

    plan.clif = ctx.func.to_string();
    module
        .define_function(id, &mut ctx)
        .expect("define_function");
    module.clear_context(&mut ctx);
    module.finalize_definitions().expect("jit finalize");
    plan.func =
        Some(unsafe { std::mem::transmute::<*const u8, JitFn>(module.get_finalized_function(id)) });
}
