#![allow(clippy::type_complexity)]
//! Build script for candlestick pattern registry generation
//!
//! This script scans all pattern module files for `#[pattern_template]` attributes
//! and generates a complete const PATTERN_REGISTRY with all patterns wired up.
//!
//! # Generated Registry Structure
//!
//! ## 1. Pattern Definitions (PatternDefinitionRegister)
//! Organizes patterns by bar count (1-5 bars). Patterns within each bar-count
//! array are sorted by (group, trend, forecast_order, name), where:
//!   group: 0=Basic, 1=Doji, 2=Marubozu, 3=SpinningTop  (final bar's candle type)
//!   trend: 0=DOWN, 1=UP  (prev_bar's trend bit)
//! Patterns spanning multiple (group,trend) pairs are duplicated.
//!
//! ## 2. Group-Trend Dispatch (GroupTrendDispatch)
//! One GroupTrendDispatch per forecast type (indexed by ForecastType as usize) plus
//! GLOBAL_GROUP_TREND_DISPATCH for the no-forecast path.
//! Each holds per-bar-count [(start,end); 8] arrays keyed by group*2+trend.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

/// Pattern metadata extracted from source files
#[derive(Debug, Clone)]
struct PatternInfo {
    name: String,
    forecast: String,
    bar_count: usize,
    has_prev_bar: bool,
    lazy_bits_mask: String,
    module_path: String,
    /// Groups (0=Basic,1=Doji,2=Marubozu,3=SpinningTop) the final bar can match.
    final_bar_groups: Vec<usize>,
    /// Colour of the final bar: None=ANY, Some(true)=GREEN, Some(false)=RED
    final_bar_colour: Option<bool>,
    /// Fill of the final bar: None=ANY, Some(true)=HALLOW, Some(false)=FILL
    final_bar_fill: Option<bool>,
    /// Within-bucket sort priority:
    ///   0 = engulfing (has engulf_prev / inside_prev) — sets bits 1-13 via apply_engulfing
    ///   1 = gap       (has body_gap / wick_gap)        — sets bits 1-10 via apply_gap
    ///   2 = other     (no engulf or gap attributes)
    priority: usize,
}

fn main() {
    println!("cargo:rerun-if-changed=src/candle_indicators/candle_patterns");

    let out_dir = env::var("OUT_DIR").unwrap();
    let patterns_dir = Path::new("src/candle_indicators/candle_patterns");

    let patterns = scan_pattern_modules(patterns_dir);
    let patterns_by_bars = organize_patterns(&patterns);
    let registry_code = generate_registry_code(&patterns, &patterns_by_bars);

    let dest_path = Path::new(&out_dir).join("generated_registry.rs");
    fs::write(&dest_path, registry_code).expect("Failed to write generated registry");

    println!(
        "cargo:warning=Generated pattern registry with {} patterns",
        patterns.len()
    );
}

/// Scan all .rs files in the patterns directory for #[pattern_template] attributes
fn scan_pattern_modules(base_dir: &Path) -> Vec<PatternInfo> {
    let mut patterns = Vec::new();

    for bar_dir in &["one_bar", "two_bar", "three_bar", "four_bar", "five_bar"] {
        let dir_path = base_dir.join(bar_dir);

        if !dir_path.exists() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                        if file_name == "mod" {
                            continue;
                        }

                        if let Some(pattern) = extract_pattern_info(&path, bar_dir, file_name) {
                            patterns.push(pattern);
                        }
                    }
                }
            }
        }
    }

    patterns
}

/// Extract pattern metadata from a source file by parsing #[pattern_template] attributes
fn extract_pattern_info(path: &Path, bar_dir: &str, file_name: &str) -> Option<PatternInfo> {
    let content = fs::read_to_string(path).ok()?;

    let mut in_attribute = false;
    let mut attribute_content = String::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("#[pattern_template(") {
            in_attribute = true;
            attribute_content.push_str(trimmed);
            attribute_content.push(' ');
        } else if in_attribute {
            attribute_content.push_str(trimmed);
            attribute_content.push(' ');

            if trimmed.contains(")]") {
                if let Some(info) = parse_attribute_content(&attribute_content, bar_dir, file_name)
                {
                    return Some(info);
                }
                in_attribute = false;
                attribute_content.clear();
            }
        }
    }

    None
}

/// Return the forecast ordering used for sort and array indexing.
/// Must match the `ForecastType` enum discriminant order.
fn forecast_order(f: &str) -> usize {
    match f {
        "BearishReversal" => 0,
        "BullishReversal" => 1,
        "BearishContinuation" => 2,
        "BullishContinuation" => 3,
        "BearishReversalOrContinuation" => 4,
        "BullishReversalOrContinuation" => 5,
        _ => 999,
    }
}

/// Map a forecast type to the set of trend bucket indices (0=DOWN, 1=UP)
/// that the prev_bar can be in for this forecast to apply.
fn forecast_trend_buckets(forecast: &str) -> Vec<usize> {
    match forecast {
        "BearishReversal" => vec![1], // uptrend prev_bar
        "BullishReversal" => vec![0], // downtrend prev_bar
        "BearishContinuation" => vec![0],
        "BullishContinuation" => vec![1],
        "BearishReversalOrContinuation" => vec![0, 1],
        "BullishReversalOrContinuation" => vec![0, 1],
        _ => vec![0, 1], // fallback: both
    }
}

/// Infer fill from a positive (non-negated) candle_type string.
/// "White*" variants → Some(true) = HALLOW, "Black*" variants → Some(false) = FILL.
/// Returns None for negation patterns, mixed types, or Doji/neutral-only types.
fn infer_fill_from_candle_type(candle_type_str: &str) -> Option<bool> {
    let ct = candle_type_str.trim();
    // Negation patterns are too complex to infer fill from
    if ct.contains('!') {
        return None;
    }
    let has_white = ct.contains("White");
    let has_black = ct.contains("Black");
    match (has_white, has_black) {
        (true, false) => Some(true),  // All White variants → HALLOW
        (false, true) => Some(false), // All Black variants → FILL
        _ => None,                    // Mixed, neither (Doji/HighWave), or empty
    }
}

/// Extract a balanced-parentheses block from `s`, where `s[0]` is the opening `(`.
/// Returns the slice `s[0..=close_pos]` including both parens.
fn extract_balanced_parens(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut depth: i32 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return &s[..=i];
                }
            }
            _ => {}
        }
    }
    s // fallback for unbalanced input
}

/// Given a candle_type string and a group name (e.g. "Doji"), extract all variants
/// listed inside `!GroupName(...)` negation blocks for that group.
fn extract_negated_variants(ct: &str, group_name: &str) -> Vec<String> {
    let needle = format!("!{}(", group_name);
    let mut variants = Vec::new();
    let mut search = ct;
    while let Some(pos) = search.find(&needle) {
        let rest = &search[pos + needle.len() - 1..]; // start at the '('
        let block = extract_balanced_parens(rest);
        // Strip outer parens
        let inner = &block[1..block.len().saturating_sub(1)];
        for v in inner.split('|') {
            let v = v.trim();
            if !v.is_empty() {
                variants.push(v.to_string());
            }
        }
        search = &search[pos + needle.len()..];
    }
    variants
}

/// Parse the candle_type string from the final bar block and return the set of
/// candle-type group indices (0=Basic, 1=Doji, 2=Marubozu, 3=SpinningTop) that the
/// pattern can match.
fn parse_candle_type_groups(candle_type_str: &str) -> Vec<usize> {
    let ct = candle_type_str.trim();

    // All variants per group (for checking if a negation covers the full group)
    let all_basic: &[&str] = &[
        "ShortWhiteCandle",
        "WhiteCandle",
        "LongWhiteCandle",
        "ShortBlackCandle",
        "BlackCandle",
        "LongBlackCandle",
    ];
    let all_doji: &[&str] = &[
        "Doji",
        "LongLeggedDoji",
        "DragonflyDoji",
        "GravestoneDoji",
        "FourPriceDoji",
    ];
    let all_marubozu: &[&str] = &[
        "WhiteMarubozu",
        "OpeningWhiteMarubozu",
        "ClosingWhiteMarubozu",
        "BlackMarubozu",
        "OpeningBlackMarubozu",
        "ClosingBlackMarubozu",
    ];
    let all_spinning_top: &[&str] = &["WhiteSpinningTop", "BlackSpinningTop", "HighWave"];

    if ct.is_empty() {
        // No candle_type constraint — matches all groups
        return vec![0, 1, 2, 3];
    }

    // Check whether any negation `!GroupName(...)` exists
    let has_negation = ct.contains('!');

    if has_negation {
        // Start with all 4 groups, then exclude any group whose ALL variants are negated
        let mut groups = vec![0usize, 1, 2, 3];

        let group_data: &[(&str, usize, &[&str])] = &[
            ("Basic", 0, all_basic),
            ("Doji", 1, all_doji),
            ("Marubozu", 2, all_marubozu),
            ("SpinningTop", 3, all_spinning_top),
        ];

        for &(group_name, group_idx, all_variants) in group_data {
            let negated = extract_negated_variants(ct, group_name);
            if negated.is_empty() {
                continue;
            }
            // Check if ALL variants of this group are covered by the negation
            let all_negated = all_variants.iter().all(|v| negated.iter().any(|n| n == v));
            if all_negated {
                groups.retain(|&g| g != group_idx);
            }
        }

        groups
    } else {
        // Positive specification: only include explicitly mentioned groups
        let mut groups = Vec::new();

        if ct.contains("Basic(") {
            groups.push(0);
        }
        if ct.contains("Doji(") {
            groups.push(1);
        }
        if ct.contains("Marubozu(") {
            groups.push(2);
        }
        if ct.contains("SpinningTop(") {
            groups.push(3);
        }

        // If nothing matched, fall back to all groups (shouldn't happen with valid patterns)
        if groups.is_empty() {
            groups = vec![0, 1, 2, 3];
        }

        groups
    }
}

/// Parse the pattern_template attribute content.
/// Extracts: name, forecast, bar_count, has_prev_bar, lazy_bits_mask, final_bar_groups.
fn parse_attribute_content(content: &str, bar_dir: &str, file_name: &str) -> Option<PatternInfo> {
    let mut name = None;
    let mut forecast = None;

    // Extract name
    if let Some(start) = content.find("name = \"") {
        let rest = &content[start + 8..];
        if let Some(end) = rest.find('"') {
            name = Some(rest[..end].to_string());
        }
    }

    // Extract forecast
    if let Some(start) = content.find("forecast = \"") {
        let rest = &content[start + 12..];
        if let Some(end) = rest.find('"') {
            forecast = Some(rest[..end].to_string());
        }
    }

    // Count bar(...) occurrences (excluding prev_bar)
    let bar_count = content.matches("bar(").count() - content.matches("prev_bar(").count();

    // Check if prev_bar is present
    let has_prev_bar = content.contains("prev_bar(");

    // Find the last occurrence of "bar(" that is NOT "prev_bar("
    let mut last_bar_paren: Option<usize> = None;
    let mut search_pos = 0;
    while search_pos < content.len() {
        if let Some(rel) = content[search_pos..].find("bar(") {
            let abs = search_pos + rel;
            // Check whether this "bar(" is actually "prev_bar("
            let is_prev = abs >= 5 && &content[abs - 5..abs] == "prev_";
            if !is_prev {
                last_bar_paren = Some(abs + 3); // index of the '('
            }
            search_pos = abs + 4;
        } else {
            break;
        }
    }

    // Extract candle_type, colour, and fill from the last bar block
    let (final_bar_groups, final_bar_colour, final_bar_fill) =
        if let Some(paren_pos) = last_bar_paren {
            let block = extract_balanced_parens(&content[paren_pos..]);

            // candle_type → group indices
            let candle_type_str = if let Some(start) = block.find("candle_type = \"") {
                let rest = &block[start + 15..];
                if let Some(end) = rest.find('"') {
                    &rest[..end]
                } else {
                    ""
                }
            } else {
                ""
            };
            let groups = parse_candle_type_groups(candle_type_str);

            // colour → None=ANY, Some(true)=GREEN, Some(false)=RED
            let colour = if let Some(start) = block.find("colour = \"") {
                let rest = &block[start + 10..];
                if let Some(end) = rest.find('"') {
                    match &rest[..end] {
                        "GREEN" => Some(true),
                        "RED" => Some(false),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // fill → None=ANY, Some(true)=HALLOW, Some(false)=FILL
            // If not explicitly specified, infer from candle_type (White→HALLOW, Black→FILL)
            let fill = if let Some(start) = block.find("fill = \"") {
                let rest = &block[start + 8..];
                if let Some(end) = rest.find('"') {
                    match &rest[..end] {
                        "HALLOW" => Some(true),
                        "FILL" => Some(false),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                infer_fill_from_candle_type(candle_type_str)
            };

            (groups, colour, fill)
        } else {
            (vec![0, 1, 2, 3], None, None)
        };

    if let (Some(name), Some(forecast)) = (name, forecast) {
        let module_path = format!("candle_patterns::{}::{}", bar_dir, file_name);
        let lazy_bits_mask = format!("PATTERN_LAZY_BITS_{}", name.to_uppercase());

        // Priority within a (group, trend, colour, fill) bucket:
        //   0 = engulfing — apply_engulfing sets bits 1-13 (superset of gap bits)
        //   1 = gap       — apply_gap sets bits 1-10
        //   2 = other
        let priority = if content.contains("engulf_prev") || content.contains("inside_prev") {
            0
        } else if content.contains("body_gap") || content.contains("wick_gap") {
            1
        } else {
            2
        };

        Some(PatternInfo {
            name,
            forecast,
            bar_count,
            has_prev_bar,
            lazy_bits_mask,
            module_path,
            final_bar_groups,
            final_bar_colour,
            final_bar_fill,
            priority,
        })
    } else {
        None
    }
}

/// Organize patterns into (group, trend) entries sorted by (group, trend, forecast_order, name).
/// Patterns spanning multiple (group, trend) pairs are duplicated — one entry per pair.
fn organize_patterns(
    patterns: &[PatternInfo],
) -> HashMap<usize, Vec<(usize, usize, usize, usize, &PatternInfo)>> {
    let mut by_bars: HashMap<usize, Vec<(usize, usize, usize, usize, &PatternInfo)>> =
        HashMap::new();

    for pattern in patterns {
        let trends = forecast_trend_buckets(&pattern.forecast);

        // colour: 0=RED, 1=GREEN — None duplicates into both
        let colours: &[usize] = match pattern.final_bar_colour {
            Some(true) => &[1],
            Some(false) => &[0],
            None => &[0, 1],
        };
        // fill: 0=FILL, 1=HALLOW — None duplicates into both
        let fills: &[usize] = match pattern.final_bar_fill {
            Some(true) => &[1],
            Some(false) => &[0],
            None => &[0, 1],
        };

        let entries = by_bars.entry(pattern.bar_count).or_default();
        for &g in &pattern.final_bar_groups {
            for &t in &trends {
                for &c in colours {
                    for &f in fills {
                        entries.push((g, t, c, f, pattern));
                    }
                }
            }
        }
    }

    for entries in by_bars.values_mut() {
        entries.sort_by(|a, b| {
            a.0.cmp(&b.0)  // group
                .then_with(|| a.1.cmp(&b.1))  // trend
                .then_with(|| a.2.cmp(&b.2))  // colour
                .then_with(|| a.3.cmp(&b.3))  // fill
                .then_with(|| forecast_order(&a.4.forecast).cmp(&forecast_order(&b.4.forecast)))
                .then_with(|| a.4.priority.cmp(&b.4.priority))  // engulf first, gap second, other last
                .then_with(|| a.4.name.cmp(&b.4.name))
        });
    }

    by_bars
}

/// Compute the [(start, end); 32] dispatch array for all patterns in `entries`.
/// Entries must be sorted by (group, trend, colour, fill, ...).
/// Key = group*8 + trend*4 + colour*2 + fill  (32 total keys)
fn compute_gtcf_dispatch(
    entries: &[(usize, usize, usize, usize, &PatternInfo)],
) -> [(usize, usize); 32] {
    let mut dispatch = [(0usize, 0usize); 32];
    for g in 0..4usize {
        for t in 0..2usize {
            for c in 0..2usize {
                for f in 0..2usize {
                    let key = g * 8 + t * 4 + c * 2 + f;
                    let start =
                        entries.partition_point(|e| e.0 * 8 + e.1 * 4 + e.2 * 2 + e.3 < key);
                    let end = entries.partition_point(|e| e.0 * 8 + e.1 * 4 + e.2 * 2 + e.3 <= key);
                    dispatch[key] = (start, end);
                }
            }
        }
    }
    dispatch
}

/// Compute the [(start, end); 32] dispatch array for a specific forecast type.
/// Within each (group, trend, colour, fill) block the range covers only entries
/// whose forecast matches `forecast_str`.
/// Key = group*8 + trend*4 + colour*2 + fill  (32 total keys)
fn compute_forecast_gtcf_dispatch(
    entries: &[(usize, usize, usize, usize, &PatternInfo)],
    forecast_str: &str,
) -> [(usize, usize); 32] {
    let f_order = forecast_order(forecast_str);
    let mut dispatch = [(0usize, 0usize); 32];
    for g in 0..4usize {
        for t in 0..2usize {
            for c in 0..2usize {
                for f in 0..2usize {
                    let key = g * 8 + t * 4 + c * 2 + f;
                    let block_start =
                        entries.partition_point(|e| e.0 * 8 + e.1 * 4 + e.2 * 2 + e.3 < key);
                    let block_end =
                        entries.partition_point(|e| e.0 * 8 + e.1 * 4 + e.2 * 2 + e.3 <= key);
                    let block = &entries[block_start..block_end];
                    let fc_start = block_start
                        + block.partition_point(|e| forecast_order(&e.4.forecast) < f_order);
                    let fc_end = block_start
                        + block.partition_point(|e| forecast_order(&e.4.forecast) <= f_order);
                    dispatch[key] = (fc_start, fc_end);
                }
            }
        }
    }
    dispatch
}

/// Format a [(usize, usize); 8] dispatch array as a Rust literal.
fn emit_gt_array(d: [(usize, usize); 32]) -> String {
    let pairs: Vec<String> = d.iter().map(|(s, e)| format!("({},{})", s, e)).collect();
    format!("[{}]", pairs.join(","))
}

/// Generate the complete registry code
fn generate_registry_code(
    patterns: &[PatternInfo],
    patterns_by_bars: &HashMap<usize, Vec<(usize, usize, usize, usize, &PatternInfo)>>,
) -> String {
    let mut code = String::new();

    // Header
    code.push_str("// AUTO-GENERATED by build.rs - DO NOT EDIT\n");
    code.push_str("// This file is generated at build time by scanning pattern modules\n\n");

    // Imports
    code.push_str("use crate::candle_indicators::{\n");
    code.push_str("    registry::*,\n");
    code.push_str("    types::ForecastType,\n");
    code.push_str("    pattern_test::EmaState,\n");
    code.push_str("    types::CandleInfo,\n");
    for pattern in patterns {
        code.push_str(&format!("    {},\n", pattern.module_path));
    }
    code.push_str("};\n\n");

    // CandlePattern enum
    code.push_str("/// Candlestick pattern enum - auto-generated from pattern modules\n");
    code.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
    code.push_str("pub enum CandlePattern {\n");
    for pattern in patterns {
        code.push_str(&format!("    {},\n", pattern.name));
    }
    code.push_str("}\n\n");

    // CandlePattern dispatch impls
    code.push_str("impl CandlePattern {\n");

    // calc()
    code.push_str("    pub fn calc(\n");
    code.push_str("        &self,\n");
    code.push_str("        inputs: (&[f64], &[f64], &[f64], &[f64]),\n");
    code.push_str("        i: usize,\n");
    code.push_str("        state: &EmaState,\n");
    code.push_str("        bars: &[CandleBits],\n");
    code.push_str("    ) -> bool {\n");
    code.push_str("        let (open, high, low, close) = inputs;\n");
    code.push_str("        match self {\n");
    for pattern in patterns {
        let module = pattern.module_path.split("::").last().unwrap();
        let slice_start = format!("i-{}", pattern.bar_count);
        code.push_str(&format!(
            "            CandlePattern::{} => {}::calc((&open[{}..=i], &high[{}..=i], &low[{}..=i], &close[{}..=i]), state, &bars[..]),\n",
            pattern.name, module, slice_start, slice_start, slice_start, slice_start
        ));
    }
    code.push_str("        }\n");
    code.push_str("    }\n\n");

    // get_info()
    code.push_str("    pub fn get_info(&self) -> CandleInfo {\n");
    code.push_str("        match self {\n");
    for pattern in patterns {
        let module = pattern.module_path.split("::").last().unwrap();
        code.push_str(&format!(
            "            CandlePattern::{} => {}::info(),\n",
            pattern.name, module
        ));
    }
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // PatternDefinition constants (one per pattern)
    code.push_str("// Pattern definition constants\n");
    for pattern in patterns {
        let module = pattern.module_path.split("::").last().unwrap();
        let const_name = format!("PATTERN_DEF_{}", pattern.name.to_uppercase());
        let masks_name = format!("PATTERN_MASKS_{}", pattern.name.to_uppercase());
        let total_masks = pattern.bar_count + 1;

        code.push_str(&format!("/// Pattern definition for {}\n", pattern.name));
        code.push_str(&format!(
            "pub const {}: PatternDefinition<{}> = PatternDefinition::new(\n",
            const_name, total_masks
        ));
        code.push_str(&format!("    CandlePattern::{},\n", pattern.name));
        code.push_str(&format!("    ForecastType::{},\n", pattern.forecast));
        code.push_str(&format!("    {}::{},\n", module, masks_name));
        code.push_str(&format!("    {},\n", pattern.has_prev_bar));
        code.push_str(&format!("    {}::{},\n", module, pattern.lazy_bits_mask));
        code.push_str(");\n\n");
    }

    // -------------------------------------------------------------------------
    // Organize entries into (group, trend) order with duplication
    // -------------------------------------------------------------------------
    let entries1 = if let Some(v) = patterns_by_bars.get(&1) {
        v.as_slice()
    } else {
        &[]
    };
    let entries2 = if let Some(v) = patterns_by_bars.get(&2) {
        v.as_slice()
    } else {
        &[]
    };
    let entries3 = if let Some(v) = patterns_by_bars.get(&3) {
        v.as_slice()
    } else {
        &[]
    };
    let entries4 = if let Some(v) = patterns_by_bars.get(&4) {
        v.as_slice()
    } else {
        &[]
    };
    let entries5 = if let Some(v) = patterns_by_bars.get(&5) {
        v.as_slice()
    } else {
        &[]
    };

    let total1 = entries1.len();
    let total2 = entries2.len();
    let total3 = entries3.len();
    let total4 = entries4.len();
    let total5 = entries5.len();

    // Diagnostic: report per-key counts for two_bar
    {
        let mut key_counts = [0usize; 32];
        for e in entries2 {
            key_counts[e.0 * 8 + e.1 * 4 + e.2 * 2 + e.3] += 1;
        }
        let group_names = ["Basic", "Doji", "Marubozu", "SpinTop"];
        let trend_names = ["DOWN", "UP"];
        let colour_names = ["RED", "GREEN"];
        let fill_names = ["FILL", "HALLOW"];
        for (g, group_name) in group_names.iter().enumerate() {
            for (t, trend_name) in trend_names.iter().enumerate() {
                for (c, colour_name) in colour_names.iter().enumerate() {
                    for (f, fill_name) in fill_names.iter().enumerate() {
                        let k = g * 8 + t * 4 + c * 2 + f;
                        if key_counts[k] > 0 {
                            println!(
                                "cargo:warning=two_bar [{} {} {} {}] key={} count={}",
                                group_name, trend_name, colour_name, fill_name, k, key_counts[k]
                            );
                        }
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // PATTERN_DEFINITIONS — arrays in (group, trend, forecast_order, name) order
    // -------------------------------------------------------------------------
    code.push_str("/// Pattern definitions organized by bar count\n");
    code.push_str(&format!(
        "pub const PATTERN_DEFINITIONS: PatternDefinitionRegister<{}, {}, {}, {}, {}> = PatternDefinitionRegister {{\n",
        total1, total2, total3, total4, total5
    ));

    for (field_name, entries) in &[
        ("one_bar", entries1),
        ("two_bar", entries2),
        ("three_bar", entries3),
        ("four_bar", entries4),
        ("five_bar", entries5),
    ] {
        code.push_str(&format!("    {}: [", field_name));
        for (_, _, _, _, p) in entries.iter() {
            code.push_str(&format!("PATTERN_DEF_{}, ", p.name.to_uppercase()));
        }
        code.push_str("],\n");
    }
    code.push_str("};\n\n");

    // -------------------------------------------------------------------------
    // GroupTrendDispatch constants — one per forecast type + global
    // -------------------------------------------------------------------------
    let all_forecast_types: [&str; 6] = [
        "BearishReversal",
        "BullishReversal",
        "BearishContinuation",
        "BullishContinuation",
        "BearishReversalOrContinuation",
        "BullishReversalOrContinuation",
    ];

    code.push_str("// Group-trend-colour-fill dispatch constants per forecast type\n");
    for &fc in &all_forecast_types {
        let d1 = compute_forecast_gtcf_dispatch(entries1, fc);
        let d2 = compute_forecast_gtcf_dispatch(entries2, fc);
        let d3 = compute_forecast_gtcf_dispatch(entries3, fc);
        let d4 = compute_forecast_gtcf_dispatch(entries4, fc);
        let d5 = compute_forecast_gtcf_dispatch(entries5, fc);
        code.push_str(&format!(
            "const FORECAST_GTD_{}: GroupTrendDispatch = GroupTrendDispatch::new(\n    {},\n    {},\n    {},\n    {},\n    {}\n);\n\n",
            fc.to_uppercase(),
            emit_gt_array(d1),
            emit_gt_array(d2),
            emit_gt_array(d3),
            emit_gt_array(d4),
            emit_gt_array(d5),
        ));
    }

    // Global dispatch
    let gd1 = compute_gtcf_dispatch(entries1);
    let gd2 = compute_gtcf_dispatch(entries2);
    let gd3 = compute_gtcf_dispatch(entries3);
    let gd4 = compute_gtcf_dispatch(entries4);
    let gd5 = compute_gtcf_dispatch(entries5);
    code.push_str(&format!(
        "const GLOBAL_GROUP_TREND_DISPATCH: GroupTrendDispatch = GroupTrendDispatch::new(\n    {},\n    {},\n    {},\n    {},\n    {}\n);\n\n",
        emit_gt_array(gd1),
        emit_gt_array(gd2),
        emit_gt_array(gd3),
        emit_gt_array(gd4),
        emit_gt_array(gd5),
    ));

    // FORECAST_DISPATCHES array
    code.push_str("pub const FORECAST_DISPATCHES: [GroupTrendDispatch; 6] = [\n");
    for &fc in &all_forecast_types {
        code.push_str(&format!("    FORECAST_GTD_{},\n", fc.to_uppercase()));
    }
    code.push_str("];\n\n");

    // PATTERN_REGISTRY
    code.push_str("/// The global pattern registry - fully populated at compile time\n");
    code.push_str(&format!(
        "pub const PATTERN_REGISTRY: PatternRegister<{}, {}, {}, {}, {}> = PatternRegister::new(\n",
        total1, total2, total3, total4, total5
    ));
    code.push_str("    PATTERN_DEFINITIONS,\n");
    code.push_str("    FORECAST_DISPATCHES,\n");
    code.push_str("    GLOBAL_GROUP_TREND_DISPATCH,\n");
    code.push_str(");\n\n");

    // get_global_registry helper
    code.push_str("/// Get the global pattern registry\n");
    code.push_str(&format!(
        "pub const fn get_global_registry() -> &'static PatternRegister<{}, {}, {}, {}, {}> {{\n",
        total1, total2, total3, total4, total5
    ));
    code.push_str("    &PATTERN_REGISTRY\n");
    code.push_str("}\n");

    code
}
