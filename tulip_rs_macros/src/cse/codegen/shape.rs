use super::*;

/// value expression of a slot's current-bar value (uniform `x{k}` binding).
pub(super) fn ev(i: usize, idents: &[String]) -> TokenStream {
    let id = format_ident!("{}", idents[i]);
    quote! { #id }
}
/// The element type a steady loop runs on: `f64` for the scalar kernel,
/// `Simd<f64, L>` for the lane kernel. This is the merge's core — the two
/// renderers carry the SAME arithmetic and used to each hold their own copy
/// of every line below, which is exactly how the paths drifted once (the
/// non-mirror ring split landed on the lane path only). An arm that differs
/// in STRUCTURE, not spelling — `WhereGe`/`WhereLt` (a branch is the
/// hand's NaN contract, a select is the lane form), `Ln` (libm vs the hand's
/// SIMD polynomial), the trail rescans, `SuperTrend`, `Hma` — is deliberately
/// NOT here: those are per-path by design, not by accident.
pub(super) struct Shape {
    pub(super) lanes: bool,
}

impl Shape {
    /// The six shared binary ops. `Max`/`Min` are IEEE maxNum/minNum both ways
    /// — the hand's own NaN rule, spelled `f64::max`/`f64::min` on scalars and
    /// `simd_max`/`simd_min` on lanes (its own split, `ultosc.rs` vs
    /// `ultosc_simd.rs`). Per bar each is ONE instruction on either axis.
    pub(super) fn binop(&self, kind: &Kind, ea: &TokenStream, eb: &TokenStream) -> TokenStream {
        match kind {
            Kind::Sub => quote! { #ea - #eb },
            Kind::Add => quote! { #ea + #eb },
            Kind::Mul => quote! { #ea * #eb },
            Kind::Div => quote! { #ea / #eb },
            Kind::Max => {
                if self.lanes {
                    quote! { #ea.simd_max(#eb) }
                } else {
                    quote! { #ea.max(#eb) }
                }
            }
            Kind::Min => {
                if self.lanes {
                    quote! { #ea.simd_min(#eb) }
                } else {
                    quote! { #ea.min(#eb) }
                }
            }
            other => unreachable!("binop called with {other:?}"),
        }
    }

    /// `|a|` — the same call on both element types (portable_simd provides
    /// `abs` for `Simd<f64, L>`).
    pub(super) fn abs(&self, ea: &TokenStream) -> TokenStream {
        quote! { #ea.abs() }
    }
}

/// One chain step: `st = st.mul_add(first, src * second); st` — one fmul then
/// one fma, in the hand's operand order, element type notwithstanding (both
/// `f64` and `Simd<f64, L>` carry `mul_add`, so the recurrence needs no
/// per-shape spelling; only `binop`'s `max` and the branch/select kinds do).
///
/// `Ema`/`Iir` pass `(inv, mult)`; mean-`Wilders` passes `(mult, inv)` — the
/// SAME two instructions with the magnitudes in the other slots, which is why
/// they are two lanes of one formula rather than two emitter rules (see the
/// `ema::calc` / `wilders::calc` pair in the hand modules). Per-slot: the
/// steady loop pays 1 fmul + 1 fma + 1 register carry.
pub(super) fn chain_step(
    carry: &Ident,
    first: &Ident,
    second: &Ident,
    src: &TokenStream,
) -> TokenStream {
    quote! { #carry = #carry.mul_add(#first, #src * #second); #carry }
}

/// THE ONE LANE-PACKABILITY LAW, stated next to the helper it describes: a node is
/// a lane-group candidate iff its steady step lowers to `chain_step` — one broadcast
/// source, two per-slot weights read from `Values::mults`/`invms`, one carry in
/// `st[k]`. Whatever recipe built it. A group of these is N lanes of ONE formula, so
/// an `Ema(9)` beside an `Ema(21)` beside an `Iir` over the same source is one
/// mul + one fma for all of them, not per chain.
///
/// `Iir` is included because it lowers to the same helper with the same arrangement
/// as `Ema` (`chain_step(&ck, &ik, &mk, &src)`, both sites), so it needs no new group
/// statement — the earlier whitelist naming only Ema/Wilders was an omission, not a
/// judgement. Excluded ON EVIDENCE, not taste:
/// · `IirClamped` carries four clamp thresholds through `b/c/d/e`; a group statement
///   hoists only two per-slot vectors, so the thresholds cannot vary per lane without
///   a new statement.
/// · `Accum`'s multiplier is a NODE (volume), not a value — it hoists no weights at
///   all, so there is nothing to broadcast.
/// · `Sma`/`RollSum`/`Stddev`/`Wma`/rings are a different formula (window fold with
///   age), grouped by the ring machinery; `Sma` is bucketed separately below.
pub(super) fn chain_step_form(kind: Kind) -> bool {
    matches!(kind, Kind::Ema | Kind::Wilders | Kind::Iir)
}
