pub const HALLOW: bool = true;
pub const FILL: bool = false;
pub const GREEN: bool = true;
pub const RED: bool = false;
pub const LONG: bool = true;
pub const SHORT: bool = false;
pub const NO_TOP_WICK: i8 = 1;
pub const NO_BOTTOM_WICK: i8 = -1;
pub const NO_WICK: i8 = 0;
pub const HAS_WICK: i8 = 2;
pub const UP_TREND: bool = true;
pub const DOWN_TREND: bool = false;
pub const GAP_UP: bool = true;
pub const GAP_DOWN: bool = false;

//min_long_cdl_height,
pub const MIN_LONG_CDL_HEIGHT: f64 = 0.7; //70%
pub const TOLERANCE: f64 = 0.005; //0.5 %
pub const DOJI_MAX_HEIGHT: f64 = 0.03;

#[inline(always)]
pub(crate) fn cdl_total_range(real1: f64, real2: f64) -> f64 {
    (real1 - real2).abs()
}
#[inline(always)]
pub(crate) fn cdl_body_greater(body: (f64, f64), real: f64, multiplier: f64) -> bool {
    let body_range = cdl_total_range(body.0, body.1);
    if body_range > real * multiplier {
        return true;
    }
    false
}
#[inline(always)]
pub(crate) fn cdl_body_greater_body(body1: (f64, f64), body2: (f64, f64), multiplier: f64) -> bool {
    let body_range1 = cdl_total_range(body1.0, body1.1);
    let body_range2 = cdl_total_range(body2.0, body2.1);
    if body_range1 > body_range2 * multiplier {
        return true;
    }
    false
}
#[inline(always)]
pub(crate) fn cdl_body_range(open: f64, close: f64) -> (f64, f64) {
    if cdl_body_fill(open, close) == HALLOW {
        return (open, close);
    }

    (close, open)
}

#[inline(always)]
pub(crate) fn cdl_real_within_body(body: (f64, f64), real: f64) -> bool {
    let (open, close) = body;
    if cdl_body_fill(open, close) == HALLOW {
        real >= open && real <= close
    } else {
        real >= close && real <= open
    }
}
#[inline(always)]
pub(crate) fn cdl_height(body: (f64, f64), avg_range: f64) -> bool {
    let (open, close) = body;
    let body_range = cdl_total_range(open, close);
    let min_range = MIN_LONG_CDL_HEIGHT * avg_range;
    let tol_range = TOLERANCE * avg_range;
    if body_range >= min_range - tol_range {
        LONG
    } else {
        SHORT
    }
}
#[inline(always)]
pub fn cdl_gap<const BODY_GAP: bool>(
    prev_candle: (f64, f64),
    current_candle: (f64, f64),
) -> Option<bool> {
    let prev_top;
    let prev_bottom;
    let top;
    let bottom;

    if BODY_GAP {
        prev_top = prev_candle.0.max(prev_candle.1);
        prev_bottom = prev_candle.0.min(prev_candle.1);
        top = current_candle.0.max(current_candle.1);
        bottom = current_candle.0.min(current_candle.1);
    } else {
        prev_top = prev_candle.0;
        prev_bottom = prev_candle.1;
        top = current_candle.0;
        bottom = current_candle.1;
    }

    if bottom >= prev_top {
        return Some(GAP_UP);
    } else if top <= prev_bottom {
        return Some(GAP_DOWN);
    }

    None
}
#[inline(always)]
pub fn cdl_similar_height(body1: (f64, f64), body2: (f64, f64), tolerance: Option<f64>) -> bool {
    let tolerance = tolerance.unwrap_or(TOLERANCE);
    let (open1, close1) = body1;
    let (open2, close2) = body2;
    let height1 = (open1 - close1).abs();
    let height2 = (open2 - close2).abs();
    //let tolerance = tolerance / 100.0 * height1;
    let average = (height1 + height2) / 2.0;
    let diff = (height1 - height2).abs();
    diff <= (average * tolerance)
}
#[inline(always)]
pub(crate) fn cdl_wick_length(body: (f64, f64), real: f64, multiplier: Option<f64>) -> bool {
    let multiplier = multiplier.unwrap_or(1.0);
    let (l_body, h_body) = cdl_body_range(body.0, body.1);
    let body_range = cdl_total_range(l_body, h_body);

    if real < l_body {
        let wick_range = cdl_total_range(real, l_body);
        if wick_range >= body_range * multiplier {
            return LONG;
        }
    } else if real > h_body {
        let wick_range = cdl_total_range(real, h_body);
        if wick_range >= body_range * multiplier {
            return LONG;
        }
    }

    SHORT
}
#[inline(always)]
pub(crate) fn cdl_total_wick_length(open: f64, high: f64, low: f64, close: f64) -> f64 {
    let (l_body, h_body) = cdl_body_range(open, close);

    let top_wick = high - h_body;
    let bottom_wick = l_body - low;
    top_wick + bottom_wick
}
#[inline(always)]
pub(crate) fn cdl_body_fill(open: f64, close: f64) -> bool {
    if open < close {
        HALLOW
    } else {
        FILL
    }
}
#[inline(always)]
pub(crate) fn cdl_colour(prev_close: f64, close: f64) -> bool {
    if prev_close > close {
        return RED;
    }
    GREEN
}

//body position within the line
#[inline(always)]
pub fn cdl_body_position(open: f64, high: f64, low: f64, close: f64) -> Option<f64> {
    let range = high - low;
    if range == 0.0 {
        return None;
    }

    // Determine the top and bottom of the candle's body.
    let body_top = open.max(close);
    let body_bottom = open.min(close);

    // If the body has no upper wick, return 100%.
    if body_top == high {
        return Some(100.0);
    }

    // If the body has no lower wick, return 0%.
    if body_bottom == low {
        return Some(0.0);
    }

    // Otherwise, calculate the center of the body using the boundaries.
    let body_center = (body_top + body_bottom) / 2.0;
    Some(((body_center - low) / range) * 100.0)
}
#[inline(always)]
pub fn cdl_bar_engulf_bar(bar1: (f64, f64), bar2: (f64, f64)) -> bool {
    let prev_top = bar1.0.max(bar1.1);
    let prev_bottom = bar1.0.min(bar1.1);
    let top = bar2.0.max(bar2.1);
    let bottom = bar2.0.min(bar2.1);

    if prev_top == top && prev_bottom == bottom {
        return false;
    } else if prev_top >= top && prev_bottom <= bottom {
        return true;
    }
    false
}
#[inline(always)]
pub fn cdl_no_wick(open: f64, high: f64, low: f64, close: f64) -> i8 {
    // Determine the top and bottom of the candle's body.
    let body_top = open.max(close);
    let body_bottom = open.min(close);
    // Calculate the wick lengths.
    let top_wick_len = high - body_top;
    let bottom_wick_len = body_bottom - low;

    match (top_wick_len == 0.0, bottom_wick_len == 0.0) {
        (true, true) => NO_WICK,
        (true, false) => NO_TOP_WICK,
        (false, true) => NO_BOTTOM_WICK,
        (false, false) => HAS_WICK,
    }
}
/// Returns the position of `val` within (or relative to) the candle's body as a percentage.
/// The candle body is defined by open and close:
///   - 0% means the value equals the lower bound of the body (min(open, close))
///   - 100% means the value equals the upper bound of the body (max(open, close))
/// Values below the body yield a negative percentage and above yield a percentage above 100.
/// If the candle has no visible body (open == close):
///   - if val equals open, returns 50.0,
///   - otherwise computes a scaled percentage difference relative to the open.
#[inline(always)]
pub fn cdl_real_in_body_position(body: (f64, f64), real: f64) -> f64 {
    let (open, close) = body;
    let lower = open.min(close);
    let upper = open.max(close);
    let range = upper - lower;
    if range.abs() < std::f64::EPSILON {
        // Candle has no visible body.
        if (real - open).abs() < std::f64::EPSILON {
            50.0
        } else if real < open {
            // Compute a negative percentage difference relative to the open.
            -(((open - real) / open.abs().max(1e-10)) * 100.0)
        } else {
            // For a value above the point body.
            100.0 + (((real - open) / open.abs().max(1e-10)) * 100.0)
        }
    } else {
        // Compute the percentage position relative to the body range.
        ((real - lower) / range) * 100.0
    }
}

/*#[inline(always)]
pub(crate) fn cdl_down_bar(prev_body: (f64, f64), body: (f64, f64)) -> bool {
    let (open1, close1) = prev_body;
    let (open2, close2) = body;
    if cdl_body_fill(open1, close1) == FILL {
        if cdl_body_fill(open2, close2) == FILL {
            open2 < open1
        } else {
            close2 < open1
        }
    } else if cdl_body_fill(open2, close2) == FILL {
        open2 < close1
    } else {
        close2 < close1
    }
}*/
