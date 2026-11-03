//! The `Restorable` trait — the compile-time contract behind state return.
//!
//! Reconstructed 2026-11-03: this file went missing (pre-existing breakage,
//! unrelated to that day's codegen-corruption incident — see
//! `proposals/Macro_Cse_Engine/handover/09-codegen-corruption-recovery-2026-11-03.md`)
//! and had no git history to restore from. Rebuilt verbatim from the design
//! doc `proposals/RESTORABLE_TRAIT.md` section 4 ("Shape — as actually built
//! this session"), which recorded the exact signature and doc comments below.

use crate::cse::ChainState;
use crate::cse::Planable;
use crate::indicator_types::Indicator;

/// Must equal the `kind` this type's `Planable::recipe` publishes, and
/// restores the continuable hand state from the arena's captured chains.
pub trait Restorable<const I: usize, const OP: usize>: Planable + Indicator<I, OP> {
    /// Must equal the `kind` this type's `Planable::recipe` publishes.
    const KIND: &'static str;

    /// Rebuild the continuable state, or `None` when the chains (or a window,
    /// for windowed kinds) do not describe this indicator's layout.
    ///
    /// `windows` holds one or more host-side price-tail slices (`JitState.window`),
    /// ordered per-kind, for kinds whose `IndicatorState` carries a raw price tail
    /// alongside the chain-captured sums. Single-input windowed kinds
    /// (`sma`/`stddev`/`wma`/`max`/`min`/`hma`/`cmo`) read `windows[0]`. The five
    /// two-input roll-consumer kinds (`aroon`/`aroonosc`/`willr`/`donchianchannel`/
    /// `ichimoku`) read `windows[0] = high`, `windows[1] = low` — a convention this
    /// trait pins by doc comment only (nothing else enforces the order; verify
    /// against each kind's `IndicatorState::new(...)` argument order directly, they
    /// are NOT all the same). Chain-only kinds (`ema`/`apo`/`macd`/`ppo`/`wilders`/
    /// `rsi`) ignore it entirely.
    fn restore_state(chains: Vec<ChainState>, windows: &[&[f64]]) -> Option<Self::IndicatorState>;
}
