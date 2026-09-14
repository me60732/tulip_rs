# C — tulip_rs_ffi

A hand-rolled `#[no_mangle] extern "C"` layer over [`tulip_rs`](https://github.com/me60732/tulip_rs), exposing 95 indicator modules — 94 technical indicators plus 77+ candlestick patterns — via raw-pointer, Tulip-Indicators-style calling conventions.

No wrapper library or language runtime required — just `extern "C"` functions consumable from C, C++, Objective-C, or any language with a C ABI.

**Repository:** [github.com/me60732/tulip_rs_ffi](https://github.com/me60732/tulip_rs_ffi)

---

## Installation

No package manager — build the cdylib from source:

```bash
# Build the library (requires pinned nightly toolchain)
cargo build --release

# Compile against the header and link the library
cc -O2 -o example example.c \
    -I include \
    -L target/release -ltulip_rs_ffi -Wl,-rpath,target/release
```

**Requirements:**
- Rust **nightly** (pinned `nightly-2026-08-31` in `rust-toolchain.toml`)
- C compiler with support for linking shared libraries

---

## API Family

Every indicator exposes a 6-function family:

| Function | Description |
|---|---|
| `<ind>_info()` | Returns metadata (name, inputs, options, outputs, display groups) |
| `<ind>_min_data(options)` | Minimum input bars required for given options |
| `<ind>_indicator(inputs, data_len, options, optional_outputs, num_optional)` | Full calculation — creates state and returns first outputs |
| `<ind>_batch(state, inputs, data_len, optional_outputs, num_optional)` | Streaming continuation — uses existing state |
| `<ind>_state_free(state)` | Free the opaque indicator state (exactly once per state) |

**SIMD variants** where supported:

| Function | Description |
|---|---|
| `<ind>_simd_by_assets(inputs, num_assets, data_len, options, optional_outputs, num_optional)` | Compute one option set across N=2/4/8/16 assets simultaneously |
| `<ind>_simd_by_options(inputs, data_len, options, num_option_sets, optional_outputs, num_optional)` | Compute one asset across N=2/4/8/16 different option sets simultaneously |

**Option-less indicators** (ad, ao, bop, obv, ...) have `*_by_assets` only.

**State persistence & clone** (shared, not per-indicator):

| Function | Description |
|---|---|
| `tulip_state_serialize(indicator_id, format, state)` | Pack a live state into a self-describing `CBytes` blob (`C_STATE_FORMAT_BINCODE` or `C_STATE_FORMAT_JSON`); the state is read, not consumed |
| `tulip_state_deserialize(bytes, len)` | Rebuild a state handle from a blob — the header embeds the indicator name + format, so corrupt/mismatched blobs return `NULL`, never a mistyped handle |
| `tulip_state_clone(indicator_id, state)` | In-process deep copy without serde |
| `tulip_ffi_bytes_free(blob)` | Free a `CBytes` returned by `tulip_state_serialize` (exactly once) |

The `indicator_id` argument is a `C_INDICATOR_ID_<NAME>` constant generated into `include/tulip_rs_ffi_state_ids.h` (an FNV-1a hash of the indicator name), which `tulip_rs_ffi.h` includes automatically. Bincode handles all `f64` values including NaN/Inf and is recommended for persistence; JSON is human-readable but fails on non-finite values. See the [State Management](../state_management.md) page for round-trip examples.

---

## Memory Ownership

| Resource | Free via |
|---|---|
| Output buffers (`outputs[i]`) | `tulip_ffi_result_free()`, `tulip_ffi_batch_result_free()`, or `tulip_ffi_simd_result_free()` |
| Indicator state (`state`) | `<ind>_state_free(state)` — exactly once |
| SIMD states (`r.states[i]`) | `<ind>_state_free(r.states[i])` — one per result lane |
| Candlestick CSR buffers | `candlestick_result_free()` / `candlestick_batch_result_free()` |
| Serialized-state blobs (`CBytes`) | `tulip_ffi_bytes_free(blob)` — exactly once |
| Strings from `*_info()` | Leaked (process-lifetime, don't free) |

!!! note "State/results separation"
    `<ind>_indicator()` returns a `CIndicatorResult` containing both output buffers and an opaque state pointer. Calling `tulip_ffi_result_free(result)` frees **only** the outputs — the state remains valid for subsequent `<ind>_batch()` calls.

---

## Parameter Conventions

- **Raw pointers:** `const double *` or `const double *const *` arrays
- **Input counts:** `<NAME>_INPUTS` C #defines (from `tulip_rs_ffi_counts.h`)
- **Option counts:** `<NAME>_OPTIONS` C #defines (from `tulip_rs_ffi_counts.h`)
- **Optional outputs:** Pass `bool*` mask + count; pass `NULL, 0` for mandatory-only
- **Ordering:** Tulip-Indicators style — pointer immediately followed by its count

Example signature:

```c
CIndicatorResult ema_indicator(
    double const *const *inputs,   // [EMA_INPUTS] pointers
    size_t data_len,               // bars per series
    double const *options,         // [EMA_OPTIONS] flat array
    bool const *optional_outputs,  // [num_optional]
    size_t num_optional);          // count of optional outputs
```

---

## Quick Examples

### SMA — Basic Indicator (Single Input, Single Output)

```c
#include <stdio.h>
#include "../include/tulip_rs_ffi.h"

static const double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                               83.15, 82.84, 83.99, 84.55, 84.36};

int main(void) {
    const double options[SMA_OPTIONS] = {5.0};
    const double *inputs[SMA_INPUTS] = {close};

    CIndicatorResult r = sma_indicator(inputs, 10, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "error=%d\n", r.error);
        return 1;
    }

    printf("SMA(5): [");
    for (size_t i = 0; i < r.output_lens[0]; i++) {
        printf("%.4f", r.outputs[0][i]);
        if (i + 1 < r.output_lens[0]) printf(", ");
    }
    printf("]\n");

    tulip_ffi_result_free(r);
    sma_state_free(r.state);
    return 0;
}
```

### MACD — Three Outputs with Optional Outputs

```c
#include <stdio.h>
#include "../include/tulip_rs_ffi.h"

static const double close[] = {81.59, 81.06, 82.87, 83.00, 83.61,
                               83.15, 82.84, 83.99, 84.55, 84.36};

int main(void) {
    const double options[MACD_OPTIONS] = {12.0, 26.0, 9.0};
    const double *inputs[MACD_INPUTS] = {close};
    bool optional_outputs[2] = {true, true}; // short_ema, long_ema

    CIndicatorResult r = macd_indicator(inputs, 10, options, optional_outputs, 2);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "error=%d\n", r.error);
        return 1;
    }

    printf("macd_line:   [%.4f]\n", r.outputs[0][r.output_lens[0] - 1]);
    printf("signal_line: [%.4f]\n", r.outputs[1][r.output_lens[1] - 1]);
    printf("histogram:   [%.4f]\n", r.outputs[2][r.output_lens[2] - 1]);

    tulip_ffi_result_free(r);
    macd_state_free(r.state);
    return 0;
}
```

### ADX — Streaming with Batch Continuation

```c
#include <stdio.h>
#include "../include/tulip_rs_ffi.h"

static const double high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30};
static const double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30};
static const double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99};

int main(void) {
    const double options[ADX_OPTIONS] = {5.0};
    const double *inputs[ADX_INPUTS] = {high, low, close};

    // Initial partial calculation (first 6 bars)
    CIndicatorResult pr = adx_indicator(inputs, 6, options, NULL, 0);
    if (pr.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "error=%d\n", pr.error);
        return 1;
    }
    void *state = pr.state;
    tulip_ffi_result_free(pr); // outputs freed; state kept

    // Continue with remaining bars
    const double *rest_inputs[ADX_INPUTS] = {high + 6, low + 6, close + 6};
    CBatchResult br = adx_batch(state, rest_inputs, 2, NULL, 0);
    if (br.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "error=%d\n", br.error);
        return 1;
    }

    printf("ADX continued: [%.4f]\n", br.outputs[0][br.output_lens[0] - 1]);

    tulip_ffi_batch_result_free(br);
    adx_state_free(state);
    return 0;
}
```

### STOCH — Optional Outputs Mask

```c
#include <stdio.h>
#include "../include/tulip_rs_ffi.h"

static const double high[] = {82.15, 81.89, 83.03, 83.30, 83.85};
static const double low[]  = {81.29, 80.64, 81.31, 82.65, 83.07};
static const double close[] = {81.59, 81.06, 82.87, 83.00, 83.61};

int main(void) {
    const double options[STOCH_OPTIONS] = {5.0, 3.0, 3.0};
    const double *inputs[STOCH_INPUTS] = {high, low, close};

    // Request only first optional output (fastk)
    bool optional_outputs[2] = {true, false};

    CIndicatorResult r = stoch_indicator(inputs, 5, options, optional_outputs, 2);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "error=%d\n", r.error);
        return 1;
    }

    // Output order: mandatory slowk + requested optional outputs
    printf("slowk: [%.4f]\n", r.outputs[0][r.output_lens[0] - 1]);
    printf("fastk: [%.4f]\n", r.outputs[1][r.output_lens[1] - 1]);

    tulip_ffi_result_free(r);
    stoch_state_free(r.state);
    return 0;
}
```

### SIMD by Assets — N=4 Lanes

```c
#include <stdio.h>
#include "../include/tulip_rs_ffi.h"

static const double close[] = {81.59, 81.06, 82.87, 83.00, 83.61};

int main(void) {
    const double options[SMA_OPTIONS] = {5.0};

    // Asset 1: original
    const double *const asset1[SMA_INPUTS] = {close};

    // Asset 2: scaled +20%
    static double close_2[5];
    for (size_t i = 0; i < 5; i++) close_2[i] = close[i] * 1.2;
    const double *const asset2[SMA_INPUTS] = {close_2};

    // Asset 3: different trend
    static double close_3[5];
    for (size_t i = 0; i < 5; i++) close_3[i] = 90.0 + i * 0.5 + close[i] * 0.1;
    const double *const asset3[SMA_INPUTS] = {close_3};

    // Asset 4: downward
    static double close_4[5];
    for (size_t i = 0; i < 5; i++) close_4[i] = 100.0 - i * 0.3 + close[i] * 0.05;
    const double *const asset4[SMA_INPUTS] = {close_4};

    // simd_inputs indexed by lane (asset), NOT by input series
    const double *const *const simd_inputs[4] = {asset1, asset2, asset3, asset4};

    CSimdResult r = sma_simd_by_assets(simd_inputs, 4, 5, options, NULL, 0);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "error=%d\n", r.error);
        return 1;
    }

    for (size_t i = 0; i < r.num_results; i++) {
        printf("Asset %zu SMA: [%.4f]\n", i + 1, r.outputs[i][0][r.output_lens[i][0] - 1]);
        sma_state_free(r.states[i]); // free each state
    }

    tulip_ffi_simd_result_free(r);
    return 0;
}
```

### Candlestick — CSR Packed Output

```c
#include <stdint.h>
#include <stdio.h>
#include "../include/tulip_rs_ffi.h"

static const double open[] = {81.85, 81.20, 81.55, 82.91, 83.10, 83.41,
                              82.71, 82.70, 84.20, 84.25, 84.03, 85.45,
                              86.18, 88.00, 87.30, 87.50, 87.00, 86.50};
static const double high[] = {82.15, 81.89, 83.03, 83.30, 83.85, 83.90,
                              83.33, 84.30, 84.84, 85.00, 85.90, 86.58,
                              86.98, 88.00, 87.31, 87.55, 87.15, 86.60};
static const double low[] = {81.29, 80.64, 81.31, 82.65, 83.07, 83.11,
                             82.49, 82.30, 84.15, 84.11, 84.03, 85.39,
                             85.76, 87.17, 87.20, 86.10, 85.90, 85.20};
static const double close[] = {81.59, 81.06, 82.87, 83.00, 83.61, 83.15,
                               82.84, 83.99, 84.55, 84.36, 85.53, 86.54,
                               86.89, 87.77, 87.29, 86.50, 86.00, 85.50};

int main(void) {
    const double options[CANDLESTICK_OPTIONS] = {5.0, 2.0, 3.0};
    const double *inputs[CANDLESTICK_INPUTS] = {open, high, low, close};

    CCandleStickResult r = candlestick_indicator(inputs, 18, options, -1);
    if (r.error != C_INDICATOR_ERROR_OK) {
        fprintf(stderr, "error=%d\n", r.error);
        return 1;
    }

    printf("Total patterns: %zu\n", r.total_patterns);
    for (size_t i = 0; i < r.num_bars; i++) {
        uint32_t start = r.bar_offsets[i];
        uint32_t end = r.bar_offsets[i + 1];
        if (start == end) continue;
        printf("Bar %zu: ", i);
        for (uint32_t k = start; k < end; k++) {
            CCandlePatternInfo info = candlestick_pattern_info(r.pattern_ids[k]);
            printf("%s (%s) ", info.name, info.full_name);
        }
        printf("\n");
    }

    candlestick_result_free(r);
    candlestick_state_free(r.state);
    return 0;
}
```

---

## Error Handling

Every result struct starts with `CIndicatorError error`:

```c
enum CIndicatorError {
    C_INDICATOR_ERROR_OK,
    C_INDICATOR_ERROR_INVALID_INPUTS,
    C_INDICATOR_ERROR_NOT_ENOUGH_DATA,
    C_INDICATOR_ERROR_INVALID_OPTIONS,
    C_INDICATOR_ERROR_INVALID_INDICATOR_STATE
};
```

Always check `error == C_INDICATOR_ERROR_OK` before using outputs.

---

## Benchmarking

See the [`tulip_rs_ffi` repository bench harness](https://github.com/me60732/tulip_rs_ffi/tree/main/bench) for complete methodology — benchmarks against Tulip Indicators (C) and TA-Lib in the same process, logging to shared `indicator_benchmark` Postgres database.

---

## Reference

The authoritative API surface is defined in [`include/tulip_rs_ffi.h`](https://github.com/me60732/tulip_rs_ffi/blob/main/include/tulip_rs_ffi.h). All function signatures, struct layouts, and enum values are generated from the Rust source via `cbindgen`.
