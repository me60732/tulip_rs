//! E4: the process-wide **plan cache** — content-addressed by the
//! serialized [`Request`].
//!
//! Compiling a plan shape costs ~0.4ms (`prepare/jit_compile`) while
//! running it over 5000 bars costs ~10-75µs: any caller that rebuilds a
//! query per chunk/tick pays a 40-100× tax. `Request::build_cached`
//! dedups identical shapes to ONE compiled plan handed out as
//! `Arc<JitPlan>`; lookup is a hash + clone of the Arc.
//!
//! # The key
//! [`Request::canonical_bytes`] serializes everything that affects
//! compilation — node graphs (tagged fields, SSA order, recipe order),
//! sink wiring + publish leads, the optional-output switch, the restorer's
//! presence (not its address: keeps the fingerprint stable across process
//! restarts for the DB-reload use), `kind` labels, and the asset-lane
//! count. Same bytes ⇒ same merge ⇒ same arena ids ⇒ same plan: identical
//! content is *the* correctness condition, so no invalidation policy is
//! needed and entries are immutable.
//!
//! # Fingerprint = the streaming safety net
//! [`crate::cse::JitPlan::fingerprint`] is `hash64` of the key (SipHash
//! with fixed keys — deterministic across processes). Every
//! `JitState` remembers the fingerprint of the plan that created it and
//! `advance`/`resume` panic on mismatch: a state image persisted against
//! one arena must never be fed to a plan compiled from a *different*
//! `Request` (the ids would mean different things — Handover §4).
//!
//! # Eviction
//! None: compiled machine code lives for the process. Query shapes are
//! configuration (a handful per deployment), not per-customer growth — if
//! that ever changes, wrap the map with a bounded policy; the cache is the
//! only owner of this decision.
//!
//! # Thread safety
//! `JitPlan` is `Send + Sync` by justified `unsafe impl` (jit.rs): the
//! `JITModule` field is compile-time-only bookkeeping and executable-memory
//! ownership — the run path reads plan data and calls the finalized
//! function with caller-owned state, exactly like the unsynchronized
//! hand code it replaced.

use super::jit::JitPlan;
use super::query::Request;
use std::collections::HashMap;
use std::hash::Hasher;
use std::sync::{Arc, OnceLock, RwLock};

/// Deterministic 64-bit digest of the canonical request bytes
/// (fixed-key SipHash — SAME value in every process for the same bytes).
pub(crate) fn hash64(bytes: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    h.write(bytes);
    h.finish()
}

static PLAN_CACHE: OnceLock<RwLock<HashMap<Vec<u8>, Arc<JitPlan>>>> = OnceLock::new();

/// Cached entry point: compile `req` (over `n_assets` streams) once per
/// distinct canonical key, share thereafter.
pub fn cached_plan(req: &Request, n_assets: usize) -> Arc<JitPlan> {
    let key = req.canonical_bytes(n_assets);
    let map = PLAN_CACHE.get_or_init(Default::default);
    if let Some(hit) = map.read().expect("plan cache poisoned").get(&key) {
        return hit.clone();
    }
    let plan = Arc::new(JitPlan::compile_assets(req, n_assets));
    // A racing duplicate compile is harmless (content-addressed, equal
    // plans): keep the FIRST insert so `Arc::ptr_eq` identifies the shape.
    let mut w = map.write().expect("plan cache poisoned");
    w.entry(key).or_insert_with(|| plan.clone()).clone()
}

/// `hash64(canonical_bytes)` — what plans stamp and states carry.
pub(crate) fn fingerprint_of(req: &Request, n_assets: usize) -> u64 {
    hash64(&req.canonical_bytes(n_assets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cse::Planable;
    use crate::indicators::apo::Apo;
    use crate::indicators::ppo::Ppo;

    fn apo() -> Request {
        Request::new().add_all(Apo::recipe(&[12.0, 26.0]))
    }

    #[test]
    fn cached_plan_hits_return_the_same_plan() {
        let a = cached_plan(&apo(), 1);
        let b = cached_plan(&apo(), 1);
        assert!(Arc::ptr_eq(&a, &b), "identical shapes must dedup to one compile");
        // ...and it runs: bit-for-bit the freshly-built answer
        let c: Vec<f64> = (0..200).map(|i| (i as f64 * 0.7).sin() * 10.0 + 100.0).collect();
        let s: [&[f64]; 1] = [c.as_slice()];
        let (r1, _) = a.run(&s);
        let (r2, _) = apo().build().run(&s);
        assert_eq!(r1.outputs[0][0].len(), r2.outputs[0][0].len());
        for i in 0..r1.outputs[0][0].len() {
            assert_eq!(r1.outputs[0][0][i].to_bits(), r2.outputs[0][0][i].to_bits());
        }
    }

    #[test]
    fn cache_keys_cover_the_switches_that_change_codegen() {
        // optional-output switch: same recipes, different flags ⇒ different plan
        let on = Request::new().add_all(Apo::recipe(&[12.0, 26.0]));
        let off = Request::new().add(Apo::recipe(&[12.0, 26.0]), None);
        let a = cached_plan(&on, 1);
        let b = cached_plan(&off, 1);
        assert!(!Arc::ptr_eq(&a, &b));
        assert_ne!(a.fingerprint(), b.fingerprint());
        // recipe ORDER changes arena ids ⇒ different plan
        let ab = Request::new().add_all(Apo::recipe(&[12.0, 26.0])).add_all(Ppo::recipe(&[12.0, 26.0]));
        let ba = Request::new().add_all(Ppo::recipe(&[12.0, 26.0])).add_all(Apo::recipe(&[12.0, 26.0]));
        assert_ne!(cached_plan(&ab, 1).fingerprint(), cached_plan(&ba, 1).fingerprint());
        // period values, too
        assert_ne!(
            cached_plan(&Request::new().add_all(Apo::recipe(&[12.0, 26.0])), 1).fingerprint(),
            cached_plan(&Request::new().add_all(Apo::recipe(&[12.0, 26.1])), 1).fingerprint()
        );
        // ...and the asset-lane count
        assert_ne!(cached_plan(&ab, 1).fingerprint(), cached_plan(&ab, 2).fingerprint());
    }

    #[test]
    fn foreign_state_cannot_resume_a_different_plan() {
        let small = cached_plan(&apo(), 1);
        let big = cached_plan(
            &Request::new().add_all(Apo::recipe(&[12.0, 26.0])).add_all(Ppo::recipe(&[12.0, 26.0])),
            1,
        );
        let c: Vec<f64> = (0..200).map(|i| (i as f64 * 0.7).sin() * 10.0 + 100.0).collect();
        let s: [&[f64]; 1] = [c.as_slice()];
        let (_res, st) = small.run(&s);
        // st's plan_key belongs to `small`'s arena — handing it to `big`
        // would reinterpret state words against foreign ids: must panic,
        // not compute garbage.
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            big.resume(st, &s);
        }));
        assert!(r.is_err(), "fingerprint assert must reject mismatched JitState");
    }

    #[test]
    fn cached_plans_stream_normally() {
        let p = cached_plan(&apo(), 1);
        let c: Vec<f64> = (0..300).map(|i| (i as f64 * 0.7).sin() * 10.0 + 100.0).collect();
        let s1: [&[f64]; 1] = [&c[..140]];
        let s2: [&[f64]; 1] = [&c[140..]];
        let (h1, qs) = p.run(&s1);
        let (h2, _) = p.resume(qs, &s2);
        let (full, _) = p.run(&[c.as_slice()]);
        let mut cat = h1.outputs[0][0].to_vec();
        cat.extend_from_slice(&h2.outputs[0][0]);
        assert_eq!(cat.len(), full.outputs[0][0].len());
        for i in 0..cat.len() {
            assert_eq!(cat[i].to_bits(), full.outputs[0][0][i].to_bits());
        }
    }
}
