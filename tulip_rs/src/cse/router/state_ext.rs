//! Runtime-side extension of the leaf's `ChainState` (Phase 1 split): the
//! leaf holds the pure data enum; the bridge back to hand `State` types is
//! a SEALED-IN-SPIRIT extension trait here because inherent impls must live
//! in the defining crate and this one names `indicators::*` types the leaf
//! may not see. Re-exported from `crate::cse` so indicator restorers add a
//! single import to keep the `v[i].as_ema()` call shape.

use crate::cse::ChainState;

/// The Ema-family view of a resolved chain state.
pub trait ChainStateEma {
    /// Rebuilds the hand `ema::State<Warm>` from an `Ema` chain (used by the
    /// ema/apo/ppo/macd restorers). Panics on a non-`Ema` variant; those
    /// indicators' `tail_inputs` only name Emas.
    fn as_ema(&self) -> crate::indicators::ema::State<crate::types::Warm>;
}

impl ChainStateEma for ChainState {
    fn as_ema(&self) -> crate::indicators::ema::State<crate::types::Warm> {
        match self {
            ChainState::Ema { value, period } => {
                crate::indicators::ema::State::new(*value, *period as usize).into_warm()
            }
            other => panic!("cse: restorer expected an Ema chain, got {other:?}"),
        }
    }
}
