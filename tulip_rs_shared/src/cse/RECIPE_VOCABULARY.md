# Recipe Authoring Guide — the CSE node vocabulary

How to add an indicator to the engine: transcribe the hand's `calc` into a `Recipe`
graph, register it, and gate it bit-exact against that hand. The hand module under
`tulip_rs/src/indicators/*.rs` is the **oracle** — never a textbook formula.

Layout: `recipes.rs` holds one `pub fn <name>(options: &[f64]) -> Recipe` per
indicator (leaf data, no indicator math in the engine). `recipe.rs` holds the
`Node`/`Recipe`/`Sink` vocabulary. `merge.rs` interns; `signature.rs` erases to a
structure-only `Signature`; `tulip_rs_macros/src/cse/codegen.rs` emits kernels.

---

## 1. Registering a recipe takes four edits

| # | File | Change |
| --- | --- | --- |
| 1 | `tulip_rs_shared/src/cse/recipes.rs` | `pub fn <name>(options: &[f64]) -> Recipe` |
| 2 | `tulip_rs_macros/src/cse/planner.rs` | dispatch arm `"Name" => recipes::<name>(&opts)` |
| 3 | `tulip_rs/src/cse/kernel/registry.rs` | a `kernel!` row, e.g. `<name>_n: Name(p);` |
| 4 | `tulip_test/tests/cse_kernel_registry_test.rs` | bit-exact gate vs the hand |

Unregistered recipes are library surface only — unreachable from `try_run`, so they
can ship wrong values silently. A registered row without a gate is worse: it IS
reachable. Never stop at step 3.

**Every recipe gets its own bit-exact gate against the hand.** The four edits are one
unit, not a menu, and "bit-exact" means the same bits, not the same to 1e-12: the hand
module is the only oracle. A gate that compares the engine against another kernel, or
against a textbook formula, proves nothing about the transcription. A recipe whose graph
cannot be erased (below) has no kernel to run and therefore cannot be gated — say so in
its doc comment rather than shipping a silent gap.

The arm string must equal the struct name used in the row (`"WcPrice" => wcprice`).
Verify the hand's arity first: `pub const INPUTS`, `pub const OPTIONS`, and
`Info { inputs, options, outputs, optional_outputs }`.

---

## 2. `Recipe` shape (`recipe.rs`)

```rust
pub struct Recipe {
    pub nodes: Vec<Node>,            // SSA order; each entry's operands are LOWER ids
    pub tags: Vec<Option<&'static str>>, // naming only; NEVER an intern key
    pub columns: Vec<Column>,        // REQUIRED for any recipe binding Input
    pub kind: &'static str,          // lowercase, e.g. "typprice"
    pub options: Vec<f64>,
    pub tail_inputs: Vec<Id>,        // carried state, for get_states
    pub outputs: Vec<Sink>,           // primaries, in Info order
    pub optional_outputs: Vec<Sink>,  // in Info order
}
```

API: `add(node) -> Id`, `tag(id, "name")`, `constant(v) -> Id` (interned by value),
`sub(sub, &[u32]) -> Id`, `sub_graph(...) -> Grafted{primary, outputs, optional}`,
`sub_at(sub, &[Id]) -> Id`.
`Recipe::new()` is **deleted**; `merge` panics on an input-binding recipe with no
columns, so a recipe that reads a price cannot opt out.

## 3. Columns — declare exactly what you read

Canonical order `Open, High, Low, Close, Volume`; a column's arena index is its
rank **among the columns present**, so subsets shift left. Slot 0 is NOT a fixed
column: `price()[Close]` → 0, `hl()[High,Low]` → 0,1, `hlc()` → 0,1,2,
`hlcvol()` → 0,1,2,3, `ohlc()` (open=0,high=1,low=2,close=3), `cvol()` → 0,1,
`oc()` → 0,1, `hlvol()` → 0,1,2, `vol()` → 0. `with_columns` is private on purpose — a hand-typed column list is the
table that drifts. Over-declaring adds streams nothing reads.

This is what closes the historical hazard where price recipes bound `Input{0}=close`
and OHLC recipes bound `Input{0}=high`, so a cross-convention co-request interned
close onto **high** while every `SIG_*` check still passed.

## 4. Sinks and the publish lead

`Sink::iface(id)` = lead 0; `Sink::iface_lead(id, lead)`.
`erase` computes `eff = if sink.lead == 0 { node_lead } else { sink.lead.max(node_lead) }`
(`put_sink`, `signature.rs:696-702`), then `max_gate = max(eff)` (`:671`). The emitter sets
`gate = (max_gate + 1).min(n)` and the steady loop runs `[gate, n)` (`codegen.rs:1933`,
`for i in 0..(n - gate)`); `exec` covers `[0, gate)`. One `+ 1`, not two.

Lead = `min_data - 1`, from the trait default `output_length = len - min_data + 1`
(`indicator_types.rs:28`). Do **not** copy a neighbour's lead: derive it from this
indicator's `min_data`/`output_length`, and check whether it overrides
`output_length`. A hand cycle starting at `for i in 0..len` still publishes at
`min_data - 1` — the write loop and the publish offset are different facts.

A node whose `Info` lists optional outputs must mirror them: `calc` returning a
tuple, or `optional_outputs: &["x"]`, means that row exists. Hand optional order is
declaration order.

## 5. Node vocabulary

`Id = u32` (`recipe.rs:30`), so **the type never tells you whether a field is a node id or
a number — the field name does.** `period` is always the NUMBER, never an id, even where
the enum spells the field `Id` (`Ema{src, period}` `recipe.rs:87-89`, `Sma` `:187-189`);
`erase` puts it into `Values::periods` (`signature.rs:359`, `:472`). Writing
`period: r.constant(9.0)` therefore binds a node id where a number belongs and the
compiler stays silent. `RollSum{period,skip,steady_from}` and
`RollMax/RollMin{period,look_back}` are the same kind of number, spelled `u32`.
`composite`/`feed`/`mean` are `bool`; `Const{bits}`, `Iir{a_bits,b_bits}`,
`SuperTrend{step_bits}` are `u64` bit patterns of an `f64` — `constant(v)` does that
conversion for you.

| Node | Meaning | Notes |
| --- | --- | --- |
| `Input{stream}` | one column | needs `columns` |
| `Const` | literal | via `constant(v)` |
| `Add Sub Mul Div` | `a op b` | order is semantics for sub/div |
| `Abs{src}` `Sqrt{src}` | unary | **no `Neg` node** — negate with `Mul(x, -1.0)` (exact, keeps `-0.0`) |
| `Max Min{a,b}` | IEEE maxNum/minNum | **this is `.max(x)`** |
| `Lag{src,n}` | value `n` bars back | `n` is a value, not structure (`Values::lag_n`); `erase` returns `None` if `n > max_gate + 1` |
| `WhereGe WhereGt WhereLe WhereLt{a,b,t}` | select `t`, else 0 | guards only |
| `Ema{src,period}` | `carry*invms + src*mults` | per-bar fma+mul |
| `Wilders{src,period,composite,feed}` | Wilder smoothing | see §7 |
| `Iir` `IirClamped` `Prev` `Fma{a,b,c}` | explicit weights / clamp-in-carry / delayed copy / `a.mul_add(b,c)` | |
| `Sma{src,period}` | rolling mean, **price only** | reads `stream[t-p]` |
| `Wma Stddev` | weighted / sum+sum_sq | own hoists |
| `RollSum{mean,skip,steady_from}` | ring-backed roll, any source | `mean:true` divides by `period` (`signature.rs:598`) — a rolling mean, but still `K_ROLLSUM`, never `Sma` |
| `RollMax RollMin Trail` | extrema + trail | rescans |
| `Accum{a,b}` | `carry = a.mul_add(b, carry)` | unbounded, e.g. AD |
| `Hma SuperTrend` | bespoke machines | internal state |
| `HighPass{input,period}` `SuperSmoother{src,period}` | Ehlers filters | **cannot be erased** — see below |
| `HtInPhase{src}` `HtQuadrature{src}` | Hilbert pair, lead +6 | **cannot be erased** — see below |
| `Ln` | natural log | `Node::Ln` → `K_LN` |
| `Mirror{src}` | zero-op state copy | for `get_states` |
| `RollAge{src,n}` | ring-backed age roll, publishes the raw evicted/age-read value | `n` is a VALUE (capacity), not a node id — same hazard class as `period`; bespoke group-of-one, `K_ROLLAGE`; used by `adxr`/`cvi` |
| `NviRecur{src,prev_close,volume,prev_volume}` `PviRecur{...}` | gated multiplicative prefix product seeded 1000.0 | scalar-only (unpackable), `K_NVI`/`K_PVI` |
| `MeanAbsDev{src,period,sma}` | window rescan mean absolute deviation | `sma` is an explicit shared `RollSum{mean:true}` operand, not an internally recomputed copy; `K_MEANABSDEV`; used by `md`/`cci` |
| `LinregFlow{src,sub,period,weighted}` | publish-BEFORE-evict moment carry: `pub = st + src·mult`, publish `pub`, then `st = pub − v[sub]` | `K_LINREGFLOW=44`; `weighted` selects `mults[k]` (`period` vs 1.0) and is therefore a VALUES fact, not structure — the instances already differ in `b`. Needed because every `RollSum` region folds the leave INTO what it prints while `linreg.rs` evicts after publishing (measured: a roll window breaks 697/698 ramp bars and is silent on flat). Below the seam the fold weight is `min(t, mult)`, which reproduces the hand's bar-index init and its never-reading `real[0]`, so no rescan and no warm branch in the steady body. Scalar-only; used by `linreg` |
| `KvoFlow{high,low,close,volume}` | bespoke 4-state recurrence, outputs the signed volume-force line `vf` | `K_KVOFLOW`; erasable and dispatching (closed D4, 2026-10-25) — reserved state storage `kvacc: [[f64;3];MAX]` (`[prev_hlc, cm, prev_hl]`), separate from `st[k]`'s trend bool; `warmup.rs`'s cold-start arm reproduces the hand's non-zero seed (`initial_trend`/`initial_cm` from bars 0-1), which the generic zero-init path cannot express |
| `SeededEma{seed,src,period}` | custom-seeded EMA: publishes `seed`'s own value at `seed`'s lead bar, then folds `src` (a DIFFERENT node) from the next bar on | `K_SEEDEDEMA=39`; `feeds[k]=leads[seed]` (mirrors `Ema`'s own-`Input`-override rule, applied to `seed`); plain `Ema{src,period}` is the degenerate case `seed==src`. Never lane-packed — `node.a` is the one-shot SEED, not a shared fold source, so `classify`/`classify_packed` always register it `Scalar::Chain(node.b)`. Used by `trvi`: seed=`high-low` range (bar 0), fold=`tr` (bar 1 on, since `tr`'s own `Lag{close,1}` gives it structural lead 1, so a plain `Ema{src:tr,..}` would seed from a real close-aware `tr[1]`, not the hand's bar-0 range) |
| `VariableAlpha{seed,value,mult,feed}` | EMA-style carry whose blend weight is read from a LIVE per-bar node value (`v[mult]`), not a recipe-time constant | `K_VARIABLEALPHA=45`; fold shape selected by `AlphaShape` (`Ema`/`Vidya`), encoded structurally into `Signature::d` (0/1) since it changes the arithmetic, not a magnitude — same principle as `composite`'s `Signature::b` encoding for `Wilders`. Wired end to end (closed, kama/vidya) — recipe, registry row, kernel arms, and a live `assert_bits` gate |
| `Psar{high,low,af_step_bits,max_af_bits}` | Parabolic SAR: bespoke 3-field carry `[psar,extream,accel]` + trend bool, read directly off TWO raw price streams (no graft chain, unlike `SuperTrend`) | `K_PSAR=46`; scalar-only (`lane_formula (0,0)`); bar 1 is a DIFFERENT formula than every bar after (the hand's `State::new`/`calc` at i=1 skip the lookback-2 clamp `calc_unchecked` applies from bar 2 on) — both halves are transcribed in `warmup.rs`'s `K_PSAR` arm (the seed, since `feeds[k]=1` puts it on the warm path) and `codegen.rs`'s steady arm (every bar after, since `lead=1` means the steady loop handles almost the whole series); carry rides the shared `kvacc`/`st` scratch arrays `KvoFlow` also uses (safe — no node is ever both); used by `psar` |
| `SdftRotate{src,period}` | Sliding DFT rotation carry for `msw`'s MESA Sine Wave: coupled 2-state carry `[rp,ip]`, seeded by a weighted dot-product over the first `period` bars via `crate::indicators::msw::calc_rp_ip` called VERBATIM (bit-identical to the hand's own seed by construction, not re-derivation), then advanced every bar by `rp'=wr*rp-wi*ip+(new-old)`, `ip'=wr*ip+wi*rp` (OLD `rp`, simultaneous-update) | `K_SDFTROTATE=47`; `src` MUST be a raw `Node::Input` (erase panics otherwise -- `old` is a raw-stream lag by `period`, not a node read); `mults[k]`=wr=cos(2*PI/period), `invms[k]`=wi=sin(2*PI/period), `periods[k]`=period, `feeds[k]`=`leads[src]+period`; publishes `sine` (via `crate::indicators::msw::phase_from_rp_ip`, also called verbatim); caches `lead_sine` in the otherwise-unused 3rd slot of `kvacc[k]` for `SdftLead` to read; scalar-only; used by `msw` |
| `SdftLead{owner}` | Companion read of a `SdftRotate`'s cached `lead_sine` | `K_SDFTLEAD=48`; `owner` is a REAL graph edge (the `SdftRotate` node's own arena id), not an aliased slot -- topological order guarantees `owner` runs first in the same bar's k-loop, so `kvacc[owner_k][2]` is always fresh; no carry, no arithmetic of its own; scalar-only; used by `msw` |
| `HomodyneDiscriminator{src}` | Ehlers Homodyne Discriminator -- bespoke single-node carry, 7-state ring-backed recurrence, scalar-only | `K_HOMODYNEDISCRIMINATOR=49`; fixed 23-bar warmup (bars 0-21), lead 22; used by `homodynediscriminator` |

**Why four of those rows USED to say "cannot be erased" — the whole paragraph was stale.**
`HighPass`, `SuperSmoother`, `HtInPhase` and `HtQuadrature` are real `Node` variants and `merge`
always knew them. They once fell into `erase`'s `_ => return None` ("bespoke/rolling/feedback:
later milestones"), which is why an arena holding one yielded no `Signature`, no `SIG_*`, no
kernel and no gate.

**Every one of those blockers is gone, measured 2026-10-31:**

* `erase` has real arms — `signature.rs:473` (`HighPass`), `:480` (`SuperSmoother`), `:488`
  (`HtInPhase`), `:493` (`HtQuadrature`); rustc now reports the wildcard at `:781` as
  **unreachable**, i.e. `erase` covers every `Node` variant.
* `planner.rs` and `codegen.rs` do NOT "contain zero references": the dispatch arm is
  `planner.rs:438` `"HilbertTransform" => recipes::hilberttransform(&opts)`, and `codegen.rs`
  renders the HT ring taps (the 7-tap FIR over the shared `RCLASS_BESPOKE` ring).
* `hilberttransform` is registered **end to end**, not "pending the erase tier": recipe
  `recipes.rs:1294`, registry row `registry.rs:127` `hilbert_n: HilbertTransform(ss, hp)`, and a
  live gate `hilberttransform_bit_exact` (`cse_kernel_registry_test.rs:4790`, inside the 153-test
  suite). "registers the day those four kinds get `erase` arms" already happened.

Line numbers inside this file drift as the engine grows (the same paragraph also cited the merge
lead/formula rows at lines that have since moved, and an `erase` wildcard at `:604` now at `:781`)
— treat any `file:line` here as a pointer to grep, not a coordinate.

**Open consequence, deliberately NOT acted on here.** A dead wildcard is the anti-pattern this
repo warns about elsewhere: `merge.rs`'s `lane_formula` is explicitly "Exhaustive with NO
wildcard: a new `Node` variant fails to compile here, so it cannot silently become unpaklable."
`erase`'s now-dead `_ => return None` does the opposite — a future variant added without an arm
would compile and silently erase to nothing. Deleting it is a one-line, compile-safe change
(rustc already proves nothing reaches it), but it changes engine behaviour for kinds not yet
written, so it is a decision to make on its own, not a stray edit inside another slice.

**`Sma` vs `RollSum` is an interning fact, not a style choice.** They are distinct
signature kinds (`K_SMA=16`, `K_ROLLSUM=24`) so they never dedup together. Use
`Node::Sma` when the source is a raw price `Input` — then a co-requested `Sma(p)`
interns to ONE roll (measured in `sma_dpo_shared_p`). Use ring `RollSum` for derived
sources, which `Sma` cannot read.

`tail_inputs` names the nodes carrying recurrence state (`sma()`: `vec![m]`). Do not
invent mirrors for values the ring already owns.

## 6. Transcribing — the traps, all real

- **No invented constants.** `roc` is `mom / prev.max(EPSILON)`; a draft added `* 100`
  because that is what ROC "usually" is. Grep the hand for the literal.
- **`.max(x)` → `Node::Max`, never a `Where*`.** A `WhereGe{a:den,b:zero,t:num}`
  feeding a `Div` returns `num/0` at exactly the case the hand guards.
- **Guard polarity, not operator.** `if x <= lim { return carry }` → `WhereLe`
  (take-branch `!(a<=b)`); `if x < lim` → `WhereLt`. They differ on NaN, which
  finite parity matrices can never see.
- **`mul_add` → `Node::Fma`.** Separate `Mul`+`Add` double-rounds and is not
  bit-exact. One fused op in the hand = one `Fma` node.
- **`Composite` Wilders is a magnitude *and* a structure fact, never a new kind.** `erase`
  folds the flag into `Signature::b` as `composite as u32` (`signature.rs:397`) so mean and
  composite never intern, and puts the difference in VALUES: `invms[k] = 1.0` composite vs
  `1/period` mean (`:393`) — `x * 1.0` is bit-identical to `x` for every f64, so one
  spelling serves every reader. The accepted cost, stated at `:379-391`: the emitted step
  stays `fmul`+`fma` where the hand's `partial_calc` pays one `fma`.
- **Preserve associativity.** `(h+l)+c` ≠ `h+(l+c)` in f64.

## 7. Lane packing

`chain_step_form(kind)` (`codegen.rs:536`, `Ema | Wilders | Iir`) is the recurrence law:
such a node lowers to `chain_step` — one broadcast source, two per-slot weights from
`Values::mults`/`invms`, one carry in `st[k]`. It is **not the whole predicate**:
`classify_packed` packs a `Kind::Sma` too, but only when price-sourced
(`codegen.rs:94`, `p.plan.is_price_src(k)`), because its step reads the stream back at
`t − p` and a derived source would need the host anchor. `packed` is an explicit per-row
keyword in `kernel!`; without it no lane kernel is emitted.

Buckets are keyed on `(Kind, source)` — **not** on form — because `Ema` and
`Wilders` swap coefficient roles, so one group statement cannot serve both lanes.
`Iir` additionally has no scalar-hoist fallback yet. Cross-form groups need both
fixed first. Packing pays only where it *replaces* work the lane-group mechanism
cannot express. Cross-source packing was built, measured slower than not packing, and
rejected: a `splat` of one broadcast source beats assembling per-lane values.

## 8. Verification

```
cargo test --offline -q --test cse_kernel_registry_test   # + merge/assets suites
```
Address rows by **owner** (`row_for(&out, req_idx, primary, idx)`) — real
co-requests carry `want=false` mirrors, so positions are a property of the merge.
Assert disabled optionals are `is_empty()`. Include a case that reaches every
guard branch (e.g. zero volume); finite-only matrices cannot see them.

`CSE_FUSED_DUMP=1` writes `/tmp/cse_fused_expanded.rs`; `CSE_FUSED_DEBUG=1` prints
lane buckets. The dump is `TokenStream::to_string()` — it separates **every** token
(`. max (`), so grep only after stripping whitespace.

Formatting: `rustfmt --edition 2021 <files you edited>`, never `cargo fmt --all`.

Two parts of this guide are machine-checked by `cse_recipe_vocabulary_doc_test.rs`, in
both directions: §5's table against `pub enum Node` (a phantom name fails, and so does a
live variant the table omits), and every `kernel!` pattern name against the gate files,
so a registered row cannot be shipped without some hand-parity test naming it. The second
is a *mention* floor, not proof of parity — only the hand gate itself proves a row
correct.
