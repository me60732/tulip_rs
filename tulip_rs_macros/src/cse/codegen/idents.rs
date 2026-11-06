use super::*;

/// Build identifier for a slot based on its optional name.
/// Sanitize: lowercase, non-alphanumeric→`_`, leading digit→prefix `_`.
/// Deduplicate: on collision append `_{k}` to the LATER node's ident.
pub(super) fn build_idents(pattern: &Pattern) -> Vec<String> {
    let mut idents: Vec<String> = Vec::with_capacity(pattern.nodes.len());
    let mut used: HashMap<String, usize> = HashMap::new();

    // Rust keywords that need suffix
    const RUST_KEYWORDS: &[&str] = &[
        "type", "const", "let", "if", "else", "match", "loop", "move", "ref", "mut", "fn", "pub",
        "use", "mod", "self", "super", "crate", "as", "in", "box", "where", "dyn", "async",
        "await", "static", "extern", "unsafe", "struct", "enum", "impl", "trait", "virtual",
        "final", "macro", "return", "yield", "break", "continue",
    ];

    for (i, opt_name) in pattern.names.iter().enumerate() {
        let ident = if let Some(name) = opt_name {
            // Sanitize: lowercase
            let mut s = name.to_lowercase();

            // Replace non-alphanumeric with _
            s = s
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect::<String>();

            // If starts with digit, prefix with _
            if s.starts_with(|c: char| c.is_ascii_digit()) {
                s.insert(0, '_');
            }

            // If is a Rust keyword, append _
            if RUST_KEYWORDS.contains(&s.as_str()) {
                s.push('_');
            }

            s
        } else {
            format!("x{}", i)
        };

        // Deduplicate: on collision, append _{k} where k is the next available
        let base = ident.clone();
        let mut final_ident = ident;
        while used.contains_key(&final_ident) {
            final_ident = format!("{}_{}", base, used[&base]);
            used.insert(base.clone(), used[&base] + 1);
        }
        used.insert(final_ident.clone(), 1);
        idents.push(final_ident);
    }

    idents
}
