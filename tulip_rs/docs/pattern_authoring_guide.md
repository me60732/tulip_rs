# Pattern Authoring Guide

This guide explains how to add a new candlestick pattern to the registry.

---

## Quick Attribute Reference

### `prev_bar` — eager (non-lazy) attributes only

| Attribute | Values | Lazy? |
|-----------|--------|-------|
| `trend` | `"UP"` / `"DOWN"` | no |
| `colour` | `"GREEN"` / `"RED"` | no |
| `fill` | `"HALLOW"` / `"FILL"` | no |
| `line_height` | `"LONG"` / `"SHORT"` | no |
| `candle_type` | *see [Candle Type Syntax](#candle-type-syntax)* | no |
| `lower_wick_lt_body` | `"TRUE"` / `"FALSE"` | no |
| `upper_wick_lt_body` | `"TRUE"` / `"FALSE"` | no |

### `bar` — all attributes

| Attribute | Values | Lazy? | Notes |
|-----------|--------|-------|-------|
| `colour` | `"GREEN"` / `"RED"` | no | |
| `fill` | `"HALLOW"` / `"FILL"` | no | |
| `line_height` | `"LONG"` / `"SHORT"` | no | Total range vs EMA |
| `candle_type` | *see [Candle Type Syntax](#candle-type-syntax)* | no | |
| `lower_wick_lt_body` | `"TRUE"` / `"FALSE"` | no | Lower wick < body |
| `upper_wick_lt_body` | `"TRUE"` / `"FALSE"` | no | Upper wick < body |
| `body_height` | `"LONG"` / `"SHORT"` | yes | Body size vs EMA |
| `body_gap` | `"GAP_UP"` / `"GAP_DOWN"` | yes | Body clear of prev body |
| `wick_gap` | `"GAP_UP"` / `"GAP_DOWN"` | yes | Full range clear of prev range |
| `open_in_prev_body` | `"TRUE"` / `"FALSE"` | yes | Open within prev body |
| `open_above_prev_mid` | `"TRUE"` / `"FALSE"` | yes | Open above prev body midpoint |
| `close_in_prev_body` | `"TRUE"` / `"FALSE"` | yes | Close within prev body |
| `close_above_prev_mid` | `"TRUE"` / `"FALSE"` | yes | Close above prev body midpoint |
| `high_in_prev_body` | `"TRUE"` / `"FALSE"` | yes | High within prev body |
| `high_in_prev_line` | `"TRUE"` / `"FALSE"` | yes | High within prev full range |
| `high_above_prev_mid` | `"TRUE"` / `"FALSE"` | yes | High above prev body midpoint |
| `low_in_prev_body` | `"TRUE"` / `"FALSE"` | yes | Low within prev body |
| `low_in_prev_line` | `"TRUE"` / `"FALSE"` | yes | Low within prev full range |
| `low_above_prev_mid` | `"TRUE"` / `"FALSE"` | yes | Low above prev body midpoint |
| `engulf_prev` | `"BODY"` / `"LINE"` | yes | I engulf prev body or full line |
| `inside_prev` | `"BODY"` / `"LINE"` | yes | I sit inside prev body or full line |
| `lower_wick_2x` | `"TRUE"` / `"FALSE"` | yes | Lower wick ≥ 2× body |
| `upper_wick_2x` | `"TRUE"` / `"FALSE"` | yes | Upper wick ≥ 2× body |
| `body_gt_prev_body` | `"TRUE"` / `"FALSE"` | yes | Body strictly larger than prev bar's body |

> **Lazy attributes** are computed automatically by the registry on demand.
> **Eager attributes** ("Lazy? = no") are pre-computed at bar-push time and
> always available at zero extra cost.
>
> **`"TRUE"` / `"FALSE"` / omit:** specifying `"TRUE"` requires the condition to
> hold; `"FALSE"` requires it not to hold; omitting the attribute means don't care.

---

The build script scans all pattern modules automatically — no manual wiring
is required beyond creating the file.

---

## Table of Contents

1. [File Location](#file-location)
2. [Module Structure](#module-structure)
3. [The `info()` Function](#the-info-function)
4. [The `#[pattern_template]` Macro](#the-pattern_template-macro)
   - [Top-level attributes](#top-level-attributes)
   - [`prev_bar` block](#prev_bar-block)
   - [`bar` blocks](#bar-blocks)
   - [Attribute value semantics](#attribute-value-semantics)
5. [Bar Attribute Reference](#bar-attribute-reference)
   - [Colour, fill, trend](#colour-fill-trend)
   - [Candle type](#candle-type)
   - [Height attributes](#height-attributes)
   - [Gap attributes](#gap-attributes)
   - [Position attributes (new)](#position-attributes-new)
   - [Engulf attributes (new)](#engulf-attributes-new)
   - [Wick ratio attributes (new)](#wick-ratio-attributes-new)
6. [Candle Type Syntax](#candle-type-syntax)
7. [The `calc()` Function](#the-calc-function)
8. [Bar Index Constants](#bar-index-constants)
9. [Forecast Types](#forecast-types)
10. [Complete Examples](#complete-examples)
11. [Decision Guide: Template vs `calc()`](#decision-guide-template-vs-calc)

---

## File Location

Place the pattern file in the directory that matches its bar count:

```
candle_patterns/one_bar/mypattern.rs      ← 1-bar patterns
candle_patterns/two_bar/mypattern.rs      ← 2-bar patterns
candle_patterns/three_bar/mypattern.rs    ← 3-bar patterns
candle_patterns/four_bar/mypattern.rs     ← 4-bar patterns
candle_patterns/five_bar/mypattern.rs     ← 5-bar patterns
```

Then add `pub mod mypattern;` to the corresponding `mod.rs`. The build script
does everything else.

---

## Module Structure

Every pattern file must contain exactly two public items:

```rust
// 1. Pattern metadata (with optional #[pattern_template] — see below)
pub fn info() -> CandleInfo { ... }

// 2. Template macro + calc function
#[pattern_template( ... )]
pub fn calc(inputs: (&[f64], &[f64], &[f64], &[f64]), state: &EmaState, bars: &[CandleBits]) -> bool {
    ...
}
```

**If all constraints are expressed in the template mask and `calc` would just
return `true`**, place `#[pattern_template]` on `info()` instead and omit
`calc` entirely — the macro will generate an `#[inline(always)]` default:

```rust
#[pattern_template( ... )]
pub fn info() -> CandleInfo { ... }
// calc() is generated automatically: #[inline(always)] pub fn calc(...) -> bool { true }
```

`compute_bits()` no longer exists. Lazy bits are computed centrally by the
registry via `ensure_lazy_bits()` before the lazy mask check — no per-pattern
compute function is needed or called.

---

## The `info()` Function

Returns static metadata about the pattern.

```rust
pub fn info() -> CandleInfo {
    CandleInfo {
        name: "mypattern",            // lowercase, no spaces — used as the key
        full_name: "My Pattern",      // human-readable display name
        forecast: ForecastType::BullishReversal,
        extended_pattern: None,       // or Some(CandlePattern::XYZ) if this pattern
                                      // is the first step of a larger one
        bars: 2,                      // must match the number of bar() blocks
        japanese_name: "",            // traditional name, empty string if unknown
    }
}
```

### Forecast types

| Value | Meaning |
|-------|---------|
| `ForecastType::BullishReversal` | Trend reverses upward |
| `ForecastType::BearishReversal` | Trend reverses downward |
| `ForecastType::BullishContinuation` | Uptrend continues |
| `ForecastType::BearishContinuation` | Downtrend continues |
| `ForecastType::BullishReversalOrContinuation` | Bullish signal, context-dependent |
| `ForecastType::BearishReversalOrContinuation` | Bearish signal, context-dependent |

---

## The `#[pattern_template]` Macro

The macro annotates either the `calc` function or the `info()` function and
generates the pattern's `PatternMask` array and lazy-bits constant. The
registry uses these masks for fast bitwise pre-filtering before calling `calc`.

- Annotate **`calc`** when the pattern needs a custom business-rule check.
  The macro automatically adds `#[inline(always)]` to `calc`.
- Annotate **`info`** when all constraints fit in the template mask. The
  macro generates a default `#[inline(always)] pub fn calc(...) -> bool { true }`.

### Top-level attributes

```rust
#[pattern_template(
    name = "MyPattern",               // PascalCase identifier for the generated enum variant
    forecast = "BullishReversal",     // matches a ForecastType variant (without the prefix)
    prev_bar( ... ),                  // trend of the bar before the pattern starts
    bar( ... ),                       // first bar of the pattern (oldest)
    bar( ... ),                       // second bar
    bar( ... ),                       // etc. — one bar() per bar in the pattern
)]
```

Bar blocks are ordered **oldest to newest**: `bar[0]` is the earliest bar,
`bar[N-1]` is the current (most recent) bar.

### `prev_bar` block

Specifies constraints on the bar that precedes the pattern. Only **mandatory**
(compulsory) attributes are valid here — lazy attributes need OHLC data from the
bar before `prev_bar`, which lies outside the sliding window and can never be
populated in `compute_bits()`. The proc macro enforces this at compile time.

| Attribute | Values | Description |
|-----------|--------|-------------|
| `trend` | `"UP"` / `"DOWN"` | Market trend entering the pattern |
| `colour` | `"GREEN"` / `"RED"` | Candle colour |
| `fill` | `"HALLOW"` / `"FILL"` | Body fill |
| `line_height` | `"LONG"` / `"SHORT"` | Total bar range vs EMA |
| `candle_type` | *(see Candle Type Syntax)* | Candle classification |
| `lower_wick_lt_body` | `"TRUE"` / `"FALSE"` | Lower wick < body height |
| `upper_wick_lt_body` | `"TRUE"` / `"FALSE"` | Upper wick < body height |

The following attributes are **not valid** on `prev_bar` and will produce a
compile error: `body_height`, `body_gap`, `wick_gap`, and all position,
engulf, and `_2x` wick attributes.

```rust
// Minimal — trend only (most common)
prev_bar(trend = "DOWN")

// With additional mandatory constraints
prev_bar(
    trend              = "UP",
    colour             = "GREEN",
    fill               = "HALLOW",
    line_height        = "LONG",
    candle_type        = "Basic(LongWhiteCandle | WhiteCandle)",
    upper_wick_lt_body = "TRUE",
)
```

### `bar` blocks

Each `bar(...)` block specifies constraints for one bar. All attributes are
optional — omitting an attribute means "don't care" for that property.

```rust
bar(
    colour      = "GREEN",
    fill        = "HALLOW",
    line_height = "LONG",
    candle_type = "Basic(LongWhiteCandle)",
)
```

### Attribute value semantics

Most attributes accept one of three states:

| Specified as | Meaning |
|---|---|
| `= "TRUE"` | This condition **must be true** |
| `= "FALSE"` | This condition **must be false** |
| *(omitted)* | **Don't care** — the bar matches regardless |

`colour`, `fill`, `candle_type`, `body_gap`, `wick_gap` use named values
(`"GREEN"`, `"GAP_UP"`, etc.) rather than `"TRUE"`/`"FALSE"`.

---

## Bar Attribute Reference

### Colour, fill, trend

| Attribute | Values | Description |
|-----------|--------|-------------|
| `colour` | `"GREEN"` / `"RED"` | Whether close > prev close (GREEN) or not (RED) |
| `fill` | `"HALLOW"` / `"FILL"` | Whether close > open (HALLOW) or close < open (FILL) |

> **Note:** `colour` and `fill` are independent. A red candle (close < prev
> close) can still be hollow (close > open) if the gap from the previous
> close is large.

---

### Candle type

```rust
candle_type = "Basic(LongWhiteCandle | WhiteCandle)"
```

See [Candle Type Syntax](#candle-type-syntax) for the full grammar.

---

### Height attributes

Both are independent checks against their respective EMA averages.

| Attribute | Values | Description | Bit type |
|-----------|--------|-------------|----------|
| `line_height` | `"LONG"` / `"SHORT"` | Total bar range (high−low) vs EMA of line ranges | mandatory |
| `body_height` | `"LONG"` / `"SHORT"` | Body size (open−close) vs EMA of body sizes | lazy |

```rust
bar(
    line_height = "LONG",   // total candle line is long relative to recent average
    body_height = "LONG",   // body is also long relative to recent average
)
```

---

### Gap attributes

Gaps are relative to the **immediately preceding bar** (the bar before this
`bar()` block in the template, or the `prev_bar` for the first `bar()`).

| Attribute | Values | Description | Bit type |
|-----------|--------|-------------|----------|
| `body_gap` | `"GAP_UP"` / `"GAP_DOWN"` | The body (open/close range) is entirely above or below the previous bar's body | lazy |
| `wick_gap` | `"GAP_UP"` / `"GAP_DOWN"` | The full candle line (high/low range) is entirely above or below the previous bar's full range — no overlap at all | lazy |

```rust
bar(
    body_gap = "GAP_DOWN",   // current body is entirely below previous body
    wick_gap = "GAP_UP",     // current bar's full range is entirely above prev bar
)
```

---

### Position attributes *(new)*

These encode where the current bar's prices fall relative to the **previous
bar's body and line**. All are lazy bits computed on first access.

The previous bar's body defines three reference levels:

```
prev HIGH ━━━━━━━━━━━━━━━  ← line top
              ↕ upper wick zone
prev body TOP ────────────  ← max(prev open, prev close)
              ↕ upper body half
prev body MID ────────────  ← midpoint
              ↕ lower body half
prev body BOT ────────────  ← min(prev open, prev close)
              ↕ lower wick zone
prev LOW  ━━━━━━━━━━━━━━━  ← line bottom
```

#### Open position

| Attribute | `"TRUE"` means | `"FALSE"` means |
|-----------|---------------|-----------------|
| `open_in_prev_body` | my open is within prev body (BOT ≤ open ≤ TOP) | my open is outside prev body |
| `open_above_prev_mid` | my open is above prev body midpoint | my open is at or below prev body midpoint |

#### Close position

| Attribute | `"TRUE"` means | `"FALSE"` means |
|-----------|---------------|-----------------|
| `close_in_prev_body` | my close is within prev body | my close is outside prev body |
| `close_above_prev_mid` | my close is above prev body midpoint | my close is at or below prev body midpoint |

> **Common composite:** `close_in_prev_body = "TRUE"` + `close_above_prev_mid = "FALSE"`
> means close is in the **lower half** of the prev body (e.g. Dark Cloud Cover,
> Thrusting). `close_above_prev_mid = "TRUE"` means the **upper half** (e.g. Piercing).

#### High position

| Attribute | `"TRUE"` means | `"FALSE"` means |
|-----------|---------------|-----------------|
| `high_in_prev_body` | my high is within prev body | my high is outside prev body |
| `high_in_prev_line` | my high is within prev bar's full range (LOW ≤ high ≤ HIGH) | my high is outside prev bar's range |
| `high_above_prev_mid` | my high is above prev body midpoint | my high is at or below prev body midpoint |

#### Low position

| Attribute | `"TRUE"` means | `"FALSE"` means |
|-----------|---------------|-----------------|
| `low_in_prev_body` | my low is within prev body | my low is outside prev body |
| `low_in_prev_line` | my low is within prev bar's full range | my low is outside prev bar's range |
| `low_above_prev_mid` | my low is above prev body midpoint | my low is at or below prev body midpoint |

#### Deriving gap checks from position bits

The position bits subsume `body_gap` and `wick_gap`. You can use the lower-level
bits directly for more precise control:

```
Body gap UP   ≡  open_in_prev_body="FALSE" + open_above_prev_mid="TRUE"
                 + close_in_prev_body="FALSE" + close_above_prev_mid="TRUE"

Body gap DOWN ≡  open_in_prev_body="FALSE" + open_above_prev_mid="FALSE"
                 + close_in_prev_body="FALSE" + close_above_prev_mid="FALSE"

Wick gap UP   ≡  low_in_prev_line="FALSE" + low_above_prev_mid="TRUE"
Wick gap DOWN ≡  high_in_prev_line="FALSE" + high_above_prev_mid="FALSE"
```

The `body_gap` and `wick_gap` shorthand attributes remain available and are
recommended when only direction matters.

---

### Engulf attributes *(new)*

These encode containment relationships between bars. All are lazy bits populated
by `apply_engulfing()` in `compute_bits()`.

#### Shorthand attributes — prefer these

Two shorthand attributes replace the verbose granular forms. Each accepts
`"BODY"` (bodies only) or `"LINE"` (full candle line including wicks).

| Attribute | Value | Meaning | Bits required |
|-----------|-------|---------|---------------|
| `engulf_prev` | `"BODY"` | My body strictly spans prev body (one side may be flush, not both) | `I_ENGULF_PREV_BODY` (11) |
| `engulf_prev` | `"LINE"` | My body spans prev bar's entire line, wicks included | `PREV_HIGH_IN_MY_BODY` (12) + `PREV_LOW_IN_MY_BODY` (13) |
| `inside_prev` | `"BODY"` | My body sits inside prev body | `OPEN_IN_PREV_BODY` (2) + `CLOSE_IN_PREV_BODY` (4) |
| `inside_prev` | `"LINE"` | My entire line sits inside prev line | `HIGH_IN_PREV_LINE` (7) + `LOW_IN_PREV_LINE` (10) |

```rust
// Bearish / bullish engulfing — current bar overtakes prev bar's body
bar(engulf_prev = "BODY")

// Strong engulf — current bar body also swallows prev bar's wicks
bar(engulf_prev = "LINE")

// Harami — current bar body sits inside prev bar's body
bar(inside_prev = "BODY")

// Inside bar — current bar's full range sits inside prev bar's full range
bar(inside_prev = "LINE")
```

#### Engulf body semantics

`engulf_prev = "BODY"` requires the current body to be **strictly wider** than
the previous body on at least one side.  One side may be flush:

- `cur_top > prev_top && cur_bot == prev_bot` ✅ valid engulf
- `cur_top == prev_top && cur_bot < prev_bot` ✅ valid engulf
- `cur_top == prev_top && cur_bot == prev_bot` ❌ same size — not an engulf

#### Registry-driven computation

Both `engulf_prev` and `inside_prev` are handled automatically by the registry.
When the template declares either attribute, the generated `PatternMask` sets
`has_engulf = true`, and the registry calls `apply_engulfing()` for that bar
before checking the lazy mask. No `compute_bits()` function is needed.

`apply_engulfing` sets all relative-position lazy bits (1–13) atomically, so
whatever subset your pattern mask checks will already be set by the time
`calc` is called.

---

### Wick ratio attributes *(new)*

These compare wick lengths to the body height.

#### `lower_wick_lt_body` / `upper_wick_lt_body` — mandatory bits

Computed at push time via `CandleShape`; always available at zero extra cost.

| Attribute | `"TRUE"` means | `"FALSE"` means |
|-----------|---------------|-----------------|
| `lower_wick_lt_body` | lower wick length **<** body height | lower wick length **≥** body height |
| `upper_wick_lt_body` | upper wick length **<** body height | upper wick length **≥** body height |

#### `lower_wick_2x` / `upper_wick_2x` — lazy bits

Computed on demand by the registry; available automatically when declared in the template.

| Attribute | `"TRUE"` means | `"FALSE"` means |
|-----------|---------------|-----------------|
| `lower_wick_2x` | lower wick **≥ 2×** body height | lower wick **< 2×** body height |
| `upper_wick_2x` | upper wick **≥ 2×** body height | upper wick **< 2×** body height |

```rust
bar(
    lower_wick_2x      = "TRUE",   // long lower shadow (hammer-like)
    upper_wick_lt_body = "TRUE",   // short upper shadow
)
```

> **Threshold note:** The 2× bits are pre-filters. If the pattern needs a
> finer threshold (e.g. ≥ 2.5×), declare `upper_wick_2x = "TRUE"` in the
> template for fast pre-filtering and add the exact check in `calc()`.

### `body_gt_prev_body` — lazy bit *(new)*

Compares the current bar's body height (`|close − open|`) to the **immediately
previous** bar's body height. Computed on demand; available automatically when
declared in the template.

| Attribute | `"TRUE"` means | `"FALSE"` means |
|-----------|---------------|-----------------|
| `body_gt_prev_body` | current body **strictly >** previous body | current body **≤** previous body (ties → `FALSE`) |

This is the canonical way to express *progressively shrinking* or *growing*
bodies across consecutive bars without writing manual `calc()` comparisons.

```rust
// Advance Block — each bar's body is smaller than the one before it
bar(colour = "GREEN", fill = "HALLOW", open_in_prev_body = "TRUE", body_gt_prev_body = "FALSE"),
bar(colour = "GREEN", fill = "HALLOW", open_in_prev_body = "TRUE", body_gt_prev_body = "FALSE"),
```

> **Limitation:** The bit compares only **adjacent** bars (current vs its
> immediate predecessor). To compare non-adjacent bars (e.g. bar 3 vs bar 1),
> you still need a manual check in `calc()`.

---

## Candle Type Syntax

The `candle_type` attribute accepts one or more space-separated type
expressions. Multiple expressions are combined with **OR** logic (the bar
matches if it satisfies any of them).

### Positive match

```rust
candle_type = "Basic(LongWhiteCandle | WhiteCandle)"
candle_type = "Doji(LongLeggedDoji | DragonflyDoji)"
candle_type = "Marubozu(WhiteMarubozu | OpeningWhiteMarubozu)"
candle_type = "SpinningTop(WhiteSpinningTop | BlackSpinningTop)"
```

Multiple categories (OR across categories):
```rust
candle_type = "Basic(BlackCandle | LongBlackCandle) Marubozu(OpeningBlackMarubozu | ClosingBlackMarubozu | BlackMarubozu)"
```

### Negated match

Prefix w

---

## The `calc()` Function

`calc` is the final guard called only after the template mask passes. Return
`true` to confirm the pattern, `false` to reject it.

```rust
pub fn calc(
    inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &EmaState,
    _bars: &[CandleBits],
) -> bool {
    let (open, high, _low, close) = inputs;
    // Example: open of second bar must gap above high of first bar
    open[SECOND] > high[FIRST]
}
```

**When to write `calc`:** only when the check cannot be expressed as a template
attribute — e.g. cross-bar comparisons like `open[SECOND] > high[FIRST]`, or
finer numeric thresholds.

**When to omit `calc`:** when all constraints fit in the template. Place
`#[pattern_template]` on `info()` instead and the macro generates:
```rust
#[inline(always)]
pub fn calc(
    _inputs: (&[f64], &[f64], &[f64], &[f64]),
    _state: &crate::candle_indicators::pattern_test::EmaState,
    _bars: &[crate::candle_indicators::registry::CandleBits],
) -> bool {
    true
}
```