# Python

**Repository:** [github.com/me60732/tulip_rs_python](https://github.com/me60732/tulip_rs_python)

The Python binding is built with [PyO3](https://pyo3.rs/) and packaged with [maturin](https://github.com/PyO3/maturin). It exposes every indicator, both SIMD modes, state management, and candlestick patterns to Python with a clean, idiomatic interface.

### Installation

**From source (recommended)** — compiling on your machine with `-C target-cpu=native` lets LLVM generate code for every instruction set your CPU supports — this speeds up the scalar indicators as much as the SIMD ones:

```bash
git clone https://github.com/me60732/tulip_rs_python
cd tulip_rs_python
git checkout {latest tag}   # or omit for the bleeding edge — see the repo's tags page
RUSTFLAGS="-C target-cpu=native" maturin develop --release
```

**Requirements:** Python 3.8+, Rust nightly (pinned by the repo's `rust-toolchain.toml`)

**From PyPI** — use this only when the deployment target architecture is unknown or a Rust toolchain can't run there (prebuilt wheels must ship generic x86-64/ARM baselines):

```bash
pip install tulip-rs
```

---

### Quick Examples

**SMA — single input, single output:**

```python
import numpy as np
import tulip_rs

close = np.array([81.59, 81.06, 82.87, 83.00, 83.61,
                  83.15, 82.84, 83.99, 84.55, 84.36], dtype=np.float64)

outputs, state = tulip_rs.indicators.sma.indicator([close], [5.0])
sma_values = outputs[0]
```

**MACD — three outputs via `get_all_outputs`:**

```python
outputs, state = tulip_rs.indicators.macd.indicator([close], [12.0, 26.0, 9.0])

macd_line = outputs[0]
signal    = outputs[1]
histogram = outputs[2]

# Or unpack as a list
all_out = outputs  # list of 3 numpy arrays
```

**ADX — multiple inputs (high, low, close):**

```python
high  = np.array([...], dtype=np.float64)
low   = np.array([...], dtype=np.float64)
close = np.array([...], dtype=np.float64)

outputs, state = tulip_rs.indicators.adx.indicator([high, low, close], [14.0])
adx_values = outputs[0]
```

**Candlestick pattern detection:**

```python
# Candlestick inputs are plain Python lists, NOT numpy arrays
open_  = [10.0, 10.5, 11.0, 10.8, 11.2]
high_  = [11.0, 11.2, 11.5, 11.1, 11.6]
low_   = [ 9.8, 10.2, 10.7, 10.5, 10.9]
close_ = [10.5, 11.0, 10.9, 11.0, 11.4]

result, state = tulip_rs.indicators.candlestick.candlestick(
    open_, high_, low_, close_,
    [1, 5, 3]  # [candle_period, trend_period, trend_signal_period]
)

for bar_patterns in result:
    for pattern in bar_patterns:
        print(pattern["name"], pattern["forecast"])
```

---

### `IndicatorState` Object API

The `state` returned by every call to `indicator()` (or `batch_indicator()`) exposes the following API:

| Method | Signature | Description |
|---|---|---|
| `batch_indicator` | `(inputs) -> list[np.ndarray]` | Continue computation on new bars; returns new output values only |
| `state_to_json` | `() -> Optional[str]` | Serialise state to a JSON string for persistence |
| `has_state` | property `bool` | Whether the state object holds valid indicator state |
| `num_outputs` | property `int` | Number of output series this indicator produces |

!!! note "Restoring state"
    To restore state from JSON, use the indicator-specific `restore_state(json_str)` function, e.g. `tulip_rs.indicators.sma.restore_state(json_str)`.

---

### Input Types

| Context | Input Type |
|---|---|
| All standard indicators | `list[np.ndarray]` where each array is `dtype=np.float64` |
| Candlestick patterns | Plain Python `list[float]` (one list per OHLC series) |
| SIMD by-assets | `list[list[np.ndarray]]` — outer list is assets, inner list is input series |
| SIMD by-options | `list[np.ndarray]` inputs + `list[list[float]]` options |

!!! warning "Always use float64"
    NumPy defaults to `float64` for most operations, but be explicit: `np.array(data, dtype=np.float64)`. Passing `float32` arrays will raise a `TypeError`.

---

### Introspection

Each indicator module exposes two utility functions:

```python
# Returns a dict describing the indicator
info = tulip_rs.indicators.sma.info()
# {
#   "name": "sma",
#   "full_name": "Simple Moving Average",
#   "indicator_type": "Trend",
#   "inputs": ["real"],
#   "options": ["period"],
#   "outputs": ["sma"],
#   "optional_outputs": [],
#   "display_groups": [
#     { "id": "sma", "label": "SMA", "display_type": "Overlay", "outputs": ["sma"] }
#   ]
# }

# Minimum number of input bars required for the given options
min_bars = tulip_rs.indicators.sma.min_data([5.0])  # returns 5
```

---
