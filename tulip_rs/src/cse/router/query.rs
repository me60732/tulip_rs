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
    Other,
}

/// Rebuild one hand `IndicatorState` from its arena chain states — the
/// signature every indicator's `restore_state` matches.
pub type StateRestorer = fn(Vec<ChainState>, &[&[f64]]) -> IndicatorStateBox;

/// The kind → restorer table. Phase 1 of "recipes as data" removed the
/// `Recipe.restorer` field (the leaf crate cannot type a runtime restorer),
/// so the recipe's identity stays a string and this table resolves it.
/// The entries are EXACTLY the 18 kinds whose recipes set
/// `r.restorer = Some(restore_state)` at the pre-JIT-retirement HEAD
/// (`git grep "r.restorer = Some"`) — a phantom entry here would claim the
/// engine can rebuild a hand state it has no wiring for, changing
/// `get_states` behavior; unknown kinds fall to `None`, mirroring the old
/// absent-field path (caller yields `IndicatorStateBox::Other`).
pub fn restorer_for(kind: &str) -> Option<StateRestorer> {
    use crate::cse::Restorable;
    match kind {
        k if k == <crate::indicators::ema::Ema as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::ema::Ema as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Ema)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::apo::Apo as Restorable<1, 2>>::KIND => Some(|v, w| {
            <crate::indicators::apo::Apo as Restorable<1, 2>>::restore_state(v, w)
                .map(IndicatorStateBox::Apo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::macd::Macd as Restorable<1, 3>>::KIND => Some(|v, w| {
            <crate::indicators::macd::Macd as Restorable<1, 3>>::restore_state(v, w)
                .map(IndicatorStateBox::Macd)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::ppo::Ppo as Restorable<1, 2>>::KIND => Some(|v, w| {
            <crate::indicators::ppo::Ppo as Restorable<1, 2>>::restore_state(v, w)
                .map(IndicatorStateBox::Ppo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::avgprice::AvgPrice as Restorable<4, 0>>::KIND => {
            Some(|v, w| {
                <crate::indicators::avgprice::AvgPrice as Restorable<4, 0>>::restore_state(v, w)
                    .map(IndicatorStateBox::AvgPrice)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::typprice::Typprice as Restorable<3, 0>>::KIND => {
            Some(|v, w| {
                <crate::indicators::typprice::Typprice as Restorable<3, 0>>::restore_state(v, w)
                    .map(IndicatorStateBox::Typprice)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::wcprice::WcPrice as Restorable<3, 0>>::KIND => {
            Some(|v, w| {
                <crate::indicators::wcprice::WcPrice as Restorable<3, 0>>::restore_state(v, w)
                    .map(IndicatorStateBox::WcPrice)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::bop::Bop as Restorable<4, 0>>::KIND => Some(|v, w| {
            <crate::indicators::bop::Bop as Restorable<4, 0>>::restore_state(v, w)
                .map(IndicatorStateBox::Bop)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::marketfi::Marketfi as Restorable<3, 0>>::KIND => {
            Some(|v, w| {
                <crate::indicators::marketfi::Marketfi as Restorable<3, 0>>::restore_state(v, w)
                    .map(IndicatorStateBox::Marketfi)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::wilders::Wilders as Restorable<1, 1>>::KIND => {
            Some(|v, w| {
                <crate::indicators::wilders::Wilders as Restorable<1, 1>>::restore_state(v, w)
                    .map(IndicatorStateBox::Wilders)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::sma::Sma as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::sma::Sma as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Sma)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::stddev::StdDev as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::stddev::StdDev as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Stddev)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::volatility::Volatility as Restorable<1, 1>>::KIND => {
            Some(|v, w| {
                <crate::indicators::volatility::Volatility as Restorable<1, 1>>::restore_state(v, w)
                    .map(IndicatorStateBox::Volatility)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::wma::Wma as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::wma::Wma as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Wma)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::max::Max as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::max::Max as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Max)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::min::Min as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::min::Min as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Min)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::rsi::Rsi as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::rsi::Rsi as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Rsi)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::hma::Hma as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::hma::Hma as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Hma)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::aroon::Aroon as Restorable<2, 1>>::KIND => Some(|v, w| {
            <crate::indicators::aroon::Aroon as Restorable<2, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Aroon)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::aroonosc::AroonOsc as Restorable<2, 1>>::KIND => {
            Some(|v, w| {
                <crate::indicators::aroonosc::AroonOsc as Restorable<2, 1>>::restore_state(v, w)
                    .map(IndicatorStateBox::AroonOsc)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::willr::Willr as Restorable<3, 1>>::KIND => Some(|v, w| {
            <crate::indicators::willr::Willr as Restorable<3, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Willr)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k
            == <crate::indicators::donchianchannel::DonchianChannel as Restorable<2, 1>>::KIND =>
        {
            Some(|v, w| {
                <crate::indicators::donchianchannel::DonchianChannel as Restorable<2, 1>>::restore_state(
                    v, w,
                )
                .map(IndicatorStateBox::DonchianChannel)
                .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::ichimoku::Ichimoku as Restorable<3, 2>>::KIND => {
            Some(|v, w| {
                <crate::indicators::ichimoku::Ichimoku as Restorable<3, 2>>::restore_state(v, w)
                    .map(IndicatorStateBox::Ichimoku)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::cmo::Cmo as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::cmo::Cmo as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Cmo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::dema::Dema as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::dema::Dema as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Dema)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::tema::Tema as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::tema::Tema as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Tema)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::trix::Trix as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::trix::Trix as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Trix)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::roc::Roc as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::roc::Roc as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Roc)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::rocr::Rocr as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::rocr::Rocr as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Rocr)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::qstick::QStick as Restorable<2, 1>>::KIND => Some(|v, w| {
            <crate::indicators::qstick::QStick as Restorable<2, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Qstick)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::zlema::Zlema as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::zlema::Zlema as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Zlema)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::highpass::HighPass as Restorable<1, 1>>::KIND => {
            Some(|v, w| {
                <crate::indicators::highpass::HighPass as Restorable<1, 1>>::restore_state(v, w)
                    .map(IndicatorStateBox::Highpass)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::supersmoother::SuperSmoother as Restorable<1, 1>>::KIND => {
            Some(|v, w| {
                <crate::indicators::supersmoother::SuperSmoother as Restorable<1, 1>>::restore_state(v, w)
                    .map(IndicatorStateBox::Supersmoother)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::roofingfilter::RoofingFilter as Restorable<1, 2>>::KIND => {
            Some(|v, w| {
                <crate::indicators::roofingfilter::RoofingFilter as Restorable<1, 2>>::restore_state(v, w)
                    .map(IndicatorStateBox::Roofingfilter)
                    .unwrap_or(IndicatorStateBox::Other)
            })
        }
        k if k == <crate::indicators::dpo::Dpo as Restorable<1, 1>>::KIND => Some(|v, w| {
            <crate::indicators::dpo::Dpo as Restorable<1, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Dpo)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::emv::Emv as Restorable<3, 0>>::KIND => Some(|v, w| {
            <crate::indicators::emv::Emv as Restorable<3, 0>>::restore_state(v, w)
                .map(IndicatorStateBox::Emv)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::obv::Obv as Restorable<2, 0>>::KIND => Some(|v, w| {
            <crate::indicators::obv::Obv as Restorable<2, 0>>::restore_state(v, w)
                .map(IndicatorStateBox::Obv)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::wad::Wad as Restorable<3, 0>>::KIND => Some(|v, w| {
            <crate::indicators::wad::Wad as Restorable<3, 0>>::restore_state(v, w)
                .map(IndicatorStateBox::Wad)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::vwap::Vwap as Restorable<4, 0>>::KIND => Some(|v, w| {
            <crate::indicators::vwap::Vwap as Restorable<4, 0>>::restore_state(v, w)
                .map(IndicatorStateBox::Vwap)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::vwma::Vwma as Restorable<2, 1>>::KIND => Some(|v, w| {
            <crate::indicators::vwma::Vwma as Restorable<2, 1>>::restore_state(v, w)
                .map(IndicatorStateBox::Vwma)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        k if k == <crate::indicators::vosc::Vosc as Restorable<1, 2>>::KIND => Some(|v, w| {
            <crate::indicators::vosc::Vosc as Restorable<1, 2>>::restore_state(v, w)
                .map(IndicatorStateBox::Vosc)
                .unwrap_or(IndicatorStateBox::Other)
        }),
        _ => None,
    }
}
