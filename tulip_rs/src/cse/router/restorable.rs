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

    /// `restore_state` plus the request's own options vector.
    ///
    /// Exists because a window slice cannot always name its own parameter. The
    /// rolling kinds are safe: `IndicatorState::new` tail-slices `period` bars out
    /// of whatever window it is handed, so the period travels inside the slice.
    /// Others read the slice LENGTH as the parameter — `mom` is exactly
    /// `period = windows[0].len()`, and `fosc`/the roll consumers land on
    /// `period ± 1`. For those, a generous window is not merely wasteful but
    /// wrong: it silently restores a different period, and the resumed series
    /// diverges from row 0 (`mom` measured 300/300 wrong rows, `1.0274769512599278`
    /// vs `0.5911711585153654`, 2026-11-15). The options vector is the only
    /// source that states the period independently of the data length.
    ///
    /// The default ignores `options` and calls `restore_state`, so every impl
    /// compiles and behaves exactly as before until it opts in. `options` is the
    /// recipe's runtime vector — the same values `Planable::recipe` published —
    /// so a kind that overrides this must not assume a fixed length; read it with
    /// `options.first()`/`options.get(n)` the way the hand's own constructors do.
    fn restore_state_with(
        chains: Vec<ChainState>,
        windows: &[&[f64]],
        _options: &[f64],
    ) -> Option<Self::IndicatorState> {
        Self::restore_state(chains, windows)
    }
}
