//! The KERNEL side of the engine: everything that generates or *is* a
//! compiled kernel.
//!
//! * [`registry`](registry) — the single `kernel!` call site. At compile
//!   time it expands to, per pattern: the `SIG_*` erased-shape consts, the
//!   scalar fused kernel, the `_assets<const L>` lane kernel (non-packed
//!   patterns), and the `try_run`/`try_run_assets` dispatchers. The module
//!   is `pub`, so any generated kernel is directly callable for A/B work
//!   (`cse::kernel::registry::atr_natr_shared_p(&vals, streams)`); routing
//!   callers use the curated re-exports below / on the `cse` facade.
//! * [`warmup`](warmup) — the const-generic machine `exec` embeds into
//!   every generated kernel as its warm bars `[0, gate)` (the hand's
//!   `init_state` equivalent: folds the carries, writes the row heads).
//!   Its former full-series `run` wrapper (oracle + bench rows) was retired
//!   2026-10-12: parity is gated generated-kernel-vs-hand, and nothing may
//!   interpret a whole series at runtime.

pub mod registry;
pub mod warmup;

pub use warmup::{exec, RingSet};

/// The dispatchers + the direct-call twins the benches A/B (scalar/packed
/// pairs share one Signature, so `try_run` cannot select between them).
pub use registry::{
    atr_multi, atr_multi_p, ema_chains, ema_chains_p, ema_six, ema_six_p, sma_six, sma_six_p,
    try_run, try_run_assets,
};
pub use registry::{
    SIG_ao_fisher_shared_med, SIG_ao_s_m, SIG_apo_ppo_macd_shared_sl, SIG_apo_ppo_shared_sl,
    SIG_aroon_osc_shared_p, SIG_atr_multi, SIG_atr_natr_shared_p, SIG_bbands_wma_shared_p,
    SIG_cmo_n, SIG_ema_chains, SIG_ema_six, SIG_ema_six_p, SIG_fisher_n, SIG_hma_n,
    SIG_ichimoku_s_m, SIG_max_min_shared_p, SIG_natr_supertrend_shared_p, SIG_rsi_cmo_shared_p,
    SIG_rsi_n, SIG_rsi_stochrsi_shared_p, SIG_sma_six, SIG_sma_stddev_shared_p,
    SIG_sma_wma_shared_p, SIG_stoch_kd, SIG_stoch_willr_donchian_shared_p,
    SIG_willr_donchian_shared_p,
};
