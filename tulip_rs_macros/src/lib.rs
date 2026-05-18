//! Proc macros for candlestick pattern registration
//!
//! This crate provides the `#[pattern_template]` attribute macro that generates
//! const pattern definitions in each pattern module.
//!
//! The build script then scans all modules and generates the final registry.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, LitStr, Token,
};

/// Parse a single bar requirement from attribute syntax
struct BarRequirement {
    trend: Option<String>,
    colour: Option<String>,
    fill: Option<String>,
    candle_type: Option<String>,
    body_height: Option<String>,
    line_height: Option<String>,
    body_gap: Option<String>,
    wick_gap: Option<String>,
}

impl Parse for BarRequirement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut trend = None;
        let mut colour = None;
        let mut fill = None;
        let mut candle_type = None;
        let mut body_height = None;
        let mut line_height = None;
        let mut body_gap = None;
        let mut wick_gap = None;

        // Expect: bar(key = "value", ...)
        let content;
        syn::parenthesized!(content in input);

        while !content.is_empty() {
            let key: Ident = content.parse()?;
            let _: Token![=] = content.parse()?;
            let value: LitStr = content.parse()?;

            match key.to_string().as_str() {
                "trend" => trend = Some(value.value()),
                "colour" => colour = Some(value.value()),
                "fill" => fill = Some(value.value()),
                "candle_type" => candle_type = Some(value.value()),
                "body_height" => body_height = Some(value.value()),
                "line_height" => line_height = Some(value.value()),
                "body_gap" => body_gap = Some(value.value()),
                "wick_gap" => wick_gap = Some(value.value()),
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "Unknown bar attribute. Expected: trend, colour, fill, candle_type, body_height, line_height, body_gap, or wick_gap",
                    ))
                }
            }

            if content.peek(Token![,]) {
                let _: Token![,] = content.parse()?;
            }
        }

        Ok(BarRequirement {
            trend,
            colour,
            fill,
            candle_type,
            body_height,
            line_height,
            body_gap,
            wick_gap,
        })
    }
}

/// Parse the pattern_template attribute
struct PatternTemplate {
    name: String,
    forecast: String,
    prev_bar: Option<BarRequirement>, // NEW: optional prev_bar constraint
    bars: Vec<BarRequirement>,
}

impl Parse for PatternTemplate {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut forecast = None;
        let mut prev_bar = None; // NEW
        let mut bars = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;

            if key == "name" {
                let _: Token![=] = input.parse()?;
                let value: LitStr = input.parse()?;
                name = Some(value.value());
            } else if key == "forecast" {
                let _: Token![=] = input.parse()?;
                let value: LitStr = input.parse()?;
                forecast = Some(value.value());
            } else if key == "prev_bar" {
                // NEW
                let bar: BarRequirement = input.parse()?;
                prev_bar = Some(bar);
            } else if key == "bar" {
                let bar: BarRequirement = input.parse()?;
                bars.push(bar);
            } else {
                return Err(syn::Error::new_spanned(
                    key,
                    "Unknown attribute. Expected: name, forecast, prev_bar, or bar",
                ));
            }

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let name = name.ok_or_else(|| input.error("Missing required 'name' attribute"))?;
        let forecast =
            forecast.ok_or_else(|| input.error("Missing required 'forecast' attribute"))?;

        if bars.is_empty() {
            return Err(input.error("Pattern must have at least one bar requirement"));
        }

        Ok(PatternTemplate {
            name,
            forecast,
            prev_bar, // NEW
            bars,
        })
    }
}

/// Get the bit position for a candle variant
fn get_variant_bit_position(kind: &str, variant: &str) -> Option<u8> {
    match kind {
        "Basic" => match variant {
            "ShortWhiteCandle" => Some(0),
            "WhiteCandle" => Some(1),
            "LongWhiteCandle" => Some(2),
            "ShortBlackCandle" => Some(3),
            "BlackCandle" => Some(4),
            "LongBlackCandle" => Some(5),
            _ => None,
        },
        "Doji" => match variant {
            "Doji" => Some(0),
            "LongLeggedDoji" => Some(1),
            "DragonflyDoji" => Some(2),
            "GravestoneDoji" => Some(3),
            "FourPriceDoji" => Some(4),
            _ => None,
        },
        "Marubozu" => match variant {
            "WhiteMarubozu" => Some(0),
            "OpeningWhiteMarubozu" => Some(1),
            "ClosingWhiteMarubozu" => Some(2),
            "BlackMarubozu" => Some(3),
            "OpeningBlackMarubozu" => Some(4),
            "ClosingBlackMarubozu" => Some(5),
            _ => None,
        },
        "SpinningTop" => match variant {
            "WhiteSpinningTop" => Some(0),
            "BlackSpinningTop" => Some(1),
            "HighWave" => Some(2),
            _ => None,
        },
        _ => None,
    }
}

/// Parse a single Type(Variant | Variant) expression
fn parse_single_type_group(type_group: &str) -> Option<proc_macro2::TokenStream> {
    let open_paren = type_group.find('(')?;
    let close_paren = type_group.rfind(')')?;

    let kind = type_group[..open_paren].trim();
    let variants_str = &type_group[open_paren + 1..close_paren];

    let kind_ident = syn::Ident::new(kind, proc_macro2::Span::call_site());

    // Parse variants: "WhiteCandle | LongWhiteCandle"
    let variant_names: Vec<&str> = variants_str
        .split('|')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    // Generate bit values directly using bit positions
    let bits: Vec<_> = variant_names
        .iter()
        .filter_map(|v| {
            get_variant_bit_position(kind, v).map(|bit_pos| {
                let bit_val = 1u8 << bit_pos;
                quote! { #bit_val }
            })
        })
        .collect();

    if bits.is_empty() {
        return None;
    }

    let bitmask = if bits.len() == 1 {
        let bit = &bits[0];
        quote! { #bit }
    } else {
        quote! { #(#bits)|* }
    };

    Some(quote! {
        crate::candle_indicators::types::CandleTypePattern::#kind_ident(#bitmask)
    })
}

/// Convert candle_type string to a Vec of (CandleTypePattern expressions, negated flag)
///
/// Supports negation with ! prefix:
/// "!Doji(Doji | LongLeggedDoji)" means match anything EXCEPT those Doji types
///
/// Supports multiple type groups separated by whitespace:
/// "Basic(WhiteCandle | LongWhiteCandle) Marubozu(OpeningWhiteMarubozu | ClosingWhiteMarubozu)"
///
/// Returns a vector of (parsed type groups, is_negated). When multiple groups exist, they should be
/// chained as separate with_candle_type() or with_negated_candle_type() calls.
fn parse_candle_type(candle_type_str: &str) -> Vec<(proc_macro2::TokenStream, bool)> {
    if candle_type_str.is_empty() {
        return vec![(
            quote! { crate::candle_indicators::types::CandleTypePattern::Any },
            false,
        )];
    }

    // Split into individual Type(...) groups
    // We need to respect parentheses when splitting
    let mut type_groups = Vec::new();
    let mut current_group = String::new();
    let mut paren_depth = 0;
    let mut has_negation = false;

    for ch in candle_type_str.chars() {
        match ch {
            '!' if paren_depth == 0 && current_group.trim().is_empty() => {
                // Negation prefix at the start of a group
                has_negation = true;
            }
            '(' => {
                paren_depth += 1;
                current_group.push(ch);
            }
            ')' => {
                paren_depth -= 1;
                current_group.push(ch);
                if paren_depth == 0 && !current_group.trim().is_empty() {
                    type_groups.push((current_group.trim().to_string(), has_negation));
                    current_group.clear();
                    has_negation = false;
                }
            }
            c if c.is_whitespace() && paren_depth == 0 => {
                // Skip whitespace between groups
                continue;
            }
            c => {
                current_group.push(c);
            }
        }
    }

    // Handle any remaining content
    if !current_group.trim().is_empty() {
        type_groups.push((current_group.trim().to_string(), has_negation));
    }

    // Parse each type group
    let parsed_groups: Vec<_> = type_groups
        .iter()
        .filter_map(|(group, negated)| {
            parse_single_type_group(group).map(|pattern| (pattern, *negated))
        })
        .collect();

    if parsed_groups.is_empty() {
        return vec![(
            quote! { crate::candle_indicators::types::CandleTypePattern::Any },
            false,
        )];
    }

    // Return all parsed groups
    // Multiple groups will be chained as separate with_candle_type() calls
    parsed_groups
}

/// Generate a PatternMask builder chain for a single bar
fn generate_pattern_mask(bar: &BarRequirement) -> proc_macro2::TokenStream {
    let mut builder = quote! { crate::candle_indicators::registry::PatternMask::wildcard() };

    if let Some(ref trend) = bar.trend {
        let trend_value = match trend.as_str() {
            "UP" => true,
            "DOWN" => false,
            _ => panic!("Invalid trend value: {}. Expected 'UP' or 'DOWN'", trend),
        };
        builder = quote! { #builder.with_trend(#trend_value) };
    }

    if let Some(ref colour) = bar.colour {
        let colour_value = colour == "GREEN";
        builder = quote! { #builder.with_colour(#colour_value) };
    }

    if let Some(ref fill) = bar.fill {
        let fill_value = fill == "HALLOW";
        builder = quote! { #builder.with_fill(#fill_value) };
    }

    if let Some(ref candle_type) = bar.candle_type {
        let candle_type_patterns = parse_candle_type(candle_type);

        // Check if we have multiple patterns and they are all negated
        let all_negated = candle_type_patterns.iter().all(|(_, is_neg)| *is_neg);

        if candle_type_patterns.len() > 1 && all_negated {
            // Multiple negations - use special method for proper AND logic
            let patterns: Vec<_> = candle_type_patterns.iter().map(|(p, _)| p).collect();
            builder = quote! {
                #builder.with_multiple_negated_candle_types(&[#(#patterns),*])
            };
        } else {
            // Single pattern or mixed/positive patterns - use existing methods
            for (pattern, is_negated) in candle_type_patterns {
                if is_negated {
                    builder = quote! { #builder.with_negated_candle_type(#pattern) };
                } else {
                    builder = quote! { #builder.with_candle_type(#pattern) };
                }
            }
        }
    }

    if let Some(ref body_height) = bar.body_height {
        let is_long = match body_height.as_str() {
            "LONG" => true,
            "SHORT" => false,
            _ => panic!(
                "Invalid body_height value: {}. Expected 'LONG' or 'SHORT'",
                body_height
            ),
        };
        builder = quote! { #builder.with_body_height(#is_long) };
    }

    if let Some(ref line_height) = bar.line_height {
        let is_long = match line_height.as_str() {
            "LONG" => true,
            "SHORT" => false,
            _ => panic!(
                "Invalid line_height value: {}. Expected 'LONG' or 'SHORT'",
                line_height
            ),
        };
        builder = quote! { #builder.with_line_height(#is_long) };
    }

    if let Some(ref body_gap) = bar.body_gap {
        // Note: GAP_UP constant = true, GAP_DOWN constant = false (consistent with other directions)
        // But with_body_gap parameter is 'gap_down', so GAP_UP should map to gap_down=false
        let gap_down = match body_gap.as_str() {
            "GAP_UP" => false,  // GAP_UP means gap_down=false
            "GAP_DOWN" => true, // GAP_DOWN means gap_down=true
            _ => panic!(
                "Invalid body_gap value: {}. Expected 'GAP_UP' or 'GAP_DOWN'",
                body_gap
            ),
        };
        builder = quote! { #builder.with_body_gap(#gap_down) };
    }

    if let Some(ref wick_gap) = bar.wick_gap {
        // Note: GAP_UP constant = true, GAP_DOWN constant = false (consistent with other directions)
        // But with_wick_gap parameter is 'gap_down', so GAP_UP should map to gap_down=false
        let gap_down = match wick_gap.as_str() {
            "GAP_UP" => false,  // GAP_UP means gap_down=false
            "GAP_DOWN" => true, // GAP_DOWN means gap_down=true
            _ => panic!(
                "Invalid wick_gap value: {}. Expected 'GAP_UP' or 'GAP_DOWN'",
                wick_gap
            ),
        };
        builder = quote! { #builder.with_wick_gap(#gap_down) };
    }

    builder
}

/// Pattern template attribute macro
///
/// Generates const pattern definitions in the module where it's used.
/// The build script will later collect all these definitions.
///
/// # Example
///
/// ```rust,ignore
/// #[pattern_template(
///     name = "AdvanceBlock",
///     forecast = "BearishReversal",
///     bar(colour = "GREEN", fill = "SOLID", candle_type = "Basic(LongWhiteCandle)"),
///     bar(colour = "GREEN", fill = "SOLID"),
///     bar(colour = "GREEN", fill = "SOLID")
/// )]
/// pub fn calc(...) -> bool {
///     // pattern implementation
/// }
/// ```
///
/// This generates:
/// - `pub const PATTERN_MASKS_ADVANCEBLOCK: [PatternMask; 3]`
/// - `pub const PATTERN_DEF_ADVANCEBLOCK: PatternDefinition<3>`
/// - Metadata comment for build script to parse
#[proc_macro_attribute]
pub fn pattern_template(attr: TokenStream, item: TokenStream) -> TokenStream {
    let template = parse_macro_input!(attr as PatternTemplate);
    let input_fn = parse_macro_input!(item as syn::ItemFn);

    let pattern_name = &template.name;
    #[allow(unused_variables)] // Used in quote! macro string interpolation
    let forecast_name = &template.forecast;

    // Generate PatternMask for prev_bar if present, or wildcard if not
    #[allow(unused_variables)] // Used in quote! macro string interpolation
    let has_prev_bar = template.prev_bar.is_some();
    let mut all_bar_masks = Vec::new();

    // Always include prev_bar slot (wildcard if not specified)
    if let Some(ref prev_bar) = template.prev_bar {
        all_bar_masks.push(generate_pattern_mask(prev_bar));
    } else {
        // Use wildcard for prev_bar when not specified
        all_bar_masks.push(quote! { crate::candle_indicators::registry::PatternMask::wildcard() });
    }

    // Generate PatternMask for each pattern bar
    for bar in &template.bars {
        all_bar_masks.push(generate_pattern_mask(bar));
    }

    // Total masks is always pattern_bar_count + 1 (for prev_bar slot)
    #[allow(unused_variables)] // Used in quote! macro string interpolation
    let pattern_bar_count = template.bars.len();
    let total_masks = all_bar_masks.len();

    // Generate unique const names
    let masks_name = format_ident!("PATTERN_MASKS_{}", pattern_name.to_uppercase());

    // Detect which lazy bits this pattern uses
    let uses_body_height = template.bars.iter().any(|b| b.body_height.is_some())
        || template
            .prev_bar
            .as_ref()
            .is_some_and(|b| b.body_height.is_some());
    let uses_body_gap = template.bars.iter().any(|b| b.body_gap.is_some())
        || template
            .prev_bar
            .as_ref()
            .is_some_and(|b| b.body_gap.is_some());
    let uses_wick_gap = template.bars.iter().any(|b| b.wick_gap.is_some())
        || template
            .prev_bar
            .as_ref()
            .is_some_and(|b| b.wick_gap.is_some());

    // Generate lazy bit mask constant name
    let lazy_bits_name = format_ident!("PATTERN_LAZY_BITS_{}", pattern_name.to_uppercase());

    // Calculate lazy bit mask
    let mut lazy_mask_value = 0u64;
    if uses_body_height {
        lazy_mask_value |= 1 << 32; // BODY_HEIGHT_BIT
    }
    if uses_body_gap {
        lazy_mask_value |= (1 << 34) | (1 << 35); // BODY_GAP bits
    }
    if uses_wick_gap {
        lazy_mask_value |= (1 << 36) | (1 << 37); // WICK_GAP bits
    }

    let expanded = quote! {
        // Keep the original function
        #input_fn

        // Metadata comment for build script - DO NOT REMOVE
        // PATTERN_REGISTRY_ENTRY: name=#pattern_name, forecast=#forecast_name, bars=#pattern_bar_count, has_prev_bar=#has_prev_bar, lazy_bits_mask=#lazy_bits_name

        /// Generated pattern masks for #pattern_name
        pub const #masks_name: [crate::candle_indicators::registry::PatternMask; #total_masks] = [
            #(#all_bar_masks),*
        ];

        /// Lazy bits required by #pattern_name
        pub const #lazy_bits_name: u64 = #lazy_mask_value;

        // Note: PATTERN_DEF constant will be generated by build script
        // The build script scans for PATTERN_REGISTRY_ENTRY comments and generates
        // the complete registry including pattern definitions
    };

    TokenStream::from(expanded)
}
