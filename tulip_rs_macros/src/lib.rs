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
    // Shorthand gap attributes (expand to position bits in generate_pattern_mask)
    body_gap: Option<String>,
    wick_gap: Option<String>,
    // Mandatory wick comparison bits (computed at push time via CandleShape)
    lower_wick_lt_body: Option<String>,
    upper_wick_lt_body: Option<String>,
    // Position bits (lazy, relative to previous bar)
    open_above_prev_mid: Option<String>,
    open_in_prev_body: Option<String>,
    close_above_prev_mid: Option<String>,
    close_in_prev_body: Option<String>,
    high_above_prev_mid: Option<String>,
    high_in_prev_body: Option<String>,
    high_in_prev_line: Option<String>,
    low_above_prev_mid: Option<String>,
    low_in_prev_body: Option<String>,
    low_in_prev_line: Option<String>,
    // Wick 2x bits (lazy)
    lower_wick_2x: Option<String>,
    upper_wick_2x: Option<String>,
    // Shorthand engulf attributes
    engulf_prev: Option<String>,
    inside_prev: Option<String>,
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
        let mut lower_wick_lt_body = None;
        let mut upper_wick_lt_body = None;
        let mut open_above_prev_mid = None;
        let mut open_in_prev_body = None;
        let mut close_above_prev_mid = None;
        let mut close_in_prev_body = None;
        let mut high_above_prev_mid = None;
        let mut high_in_prev_body = None;
        let mut high_in_prev_line = None;
        let mut low_above_prev_mid = None;
        let mut low_in_prev_body = None;
        let mut low_in_prev_line = None;
        let mut lower_wick_2x = None;
        let mut upper_wick_2x = None;
        let mut engulf_prev = None;
        let mut inside_prev = None;

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
                "lower_wick_lt_body" => lower_wick_lt_body = Some(value.value()),
                "upper_wick_lt_body" => upper_wick_lt_body = Some(value.value()),
                "open_above_prev_mid" => open_above_prev_mid = Some(value.value()),
                "open_in_prev_body" => open_in_prev_body = Some(value.value()),
                "close_above_prev_mid" => close_above_prev_mid = Some(value.value()),
                "close_in_prev_body" => close_in_prev_body = Some(value.value()),
                "high_above_prev_mid" => high_above_prev_mid = Some(value.value()),
                "high_in_prev_body" => high_in_prev_body = Some(value.value()),
                "high_in_prev_line" => high_in_prev_line = Some(value.value()),
                "low_above_prev_mid" => low_above_prev_mid = Some(value.value()),
                "low_in_prev_body" => low_in_prev_body = Some(value.value()),
                "low_in_prev_line" => low_in_prev_line = Some(value.value()),
                "lower_wick_2x" => lower_wick_2x = Some(value.value()),
                "upper_wick_2x" => upper_wick_2x = Some(value.value()),
                "engulf_prev" => engulf_prev = Some(value.value()),
                "inside_prev" => inside_prev = Some(value.value()),
                _ => {
                    return Err(syn::Error::new_spanned(
                        key,
                        "Unknown bar attribute. Valid attributes: trend, colour, fill, \
                         candle_type, body_height, line_height, body_gap, wick_gap, \
                         lower_wick_lt_body, upper_wick_lt_body, \
                         open_above_prev_mid, open_in_prev_body, \
                         close_above_prev_mid, close_in_prev_body, \
                         high_above_prev_mid, high_in_prev_body, high_in_prev_line, \
                         low_above_prev_mid, low_in_prev_body, low_in_prev_line, \
                         lower_wick_2x, upper_wick_2x, engulf_prev, inside_prev",
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
            lower_wick_lt_body,
            upper_wick_lt_body,
            open_above_prev_mid,
            open_in_prev_body,
            close_above_prev_mid,
            close_in_prev_body,
            high_above_prev_mid,
            high_in_prev_body,
            high_in_prev_line,
            low_above_prev_mid,
            low_in_prev_body,
            low_in_prev_line,
            lower_wick_2x,
            upper_wick_2x,
            engulf_prev,
            inside_prev,
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

        // Validate that prev_bar only uses mandatory (compulsory) attributes.
        // Lazy attributes require OHLC data from the bar before prev_bar, which
        // is outside the sliding window and can never be populated in compute_bits().
        // Mandatory bits (trend, colour, fill, line_height, candle_type,
        // lower_wick_lt_body, upper_wick_lt_body, body_height) are always valid on prev_bar.
        if let Some(ref pb) = prev_bar {
            if pb.body_gap.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'body_gap' — it is a lazy bit that requires \
                     data from outside the pattern window. Use only mandatory attributes on \
                     prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.wick_gap.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'wick_gap' — it is a lazy bit that requires \
                     data from outside the pattern window. Use only mandatory attributes on \
                     prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.open_above_prev_mid.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'open_above_prev_mid' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.open_in_prev_body.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'open_in_prev_body' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.close_above_prev_mid.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'close_above_prev_mid' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.close_in_prev_body.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'close_in_prev_body' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.high_above_prev_mid.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'high_above_prev_mid' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.high_in_prev_body.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'high_in_prev_body' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.high_in_prev_line.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'high_in_prev_line' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.low_above_prev_mid.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'low_above_prev_mid' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.low_in_prev_body.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'low_in_prev_body' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.low_in_prev_line.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'low_in_prev_line' — it is a lazy bit that \
                     requires data from outside the pattern window. Use only mandatory attributes \
                     on prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.lower_wick_2x.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'lower_wick_2x' — it is a lazy bit that requires \
                     data from outside the pattern window. Use only mandatory attributes on \
                     prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.upper_wick_2x.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'upper_wick_2x' — it is a lazy bit that requires \
                     data from outside the pattern window. Use only mandatory attributes on \
                     prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.engulf_prev.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'engulf_prev' — it is a lazy bit that requires \
                     data from outside the pattern window. Use only mandatory attributes on \
                     prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
            if pb.inside_prev.is_some() {
                return Err(input.error(
                    "prev_bar does not support 'inside_prev' — it is a lazy bit that requires \
                     data from outside the pattern window. Use only mandatory attributes on \
                     prev_bar: trend, colour, fill, line_height, candle_type, \
                     body_height, lower_wick_lt_body, upper_wick_lt_body",
                ));
            }
        }

        Ok(PatternTemplate {
            name,
            forecast,
            prev_bar,
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

    // Mandatory wick comparison bits (bits 25–26)
    if let Some(ref v) = bar.lower_wick_lt_body {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_lower_wick_lt_body(#is_true) };
    }
    if let Some(ref v) = bar.upper_wick_lt_body {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_upper_wick_lt_body(#is_true) };
    }

    // Gap shorthand attributes — PatternMask::with_body_gap / with_wick_gap expand
    // these to the appropriate position bits internally.
    if let Some(ref body_gap) = bar.body_gap {
        let gap: i8 = match body_gap.as_str() {
            "GAP_UP" => tulip_rs_shared::BODY_GAP_UP,
            "GAP_DOWN" => tulip_rs_shared::BODY_GAP_DOWN,
            _ => panic!(
                "Invalid body_gap value: {}. Expected 'GAP_UP' or 'GAP_DOWN'",
                body_gap
            ),
        };
        builder = quote! { #builder.with_body_gap(#gap) };
    }

    if let Some(ref wick_gap) = bar.wick_gap {
        let gap: i8 = match wick_gap.as_str() {
            "GAP_UP" => tulip_rs_shared::WICK_GAP_UP,
            "GAP_DOWN" => tulip_rs_shared::WICK_GAP_DOWN,
            _ => panic!(
                "Invalid wick_gap value: {}. Expected 'GAP_UP' or 'GAP_DOWN'",
                wick_gap
            ),
        };
        builder = quote! { #builder.with_wick_gap(#gap) };
    }

    // Position bits (lazy, relative to previous bar)
    if let Some(ref v) = bar.open_above_prev_mid {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_open_above_prev_mid(#is_true) };
    }
    if let Some(ref v) = bar.open_in_prev_body {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_open_in_prev_body(#is_true) };
    }
    if let Some(ref v) = bar.close_above_prev_mid {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_close_above_prev_mid(#is_true) };
    }
    if let Some(ref v) = bar.close_in_prev_body {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_close_in_prev_body(#is_true) };
    }
    if let Some(ref v) = bar.high_above_prev_mid {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_high_above_prev_mid(#is_true) };
    }
    if let Some(ref v) = bar.high_in_prev_body {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_high_in_prev_body(#is_true) };
    }
    if let Some(ref v) = bar.high_in_prev_line {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_high_in_prev_line(#is_true) };
    }
    if let Some(ref v) = bar.low_above_prev_mid {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_low_above_prev_mid(#is_true) };
    }
    if let Some(ref v) = bar.low_in_prev_body {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_low_in_prev_body(#is_true) };
    }
    if let Some(ref v) = bar.low_in_prev_line {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_low_in_prev_line(#is_true) };
    }

    // Shorthand engulf attributes
    if let Some(ref v) = bar.engulf_prev {
        let kind: i8 = match v.as_str() {
            "BODY" => tulip_rs_shared::ENGULF_BODY,
            "LINE" => tulip_rs_shared::ENGULF_LINE,
            _ => panic!(
                "Invalid engulf_prev value: '{}'. Expected 'BODY' or 'LINE'",
                v
            ),
        };
        builder = quote! { #builder.with_engulf_prev(#kind) };
    }
    if let Some(ref v) = bar.inside_prev {
        let kind: i8 = match v.as_str() {
            "BODY" => tulip_rs_shared::ENGULF_BODY,
            "LINE" => tulip_rs_shared::ENGULF_LINE,
            _ => panic!(
                "Invalid inside_prev value: '{}'. Expected 'BODY' or 'LINE'",
                v
            ),
        };
        builder = quote! { #builder.with_inside_prev(#kind) };
    }

    // Wick 2x bits (lazy)
    if let Some(ref v) = bar.lower_wick_2x {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_lower_wick_2x(#is_true) };
    }
    if let Some(ref v) = bar.upper_wick_2x {
        let is_true = v == "TRUE";
        builder = quote! { #builder.with_upper_wick_2x(#is_true) };
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

    // -----------------------------------------------------------------------
    // Lazy bits mask computation
    //
    // Determines which lazy bits must be computed before this pattern can be
    // evaluated.  We check every bar (including prev_bar) for each attribute
    // and OR in the corresponding bit(s).
    //
    // Mandatory bits (lower_wick_lt_body / upper_wick_lt_body) are NOT lazy —
    // they are computed eagerly at push time, so they do NOT contribute here.
    // -----------------------------------------------------------------------

    // Helper: does any bar (including prev_bar) have this attribute set?
    // Uses a fn pointer so iterator adapters can accept the predicate without wrapping.
    let any_bars_with = |pred: fn(&BarRequirement) -> bool| -> bool {
        template.bars.iter().any(pred) || template.prev_bar.as_ref().is_some_and(pred)
    };

    let mut lazy_mask_value: u16 = 0;

    // body_gap shorthand expands to: open_above_prev_mid (0), open_in_prev_body (1),
    //   close_above_prev_mid (2), close_in_prev_body (3)
    if any_bars_with(|b| b.body_gap.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::OPEN_ABOVE_PREV_BODY_MID_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::OPEN_IN_PREV_BODY_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::CLOSE_ABOVE_PREV_BODY_MID_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::CLOSE_IN_PREV_BODY_BIT;
    }

    // wick_gap shorthand expands to: high_above_prev_mid (5), high_in_prev_line (7),
    //   low_above_prev_mid (8), low_in_prev_line (10)
    if any_bars_with(|b| b.wick_gap.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::HIGH_ABOVE_PREV_BODY_MID_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::HIGH_IN_PREV_LINE_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::LOW_ABOVE_PREV_BODY_MID_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::LOW_IN_PREV_LINE_BIT;
    }

    // Individual position bits
    if any_bars_with(|b| b.open_above_prev_mid.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::OPEN_ABOVE_PREV_BODY_MID_BIT;
    }
    if any_bars_with(|b| b.open_in_prev_body.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::OPEN_IN_PREV_BODY_BIT;
    }
    if any_bars_with(|b| b.close_above_prev_mid.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::CLOSE_ABOVE_PREV_BODY_MID_BIT;
    }
    if any_bars_with(|b| b.close_in_prev_body.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::CLOSE_IN_PREV_BODY_BIT;
    }
    if any_bars_with(|b| b.high_above_prev_mid.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::HIGH_ABOVE_PREV_BODY_MID_BIT;
    }
    if any_bars_with(|b| b.high_in_prev_body.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::HIGH_IN_PREV_BODY_BIT;
    }
    if any_bars_with(|b| b.high_in_prev_line.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::HIGH_IN_PREV_LINE_BIT;
    }
    if any_bars_with(|b| b.low_above_prev_mid.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::LOW_ABOVE_PREV_BODY_MID_BIT;
    }
    if any_bars_with(|b| b.low_in_prev_body.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::LOW_IN_PREV_BODY_BIT;
    }
    if any_bars_with(|b| b.low_in_prev_line.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::LOW_IN_PREV_LINE_BIT;
    }

    // Wick 2x bits
    if any_bars_with(|b| b.lower_wick_2x.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::LOWER_WICK_LONG_2X_BIT;
    }
    if any_bars_with(|b| b.upper_wick_2x.is_some()) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::UPPER_WICK_LONG_2X_BIT;
    }

    // engulf_prev shorthand:
    //   BODY → I_ENGULF_PREV_BODY (bit 11)
    //   LINE → PREV_HIGH_IN_MY_BODY (bit 12) + PREV_LOW_IN_MY_BODY (bit 13)
    if any_bars_with(|b| b.engulf_prev.as_deref() == Some("BODY")) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::I_ENGULF_PREV_BODY_BIT;
    }
    if any_bars_with(|b| b.engulf_prev.as_deref() == Some("LINE")) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::PREV_HIGH_IN_MY_BODY_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::PREV_LOW_IN_MY_BODY_BIT;
    }

    // inside_prev shorthand:
    //   BODY → OPEN_IN_PREV_BODY (bit 2) + CLOSE_IN_PREV_BODY (bit 4)
    //   LINE → HIGH_IN_PREV_LINE (bit 7) + LOW_IN_PREV_LINE (bit 10)
    if any_bars_with(|b| b.inside_prev.as_deref() == Some("BODY")) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::OPEN_IN_PREV_BODY_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::CLOSE_IN_PREV_BODY_BIT;
    }
    if any_bars_with(|b| b.inside_prev.as_deref() == Some("LINE")) {
        lazy_mask_value |= 1u16 << tulip_rs_shared::HIGH_IN_PREV_LINE_BIT;
        lazy_mask_value |= 1u16 << tulip_rs_shared::LOW_IN_PREV_LINE_BIT;
    }

    // Generate lazy bit mask constant name
    let lazy_bits_name = format_ident!("PATTERN_LAZY_BITS_{}", pattern_name.to_uppercase());

    // Determine whether we need to emit a default calc.
    // If the macro is applied to a function OTHER than `calc` (e.g. `info`),
    // the module has no hand-written calc, so we generate one that returns true.
    let is_calc = input_fn.sig.ident == "calc";

    // When the annotated function IS calc, add #[inline(always)] automatically.
    let maybe_inline = if is_calc {
        quote! { #[inline(always)] }
    } else {
        quote! {}
    };

    // Generated default calc used when the module omits it entirely.
    let default_calc = if !is_calc {
        quote! {
            /// Generated default calc — all constraints are expressed in the pattern template mask.
            #[inline(always)]
            pub fn calc(
                _inputs: (&[f64], &[f64], &[f64], &[f64]),
                _state: &crate::candle_indicators::pattern_test::EmaState,
                _bars: &[crate::candle_indicators::registry::CandleBits],
            ) -> bool {
                true
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        // Keep the original function (with inline hint when it is calc)
        #maybe_inline
        #input_fn

        // Default calc (emitted only when macro is not applied to calc)
        #default_calc

        // Metadata comment for build script - DO NOT REMOVE
        // PATTERN_REGISTRY_ENTRY: name=#pattern_name, forecast=#forecast_name, bars=#pattern_bar_count, has_prev_bar=#has_prev_bar, lazy_bits_mask=#lazy_bits_name

        /// Generated pattern masks for #pattern_name
        pub const #masks_name: [crate::candle_indicators::registry::PatternMask; #total_masks] = [
            #(#all_bar_masks),*
        ];

        /// Lazy bits required by #pattern_name
        pub const #lazy_bits_name: u16 = #lazy_mask_value;

        // Note: PATTERN_DEF constant will be generated by build script
        // The build script scans for PATTERN_REGISTRY_ENTRY comments and generates
        // the complete registry including pattern definitions
    };

    TokenStream::from(expanded)
}
