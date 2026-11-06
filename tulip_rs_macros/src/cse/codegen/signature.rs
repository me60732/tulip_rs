use super::*;

pub(super) fn sig_const(p: &Pattern) -> TokenStream {
    let name = format_ident!("SIG_{}", p.name);
    let kinds: Vec<u8> = p.nodes.iter().map(|x| x.kind.tag()).collect();
    let a: Vec<u32> = p.nodes.iter().map(|x| x.a as u32).collect();
    let b: Vec<u32> = p.nodes.iter().map(|x| x.b as u32).collect();
    // For SuperTrend, Fma, and the Where guards, c is the third operand id;
    // for others it's 0
    let c: Vec<u32> = p
        .nodes
        .iter()
        .map(|x| {
            if matches!(
                x.kind,
                Kind::SuperTrend
                    | Kind::Fma
                    | Kind::WhereGe
                    | Kind::WhereLt
                    | Kind::WhereGt
                    | Kind::WhereLe
                    | Kind::IirClamped
                    | Kind::IirClampedPlain
                    | Kind::KvoFlow
                    | Kind::NviRecur
                    | Kind::PviRecur
                    | Kind::VariableAlpha
            ) {
                x.c.unwrap_or(0) as u32
            } else {
                0u32
            }
        })
        .collect();
    // K_IIRCLAMPED's lo_t/lo_v refs (every other kind: 0, matching erase).
    // For Mama/Fama, d/e are unused at runtime - set to 0.
    let d: Vec<u32> = p
        .nodes
        .iter()
        .map(|x| {
            if x.kind == Kind::Mama || x.kind == Kind::Fama {
                0u32
            } else {
                x.d as u32
            }
        })
        .collect();
    let e: Vec<u32> = p
        .nodes
        .iter()
        .map(|x| {
            if x.kind == Kind::Mama || x.kind == Kind::Fama {
                0u32
            } else {
                x.e as u32
            }
        })
        .collect();
    let sinks: Vec<u32> = p.sinks.iter().map(|s| *s as u32).collect();
    // Compute n_streams from Input nodes (max stream index + 1)
    let max_stream = p
        .nodes
        .iter()
        .filter_map(|n| {
            if matches!(n.kind, Kind::Input) {
                Some(n.a)
            } else {
                None
            }
        })
        .fold(0usize, |acc, s| acc.max(s));
    let n_streams = max_stream as u32 + 1;
    quote! {
        pub const #name: crate::cse::Signature = crate::cse::Signature::build(
            &[#(#kinds),*],
            &[#(#a),*],
            &[#(#b),*],
            &[#(#c),*],
            &[#(#d),*],
            &[#(#e),*],
            &[#(#sinks),*],
            #n_streams,
        );
    }
}
