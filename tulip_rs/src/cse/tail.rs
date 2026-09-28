//! The tail **contract** — the reference form of each indicator's math.
//!
//! The tail *bodies* live with their indicators (`apo::ApoTail`,
//! `ppo::PpoTail`, `macd::MacdTail`): a tail is the indicator's own math —
//! the `calc_cse` style you seeded — consuming the planner's gated **EMA
//! values, not price**, and owning any state the planner does not model
//! (macd's signal EMA). Keeping each body beside the hand `TState::calc` it
//! mirrors is what makes bit-exactness reviewable; the JIT emitter lifts
//! exactly this arithmetic into CLIF for the closed kind set, op-for-op.
//! These impls are the reference the emitter is diffed against by the hand
//! parity tests; the unit tests on the bodies (macd's seed rule) pin the
//! lifecycle independently of the engine.
//!
//! Contract:
//!
//! * inputs arrive **gated** (NaN before each interface node's defined-from
//!   lead) and every call sees every input reporting, so a body is pure
//!   straight-line hand math with no NaN handling;
//! * tail state is **minimal and private** — never the hand `State<Warm>`
//!   with interface-owned fields inside it (single authority by type);
//! * `calc_cse` bodies mirror their hand `TState::calc` ops in the same
//!   order — bit-exactness by construction, gated by the parity tests.

/// One indicator tail. `N_IN`/`N_OUT` are consts so the emitter can validate
/// the recipe wiring against the body at build.
pub trait Tail {
    /// Interface values consumed (in `Recipe::tail_inputs` order).
    const N_IN: usize;
    /// Outputs produced, in sink declaration order (primaries then the
    /// optionals this tail computes).
    const N_OUT: usize;
    /// Advance one bar: read `ins` (gated interface EMA values), write
    /// exactly `N_OUT` outputs.
    fn calc_cse(&mut self, ins: &[f64], outs: &mut [f64]);
}
