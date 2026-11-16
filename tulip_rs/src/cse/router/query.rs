//! `Request` → merged arena → fused kernel — the query surface.
//!
//! * [`Request`] builds the recipe list. `add(recipe, None)` mirrors hand
//!   `::indicator(None)` (primary-only: disabled optionals print EMPTY arrays
//!   in position, their stores are gated by `Values.sink_want` at runtime);
//!   `add(recipe, Some(&flags))` turns optionals on per
//!   `Info::optional_outputs`, and [`Request::add_all`] is the explicit
//!   all-on form.
//! * [`crate::cse::erase_request`] merges (CSE) the interface graphs, bakes
//!   leads and want-bits into `Signature`/`Values` — the dispatch key plus
//!   the runtime bindings for the macro-fused kernels (tier 1, see
//!   `proposals/Macro_Cse_Engine/`), the fallback yardsticks for hand and
//!   interpreter, and the input for the routing decision.
//! * The retired Cranelift JIT (`Request::build` → `JitPlan`) now lives in
//!   the top-level `old_jit/` archive; `get_states`/`resume` parity ports to
//!   the kernel registry as M1 (ROADMAP).
//!
//! The parity tests pin streaming invariants against the hand indicators
//! (integration suite: `tulip_test/tests/cse_kernel_registry_test.rs`).

use crate::cse::{ChainState, Recipe};
use crate::indicator_types::Indicator;
use crate::indicators::ema::State as EmaState;
use crate::types::Warm;
use std::sync::Arc;

/// Ordered list of indicator requests.
#[derive(Default)]
pub struct Request {
    pub(crate) recipes: Vec<Recipe>,
    /// Per-request optional-output flags (parallel to `recipes`).
    pub(crate) opt_flags: Vec<Vec<bool>>,
}

impl Request {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a request (from e.g. `Macd::recipe(&[12.,26.,9.])`) with
    /// hand-`::indicator`-parity optional-output flags — the ONE builder
    /// method, mirroring `indicator(inputs, options, Option<&[bool]>)`.
    /// `None` ⇒ every optional off (primary-only emission, the hand `None`
    /// call); `Some(flags)` ⇒ per declared optional in `Info::optional_outputs`
    /// order, missing tail = off. Disabled optionals print EMPTY arrays in
    /// declared position; enabled arrays and primaries are bit-identical to
    /// the all-on run.
    pub fn add(mut self, recipe: Recipe, optional_outputs: Option<&[bool]>) -> Self {
        let n = recipe.optional_outputs.len();
        let mut flags = vec![false; n];
        if let Some(f) = optional_outputs {
            for i in 0..n.min(f.len()) {
                flags[i] = f[i];
            }
        }
        self.recipes.push(recipe);
        self.opt_flags.push(flags);
        self
    }

    /// Append emitting **every** declared optional output (full `Results`).
    /// Explicitly named so "all-on" is never the silent default — the plain
    /// [`add`] mirrors the hand and defaults to primary-only (`None`).
    pub fn add_all(self, recipe: Recipe) -> Self {
        let n = recipe.optional_outputs.len();
        self.add(recipe, Some(&vec![true; n]))
    }

    /// Partition this request's indicators into **sharing groups**: request
    /// indices that reach a common computed node (a chain / arith — not a raw
    /// price `Input`) group together; disjoint ones split. The engine uses this
    /// to run one fused loop *per group* rather than one monolithic loop for the
    /// whole request (measured: fusing disjoint indicators into one loop gives no
    /// dedup and worse — see `natr`+`adx`). Cheap: merge + a union-find walk, no
    /// codegen. Exposed for measurement/diagnostics; the grouping is applied
    /// internally once the composite-plan path lands.
    pub fn sharing_groups(&self) -> Vec<Vec<usize>> {
        let (arena, mut resolved) = crate::cse::merge::merge(&self.recipes);
        for (r, f) in resolved.iter_mut().zip(&self.opt_flags) {
            r.optional_enabled = f.clone();
        }
        arena.sharing_groups(&resolved)
    }
}

/// Outputs for every request in a run/resume, request-major.
///
/// E2c: slots are `RowRef` VIEWS over one shared buffer per unique output
/// node — mirror lines (three requests' `short_ema`, say) alias the same
/// row the engine wrote once, and ±1 publish-lead differences are windows
/// (an offset), never a copy. Deref makes every existing call site
/// (`res.outputs[r][o].len()`, `[..]`, `&*` → `&[f64]`) work unchanged;
/// code that needs to OWN a row calls `to_vec()`.
#[derive(Debug, Clone, Default)]
pub struct Results {
    /// `outputs[request][output]` — primary, in `Info::outputs` order.
    pub outputs: Vec<Vec<RowRef>>,
    /// `optional[request][output]` — `Info::optional_outputs` order; EMPTY
    /// for sinks disabled via [`Request::add`].
    pub optional: Vec<Vec<RowRef>>,
}

// `Results` is `Send + Sync`, so a consumer may fan symbols across a thread
// pool and collect the rows off the workers (review §3.3 / §2.7). Pinned at
// compile time the way `JitPlan` pins itself (jit.rs), so an accidental
// revert to `Rc` fails the build rather than the scanner.
const _: fn() = || {
    fn assert<T: Send + Sync>() {}
    assert::<Results>();
    assert::<RowRef>();
};

/// One output row as written by the plan: a window into the chunk's shared
/// output ARENA (see [`Results`]). `advance` allocates ONE `Arc<[f64]>`
/// per chunk and every store writes its slice of it; each request slot
/// reads `arena[start..start + len]` — mirrors alias, ±1 publish leads are
/// offsets, nothing is copied, and the `Results` clone cost of a 5000-bar
/// dashboard is one refcount bump per row. `Arc` (not `Rc`) so `Results` is
/// `Send` — a consumer may fan symbols across threads and collect the rows
/// (review §3.3 / §2.7).
#[derive(Clone)]
pub struct RowRef {
    arena: Arc<[f64]>,
    start: usize,
    len: usize,
}

impl RowRef {
    pub(crate) fn window(arena: &Arc<[f64]>, start: usize, len: usize) -> Self {
        // a slot whose chunk ENDED before its publish lead projects past a
        // zero-row window (lead_off > rows); clamp to a valid empty view
        let start = start.min(arena.len());
        let len = len.min(arena.len() - start);
        Self {
            arena: Arc::clone(arena),
            start,
            len,
        }
    }
    pub fn as_slice(&self) -> &[f64] {
        &self.arena[self.start..self.start + self.len]
    }
    /// Own a copy (streaming-concat in tests; DB rows that must outlive
    /// the `Results`).
    pub fn to_vec(&self) -> Vec<f64> {
        self.as_slice().to_vec()
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// Do two slots read inside the SAME chunk arena? (E2c aliasing.)
    pub fn shares_data_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.arena, &other.arena)
    }
}

impl std::ops::Deref for RowRef {
    type Target = [f64];
    fn deref(&self) -> &[f64] {
        self.as_slice()
    }
}

impl std::fmt::Debug for RowRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl Results {
    pub fn request(&self, i: usize) -> RequestResults<'_> {
        RequestResults {
            outputs: &self.outputs[i],
            optional: &self.optional[i],
        }
    }
    /// Number of request slots in this `Results` (= plans' requests).
    pub fn request_count(&self) -> usize {
        self.outputs.len()
    }
}

/// A single request's outputs within a [`Results`].
pub struct RequestResults<'a> {
    outputs: &'a [RowRef],
    optional: &'a [RowRef],
}

impl<'a> RequestResults<'a> {
    pub fn output(&self, i: usize) -> &'a [f64] {
        self.outputs[i].as_slice()
    }
    pub fn optional(&self, i: usize) -> &'a [f64] {
        self.optional[i].as_slice()
    }
    /// Primary output-row count for this request (≤ `Info::outputs`).
    pub fn output_count(&self) -> usize {
        self.outputs.len()
    }
}
// (the returns above coerce `&RowRef` → `&[f64]` through Deref — the
//  accessor surface is byte-for-byte the pre-E2c API)

/// Concrete hand `IndicatorState`s for DB/FFI/legacy streaming, exported
/// one-way (never fed back into `resume` — independent timelines).
pub enum IndicatorStateBox {
    /// An `Ema` mirror request: exactly the arena node's own interface state.
    Ema(EmaState<Warm>),
    Apo(crate::indicators::apo::State<Warm>),
    Macd(crate::indicators::macd::State<Warm>),
    Ppo(crate::indicators::ppo::State<Warm>),
    AvgPrice(crate::indicators::avgprice::IndicatorState),
    Typprice(crate::indicators::typprice::IndicatorState),
    WcPrice(crate::indicators::wcprice::IndicatorState),
    Bop(crate::indicators::bop::IndicatorState),
    Marketfi(crate::indicators::marketfi::IndicatorState),
    Dema(crate::indicators::dema::IndicatorState),
    Tema(crate::indicators::tema::IndicatorState),
    Trix(crate::indicators::trix::IndicatorState),
    Roc(crate::indicators::roc::IndicatorState),
    Rocr(crate::indicators::rocr::IndicatorState),
    Qstick(crate::indicators::qstick::IndicatorState),
    Zlema(crate::indicators::zlema::IndicatorState),
    Highpass(crate::indicators::highpass::IndicatorState),
    Supersmoother(crate::indicators::supersmoother::IndicatorState),
    Roofingfilter(crate::indicators::roofingfilter::IndicatorState),
    Dpo(crate::indicators::dpo::IndicatorState),
    Emv(crate::indicators::emv::IndicatorState),
    Obv(crate::indicators::obv::IndicatorState),
    Wad(crate::indicators::wad::IndicatorState),
    Vwap(crate::indicators::vwap::IndicatorState),
    Vwma(crate::indicators::vwma::IndicatorState),
    Vosc(crate::indicators::vosc::IndicatorState),
    Chaikinmf(crate::indicators::chaikinmf::IndicatorState),
    Mfi(crate::indicators::mfi::IndicatorState),
    Vortex(crate::indicators::vortex::IndicatorState),
    Trima(crate::indicators::trima::IndicatorState),
    AdaptiveMsw(crate::indicators::adaptivemsw::IndicatorState),
    Cybercycle(crate::indicators::cybercycle::IndicatorState),
    Instantaneoustrendline(crate::indicators::instantaneoustrendline::IndicatorState),
    Mama(crate::indicators::mama::IndicatorState),
    Psar(crate::indicators::psar::IndicatorState),
    Trendmode(crate::indicators::trendmode::IndicatorState),
    Ccfisher(crate::indicators::ccfisher::IndicatorState),
    Fosc(crate::indicators::fosc::IndicatorState),
    Homodynediscriminator(crate::indicators::homodynediscriminator::IndicatorState),
    Kama(crate::indicators::kama::IndicatorState),
    Keltnerchannel(crate::indicators::keltnerchannel::IndicatorState),
    Linreg(crate::indicators::linreg::IndicatorState),
    Msw(crate::indicators::msw::IndicatorState),
    Smaenvelope(crate::indicators::smaenvelope::IndicatorState),
    Tsf(crate::indicators::tsf::IndicatorState),
    Vidya(crate::indicators::vidya::IndicatorState),
    /// E3 rolling recurrences — the hand's full continuable `IndicatorState`
    /// (`{real, state, period}`): the window `restore_state` now receives as its
    /// `window: &[f64]` argument is stitched in here, not left for the caller.
    Sma(crate::indicators::sma::IndicatorState),
    Stddev(crate::indicators::stddev::IndicatorState),
    Wma(crate::indicators::wma::IndicatorState),
    Wilders(crate::indicators::wilders::State<Warm>),
    /// E9 M1 — the hand's full continuable `IndicatorState` (`{real, state,
    /// period, period2}`); window-stitched the same way as the E3/E9 group
    /// above.
    Hma(crate::indicators::hma::IndicatorState),
    /// E9 trail-rescan rolls — the hand's full continuable `IndicatorState`
    /// (`{real, state, periods}`); same window-stitching as the E3 group above.
    Max(crate::indicators::max::IndicatorState),
    Min(crate::indicators::min::IndicatorState),
    /// E9 M1 — the trail consumers: `aroon` (`{min_state, max_state,
    /// multiplier}`) and `aroonosc` (which wraps the same aroon state). Both
    /// now hold the hand's full continuable `IndicatorState` (`{high, low,
    /// state, period}`); window-stitched the same way as the E3/E9 group above.
    Aroon(crate::indicators::aroon::IndicatorState),
    AroonOsc(crate::indicators::aroonosc::IndicatorState),
    /// E9 M2 — same-window consumers: `willr` and `donchianchannel` both carry
    /// exactly `{min_state, max_state}`, so they restore the same way and (when
    /// their windows match) read the SAME two roll chains. Both now hold the
    /// hand's full continuable `IndicatorState` (`{state, high, low,
    /// period(s)}`); window-stitched the same way as the E3/E9 group above.
    Willr(crate::indicators::willr::IndicatorState),
    DonchianChannel(crate::indicators::donchianchannel::IndicatorState),
    /// E9 M4 — the densest nesting case: three canonical windows per column, so
    /// the state is six `{value, trail}` pairs with no derived constants. Now
    /// the hand's full continuable `IndicatorState` (`{periods, high, low,
    /// state}`); window-stitched the same way as the E3/E9 group above.
    Ichimoku(crate::indicators::ichimoku::IndicatorState),
    /// E12 — `rsi`: the two MEAN-mode `Wilders` chains (the hand keeps both sums
    /// in one SIMD state) plus `prev_real`, which the recipe carries as an
    /// `Ema{period: 1}` — α=1 makes that recurrence the identity, so its chain
    /// slot holds the last processed close without any new engine concept.
    Rsi(crate::indicators::rsi::State<Warm>),
    /// E12 — `cmo`: two rolling SUMS over the same `up_down` subgraph `rsi`
    /// smooths, so an `rsi`+`cmo` query shares the movement nodes exactly. Now
    /// the hand's full continuable `IndicatorState` (`{real, state, period}`);
    /// window-stitched the same way as the E3/E9 group above.
    Cmo(crate::indicators::cmo::IndicatorState),
    /// Volatility: the Stddev chain (sum/sum_sq/period) plus close mirror
    /// (prev_real). The hand's full continuable `IndicatorState` (`{buffer,
    /// stddev_state, prev_real}`); window-stitched as with other rolling kinds.
    Volatility(crate::indicators::volatility::IndicatorState),
    /// The five kinds whose capture→decode→restore path is gated end-to-end in
    /// `cse_state_capture_test.rs` (`engine_carry_{kvo,hilbert,supertrend,adxr,
    /// mass}_*`) and which were therefore unreachable through the router until
    /// their arms were added below: no box variant meant no `restorer_for` arm.
    Kvo(crate::indicators::kvo::State<Warm>),
    HilbertTransform(crate::indicators::hilberttransform::State<Warm>),
    SuperTrend(crate::indicators::supertrend::State<Warm>),
    Adxr(crate::indicators::adxr::State<Warm>),
    Mass(crate::indicators::mass::State<Warm>),
    /// C9 batch 1 — the six kinds gated by
    /// `c9_batch1_dm_di_dx_adx_atr_natr_continue_bit_exact` before any wiring.
    /// Their tails are built only from `Wilders` + `Mirror` slots, both of which
    /// the decode table already answers.
    Atr(crate::indicators::atr::State<Warm>),
    Natr(crate::indicators::natr::State<Warm>),
    Dm(crate::indicators::dm::State<Warm>),
    Di(crate::indicators::di::State<Warm>),
    Dx(crate::indicators::dx::State<Warm>),
    Adx(crate::indicators::adx::State<Warm>),
    /// C9 batch 2 — the seven kinds `c9_batch2_group1_kinds_continue_bit_exact`
    /// proves resume bit-exactly from engine captures. The eight kinds that did
    /// NOT pass are deliberately absent (their names are pinned in that gate's
    /// expected-failure list, so a future pass shows up as a test change rather
    /// than as a silent wiring).
    Tr(crate::indicators::tr::State),
    Ad(crate::indicators::ad::IndicatorState),
    BBands(crate::indicators::bbands::IndicatorState),
    Cvi(crate::indicators::cvi::State<Warm>),
    Elderray(crate::indicators::elderray::State<Warm>),
    ChandelierExit(crate::indicators::chandelierexit::IndicatorState),
    Trvi(crate::indicators::trvi::State<Warm>),
    /// C11.3's first migration: `mom`'s state is a raw price tail, so it is the
    /// kind whose restore needs the OPTIONS contract — the window length is not
    /// its period (see `mom.rs`'s `restore_state_with`).
    Mom(crate::indicators::mom::IndicatorState),
    /// C11.3: `medprice`'s hand state is a UNIT STRUCT — the indicator carries no
    /// data across bars. The variant exists so the router can honestly answer a
    /// state request for it; resuming one is exactly recomputing the chunk.
    Medprice(crate::indicators::medprice::IndicatorState),
    /// C11.3: EF's state is the rolling `|Δreal|` sum plus the hand's two scalar
    /// retentions, all three captured — reachable once the recipe mirrored `real`
    /// for `prev` at the right depth.
    Ef(crate::indicators::ef::IndicatorState),
    /// C11.3: `vhf`'s state is its own `init_state` over the last `period + 1`
    /// bars — reachable once the restorer sliced that tail instead of trusting
    /// the window it was handed.
    Vhf(crate::indicators::vhf::IndicatorState),
    /// C11.3 / C12: AO's whole state is its two rolling sums and the 34-wide
    /// medprice ring, all carried. Reachable once `decode` could age-read a
    /// narrow member out of a wide shared ring group.
    Ao(crate::indicators::ao::State<Warm>),
    /// C12: `md`'s whole state is one rolling sum, carried off its `RollSum` node.
    Md(crate::indicators::md::IndicatorState),
    /// C12: CCI's state is the typprice ring plus the sma sum, both already in the
    /// one `RollSum` chain its recipe publishes — no `MeanAbsDev` variant needed.
    Cci(crate::indicators::cci::IndicatorState),
    /// C12: stoch's eight state values are all carried — two extremum trackers,
    /// two sums, and the two rings those sums evict from.
    Stoch(crate::indicators::stoch::IndicatorState),
    /// C12: ADOSC's state is two EMA chains plus the AD accumulator mirrored off
    /// its `Accum` node — no new variant was required.
    Adosc(crate::indicators::adosc::State<Warm>),
    /// C12: the NVI/PVI carries were recorded as needing new vocabulary; the
    /// recipes already export them as three `Mirror`s (carry, `close`, `volume`).
    Nvi(crate::indicators::nvi::IndicatorState),
    Pvi(crate::indicators::pvi::IndicatorState),
    /// C11.5: fisher's two IIR carries come from the chain (`IirClamped` and the
    /// `Prev` node decoded through the `Mirror` shape), the ring from an exactly
    /// `period`-bar tail, and the period from the request's options.
    Fisher(crate::indicators::fisher::IndicatorState),
    /// C12: ULTOSC's six bp/tr accumulators are the graph's own `RollSum`
    /// members; only its `(bp, tr)` ring is rebuilt, from the exact tail.
    Ultosc(crate::indicators::ultosc::IndicatorState),
    /// C14: stochrsi's state is the grafted `rsi`'s carries, the two rolls'
    /// `(value, trail)`, and the roll's own exported window as a `MirrorBuffer`.
    StochRsi(crate::indicators::stochrsi::IndicatorState),
    Other,
}

/// Rebuild one hand `IndicatorState` from its arena chain states — the
/// signature every trait-path restorer matches.
///
/// The third parameter is the request's own options vector, delivered to
/// `Restorable::restore_state_with`. Every entry in `restorer_for` forwards it;
/// the trait's default ignores it, so a kind behaves exactly as it did under the
/// two-parameter contract until it opts in (see the `restore_state_with` doc for
/// why a window length is not always a substitute).
pub type StateRestorer = fn(Vec<ChainState>, &[&[f64]], &[f64]) -> IndicatorStateBox;

/// Option-aware restorer used when the caller has the concrete request options
/// and wants exact state reconstruction for option-sensitive indicators.
pub type StateRestorerWithOptions = Box<
    dyn for<'a, 'b, 'c> Fn(Vec<ChainState>, &'a [&'b [f64]], &'c [f64]) -> IndicatorStateBox
        + Send
        + Sync
        + 'static,
>;

/// The kind → restorer table. Phase 1 of "recipes as data" removed the
/// `Recipe.restorer` field (the leaf crate cannot type a runtime restorer),
/// so the recipe's identity stays a string and this table resolves it.
///
/// Every arm in this table joins on the `<Type as Restorable<I, OP>>::KIND`
/// constant, never on a string literal, so the table cannot drift from the impl
/// it names. The rule for
/// adding one is gate-first: a kind enters only after
/// `cse_state_capture_test.rs` proves its captured carries decode and its resume
/// is bit-exact against an uninterrupted hand run. A phantom arm would promise a
/// continuable state on reasoning alone; unknown kinds fall to `None` and the
/// caller yields `IndicatorStateBox::Other`, which is how the hand path still
/// serves them. Census 2026-11-16: **93 arms over 93 `Restorable` impls** — every
/// `Planable` kind has an impl and an arm, and the join has no gap in either
/// direction. `cse_state_capture_test.rs` holds the continuation gate for each
/// kind whose resume was ever in doubt (the batch-2 probe's pinned failure list is
/// now empty for exactly this reason).
pub fn restorer_for(kind: &str) -> Option<StateRestorer> {
    use crate::cse::Restorable;
    match kind {
        k if k == <crate::indicators::ema::Ema as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::ema::Ema as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Ema)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::apo::Apo as Restorable<1, 2>>::KIND => Some(|v, w, o| {
            <crate::indicators::apo::Apo as Restorable<1, 2>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Apo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::macd::Macd as Restorable<1, 3>>::KIND => Some(|v, w, o| {
            <crate::indicators::macd::Macd as Restorable<1, 3>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Macd)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::ppo::Ppo as Restorable<1, 2>>::KIND => Some(|v, w, o| {
            <crate::indicators::ppo::Ppo as Restorable<1, 2>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Ppo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::avgprice::AvgPrice as Restorable<4, 0>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::avgprice::AvgPrice as Restorable<4, 0>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::AvgPrice)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::typprice::Typprice as Restorable<3, 0>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::typprice::Typprice as Restorable<3, 0>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Typprice)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::wcprice::WcPrice as Restorable<3, 0>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::wcprice::WcPrice as Restorable<3, 0>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::WcPrice)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::bop::Bop as Restorable<4, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::bop::Bop as Restorable<4, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Bop)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::marketfi::Marketfi as Restorable<3, 0>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::marketfi::Marketfi as Restorable<3, 0>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Marketfi)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::wilders::Wilders as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::wilders::Wilders as Restorable<1, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Wilders)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::sma::Sma as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::sma::Sma as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Sma)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::stddev::StdDev as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::stddev::StdDev as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Stddev)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::volatility::Volatility as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::volatility::Volatility as Restorable<1, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Volatility)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::wma::Wma as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::wma::Wma as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Wma)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::max::Max as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::max::Max as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Max)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::min::Min as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::min::Min as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Min)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::rsi::Rsi as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::rsi::Rsi as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Rsi)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::hma::Hma as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::hma::Hma as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Hma)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::aroon::Aroon as Restorable<2, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::aroon::Aroon as Restorable<2, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Aroon)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::aroonosc::AroonOsc as Restorable<2, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::aroonosc::AroonOsc as Restorable<2, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::AroonOsc)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::willr::Willr as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::willr::Willr as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Willr)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k
            == <crate::indicators::donchianchannel::DonchianChannel as Restorable<2, 1>>::KIND =>
        {
            Some(|v, w, o| {
                <crate::indicators::donchianchannel::DonchianChannel as Restorable<2, 1>>::restore_state_with(
                    v, w, o,
                )
                .map(IndicatorStateBox::DonchianChannel)
                .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::ichimoku::Ichimoku as Restorable<3, 2>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::ichimoku::Ichimoku as Restorable<3, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Ichimoku)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::cmo::Cmo as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::cmo::Cmo as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Cmo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::dema::Dema as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::dema::Dema as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Dema)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::tema::Tema as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::tema::Tema as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Tema)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::trix::Trix as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::trix::Trix as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Trix)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::roc::Roc as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::roc::Roc as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Roc)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::rocr::Rocr as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::rocr::Rocr as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Rocr)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::qstick::QStick as Restorable<2, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::qstick::QStick as Restorable<2, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Qstick)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::zlema::Zlema as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::zlema::Zlema as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Zlema)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::highpass::HighPass as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::highpass::HighPass as Restorable<1, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Highpass)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::supersmoother::SuperSmoother as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::supersmoother::SuperSmoother as Restorable<1, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Supersmoother)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::roofingfilter::RoofingFilter as Restorable<1, 2>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::roofingfilter::RoofingFilter as Restorable<1, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Roofingfilter)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::dpo::Dpo as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::dpo::Dpo as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Dpo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::emv::Emv as Restorable<3, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::emv::Emv as Restorable<3, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Emv)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::obv::Obv as Restorable<2, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::obv::Obv as Restorable<2, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Obv)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::wad::Wad as Restorable<3, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::wad::Wad as Restorable<3, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Wad)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::vwap::Vwap as Restorable<4, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::vwap::Vwap as Restorable<4, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Vwap)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::vwma::Vwma as Restorable<2, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::vwma::Vwma as Restorable<2, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Vwma)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::vosc::Vosc as Restorable<1, 2>>::KIND => Some(|v, w, o| {
            <crate::indicators::vosc::Vosc as Restorable<1, 2>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Vosc)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::chaikinmf::ChaikinMf as Restorable<4, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::chaikinmf::ChaikinMf as Restorable<4, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Chaikinmf)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::mfi::Mfi as Restorable<4, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::mfi::Mfi as Restorable<4, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Mfi)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::vortex::Vortex as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::vortex::Vortex as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Vortex)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::trima::Trima as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::trima::Trima as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Trima)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::adaptivemsw::AdaptiveMSW as Restorable<1, 0>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::adaptivemsw::AdaptiveMSW as Restorable<1, 0>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::AdaptiveMsw)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::cybercycle::Cybercycle as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::cybercycle::Cybercycle as Restorable<1, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Cybercycle)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k
            == <crate::indicators::instantaneoustrendline::InstantaneousTrendline as Restorable<1, 0>>::KIND =>
        {
            Some(|v, w, o| {
                <crate::indicators::instantaneoustrendline::InstantaneousTrendline as Restorable<1, 0>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Instantaneoustrendline)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::mama::Mama as Restorable<1, 2>>::KIND => Some(|v, w, o| {
            <crate::indicators::mama::Mama as Restorable<1, 2>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Mama)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::psar::Psar as Restorable<2, 2>>::KIND => Some(|v, w, o| {
            <crate::indicators::psar::Psar as Restorable<2, 2>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Psar)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::trendmode::TrendMode as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::trendmode::TrendMode as Restorable<1, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Trendmode)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::ccfisher::CcFisher as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::ccfisher::CcFisher as Restorable<1, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Ccfisher)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::fosc::Fosc as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::fosc::Fosc as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Fosc)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k
            == <crate::indicators::homodynediscriminator::HomodyneDiscriminator as Restorable<1, 0>>::KIND =>
        {
            Some(|v, w, o| {
                <crate::indicators::homodynediscriminator::HomodyneDiscriminator as Restorable<1, 0>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Homodynediscriminator)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::kama::Kama as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::kama::Kama as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Kama)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k
            == <crate::indicators::keltnerchannel::KeltnerChannel as Restorable<3, 2>>::KIND =>
        {
            Some(|v, w, o| {
                <crate::indicators::keltnerchannel::KeltnerChannel as Restorable<3, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Keltnerchannel)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::linreg::Linreg as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::linreg::Linreg as Restorable<1, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Linreg)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::msw::Msw as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::msw::Msw as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Msw)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::smaenvelope::SmaEnvelope as Restorable<1, 2>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::smaenvelope::SmaEnvelope as Restorable<1, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Smaenvelope)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::tsf::Tsf as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::tsf::Tsf as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Tsf)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::vidya::Vidya as Restorable<1, 3>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::vidya::Vidya as Restorable<1, 3>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Vidya)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        // The five kinds whose chains are captured, decoded AND proven to
        // continue bit-exactly by `cse_state_capture_test.rs` (`engine_carry_*`).
        // Before these arms the router could not serve them AT ALL: the table
        // covered 61 of the 93 `Restorable` impls, so a state the engine could
        // build was unreachable from the only surface a caller uses. The census
        // must join on the TYPE in `<…::Type as Restorable>` against the
        // `Restorable<…> for Type` clause — a first pass joined module names
        // against types (`ema` vs `Ema`) and looked like a different gap; both
        // wrong joins were caught by the same check (an arm list that disagreed
        // with a gate that had just passed). No impl is left armless: every kind
        // the registry can compose now has a restorer that a continuation gate
        // proved, so a state is never promised on reasoning alone.
        k if k == <crate::indicators::kvo::Kvo as Restorable<4, 2>>::KIND => Some(|v, w, o| {
            <crate::indicators::kvo::Kvo as Restorable<4, 2>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Kvo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k
            == <crate::indicators::hilberttransform::HilbertTransform as Restorable<1, 2>>::KIND =>
        {
            Some(|v, w, o| {
                <crate::indicators::hilberttransform::HilbertTransform as Restorable<1, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::HilbertTransform)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::supertrend::SuperTrend as Restorable<3, 2>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::supertrend::SuperTrend as Restorable<3, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::SuperTrend)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::adxr::Adxr as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::adxr::Adxr as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Adxr)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::mass::Mass as Restorable<2, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::mass::Mass as Restorable<2, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Mass)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C9 batch 1: gated first (`c9_batch1_dm_di_dx_adx_atr_natr_continue_bit_exact`),
        // wired second — every one of the six resumed bit-exactly from the
        // engine's own captures before these arms existed.
        k if k == <crate::indicators::atr::Atr as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::atr::Atr as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Atr)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::natr::Natr as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::natr::Natr as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Natr)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::dm::Dm as Restorable<2, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::dm::Dm as Restorable<2, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Dm)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::di::Di as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::di::Di as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Di)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::dx::Dx as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::dx::Dx as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Dx)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::adx::Adx as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::adx::Adx as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Adx)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C9 batch 2: gated first, wired second. Seven of fifteen group-1 kinds
        // passed; the eight that did not (medprice, ao, mom, ef, vhf, ultosc,
        // stochrsi, stoch) are NOT wired, so they keep falling to the hand path
        // instead of shipping a state nobody proved.
        k if k == <crate::indicators::tr::Tr as Restorable<3, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::tr::Tr as Restorable<3, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Tr)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::ad::Ad as Restorable<4, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::ad::Ad as Restorable<4, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Ad)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::bbands::BBands as Restorable<1, 2>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::bbands::BBands as Restorable<1, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::BBands)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::cvi::Cvi as Restorable<2, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::cvi::Cvi as Restorable<2, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Cvi)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::elderray::Elderray as Restorable<3, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::elderray::Elderray as Restorable<3, 1>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Elderray)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k
            == <crate::indicators::chandelierexit::ChandelierExit as Restorable<3, 2>>::KIND =>
        {
            Some(|v, w, o| {
                <crate::indicators::chandelierexit::ChandelierExit as Restorable<3, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::ChandelierExit)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::trvi::Trvi as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::trvi::Trvi as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Trvi)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C11.3 first migration: gated FIRST
        // (`engine_carry_mom_continues_bit_exact`, which also pins that the
        // window-length spelling does NOT resume), wired second. This is the
        // first kind whose arm only works because `StateRestorer` carries the
        // options vector — its state is a price tail with no chain to decode, so
        // before the shim the router's full-column window silently restored a
        // different period.
        k if k == <crate::indicators::mom::Mom as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::mom::Mom as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Mom)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C11.3: gated by the batch-2 probe through `engine_stateless_family`.
        // The state is a unit struct, so this arm carries no decode and no
        // parameter — it exists so a medprice request stops withdrawing the
        // router's state promise for want of an arm.
        k if k == <crate::indicators::medprice::Medprice as Restorable<2, 0>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::medprice::Medprice as Restorable<2, 0>>::restore_state_with(
                    v, w, o,
                )
                .map(IndicatorStateBox::Medprice)
                .unwrap_or(IndicatorStateBox::Other)
            })
        }
        // C11.3: gated first by the batch-2 probe, which went from failing to
        // passing when `recipes::ef` mirrored `real` instead of `Lag{real, 1}` for
        // the hand's `prev` scalar. Its state is three carried scalars, so this is
        // a genuine capture — no window replay, no re-initialization.
        k if k == <crate::indicators::ef::Ef as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::ef::Ef as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Ef)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C11.3: gated first by the batch-2 probe; it went bit-exact when
        // `vhf::restore_state` started slicing the `period + 1` bars its
        // `init_state` actually seeds from.
        k if k == <crate::indicators::vhf::Vhf as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::vhf::Vhf as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Vhf)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C12: gated first by the batch-2 probe. It went from last-bit divergence
        // to bit-exact when the sums started being CARRIED (`tail_inputs` on the
        // two SMA nodes) instead of re-summed by `init_state`.
        k if k == <crate::indicators::ao::Ao as Restorable<2, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::ao::Ao as Restorable<2, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Ao)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C12: gated by `engine_carry_md_continues_bit_exact`.
        k if k == <crate::indicators::md::Md as Restorable<1, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::md::Md as Restorable<1, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Md)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C12: gated by `engine_carry_cci_continues_bit_exact`.
        k if k == <crate::indicators::cci::Cci as Restorable<3, 1>>::KIND => Some(|v, w, o| {
            <crate::indicators::cci::Cci as Restorable<3, 1>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Cci)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C12: gated by the batch-2 probe after `mn` joined `tail_inputs` and the
        // restorer switched from `init_state` to the carried chains.
        k if k == <crate::indicators::stoch::Stoch as Restorable<3, 3>>::KIND => Some(|v, w, o| {
            <crate::indicators::stoch::Stoch as Restorable<3, 3>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Stoch)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C12: gated by `engine_carry_adosc_continues_bit_exact`.
        k if k == <crate::indicators::adosc::Adosc as Restorable<4, 2>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::adosc::Adosc as Restorable<4, 2>>::restore_state_with(v, w, o)
                    .map(IndicatorStateBox::Adosc)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        // C12: gated by `engine_carry_nvi_pvi_continue_bit_exact`. Each is three
        // carried scalars — the index itself plus the two gate inputs the next
        // bar compares — so the resume continues the prefix product exactly.
        k if k == <crate::indicators::nvi::Nvi as Restorable<2, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::nvi::Nvi as Restorable<2, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Nvi)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::pvi::Pvi as Restorable<2, 0>>::KIND => Some(|v, w, o| {
            <crate::indicators::pvi::Pvi as Restorable<2, 0>>::restore_state_with(v, w, o)
                .map(IndicatorStateBox::Pvi)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        // C11.5: gated by `engine_carry_fisher_continues_bit_exact`.
        k if k == <crate::indicators::fisher::Fisher as Restorable<2, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::fisher::Fisher as Restorable<2, 1>>::restore_state_with(
                    v, w, o,
                )
                .map(IndicatorStateBox::Fisher)
                .unwrap_or(IndicatorStateBox::Other)
            })
        }
        // C12: gated by the batch-2 probe after its six sums started being
        // exported and its `init_state` tail started being sliced.
        k if k == <crate::indicators::ultosc::Ultosc as Restorable<3, 3>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::ultosc::Ultosc as Restorable<3, 3>>::restore_state_with(
                    v, w, o,
                )
                .map(IndicatorStateBox::Ultosc)
                .unwrap_or(IndicatorStateBox::Other)
            })
        }
        // C14: gated by the batch-2 probe, which emptied the pinned failure list.
        k if k == <crate::indicators::stochrsi::StochRsi as Restorable<1, 1>>::KIND => {
            Some(|v, w, o| {
                <crate::indicators::stochrsi::StochRsi as Restorable<1, 1>>::restore_state_with(
                    v, w, o,
                )
                .map(IndicatorStateBox::StochRsi)
                .unwrap_or(IndicatorStateBox::Other)
            })
        }
        _ => None,
    }
}

/// Option-aware resolver for state restoration.
///
/// Eight kinds (`cybercycle`, `trendmode`, `ccfisher`, `mama`, `psar`,
/// `keltnerchannel`, `smaenvelope`, `vidya`) are special-cased here. Their entry
/// is NOT a capture decode: it ignores the chain list and rebuilds the state by
/// RE-RUNNING the hand `Indicator::indicator` over the caller's windows with the
/// request's own option constants, so it must resolve the options up front and
/// capture them in the closure. That is why they are keyed by string literal and
/// live in a separate table from `restorer_for`, whose arms are all
/// `<Type as Restorable>::KIND`-keyed chain decodes.
///
/// Every other kind falls through to `restorer_for`, which now forwards the same
/// options vector to `Restorable::restore_state_with`; these eight are therefore
/// migratable into their own `restore_state_with` impl, one at a time, and each
/// migration deletes one arm from this table.
pub fn restorer_for_with_options(kind: &str, options: &[f64]) -> Option<StateRestorerWithOptions> {
    match kind {
        "cybercycle" => {
            let alpha = options.first().copied().unwrap_or(0.07);
            Some(Box::new(move |_v, windows, _opts| {
                let [real] = windows else {
                    return IndicatorStateBox::Other;
                };
                match crate::indicators::cybercycle::Cybercycle::indicator(&[real], &[alpha], None)
                {
                    Ok((_o, state)) => IndicatorStateBox::Cybercycle(state),
                    Err(_) => IndicatorStateBox::Other,
                }
            }))
        }
        "trendmode" => {
            let alpha = options.first().copied().unwrap_or(0.07);
            Some(Box::new(move |_v, windows, _opts| {
                let [real] = windows else {
                    return IndicatorStateBox::Other;
                };
                match crate::indicators::trendmode::TrendMode::indicator(&[real], &[alpha], None) {
                    Ok((_o, state)) => IndicatorStateBox::Trendmode(state),
                    Err(_) => IndicatorStateBox::Other,
                }
            }))
        }
        "ccfisher" => {
            let alpha = options.first().copied().unwrap_or(0.07);
            Some(Box::new(move |_v, windows, _opts| {
                let [real] = windows else {
                    return IndicatorStateBox::Other;
                };
                match crate::indicators::ccfisher::CcFisher::indicator(&[real], &[alpha], None) {
                    Ok((_o, state)) => IndicatorStateBox::Ccfisher(state),
                    Err(_) => IndicatorStateBox::Other,
                }
            }))
        }
        "mama" => {
            let fast_limit = options.first().copied().unwrap_or(0.5);
            let slow_limit = options.get(1).copied().unwrap_or(0.05);
            Some(Box::new(move |_v, windows, _opts| {
                let [real] = windows else {
                    return IndicatorStateBox::Other;
                };
                match crate::indicators::mama::Mama::indicator(
                    &[real],
                    &[fast_limit, slow_limit],
                    None,
                ) {
                    Ok((_o, state)) => IndicatorStateBox::Mama(state),
                    Err(_) => IndicatorStateBox::Other,
                }
            }))
        }
        "psar" => {
            let af_step = options.first().copied().unwrap_or(0.02);
            let max_af = options.get(1).copied().unwrap_or(0.2);
            Some(Box::new(move |_v, windows, _opts| {
                let [high, low] = windows else {
                    return IndicatorStateBox::Other;
                };
                match crate::indicators::psar::Psar::indicator(
                    &[high, low],
                    &[af_step, max_af],
                    None,
                ) {
                    Ok((_o, state)) => IndicatorStateBox::Psar(state),
                    Err(_) => IndicatorStateBox::Other,
                }
            }))
        }
        "keltnerchannel" => {
            let period = options.first().copied().unwrap_or(20.0);
            let step = options.get(1).copied().unwrap_or(2.0);
            Some(Box::new(move |_v, windows, _opts| {
                let [high, low, close] = windows else {
                    return IndicatorStateBox::Other;
                };
                match crate::indicators::keltnerchannel::KeltnerChannel::indicator(
                    &[high, low, close],
                    &[period, step],
                    None,
                ) {
                    Ok((_o, state)) => IndicatorStateBox::Keltnerchannel(state),
                    Err(_) => IndicatorStateBox::Other,
                }
            }))
        }
        "smaenvelope" => {
            let period = options.first().copied().unwrap_or(20.0);
            let percentage = options.get(1).copied().unwrap_or(1.0);
            Some(Box::new(move |_v, windows, _opts| {
                let [real] = windows else {
                    return IndicatorStateBox::Other;
                };
                match crate::indicators::smaenvelope::SmaEnvelope::indicator(
                    &[real],
                    &[period, percentage],
                    None,
                ) {
                    Ok((_o, state)) => IndicatorStateBox::Smaenvelope(state),
                    Err(_) => IndicatorStateBox::Other,
                }
            }))
        }
        "vidya" => {
            let short_period = options.first().copied().unwrap_or(5.0);
            let long_period = options.get(1).copied().unwrap_or(20.0);
            let alpha = options.get(2).copied().unwrap_or(0.2);
            Some(Box::new(move |_v, windows, _opts| {
                let [real] = windows else {
                    return IndicatorStateBox::Other;
                };
                match crate::indicators::vidya::Vidya::indicator(
                    &[real],
                    &[short_period, long_period, alpha],
                    None,
                ) {
                    Ok((_o, state)) => IndicatorStateBox::Vidya(state),
                    Err(_) => IndicatorStateBox::Other,
                }
            }))
        }
        _ => restorer_for(kind).map(|f| Box::new(f) as StateRestorerWithOptions),
    }
}
