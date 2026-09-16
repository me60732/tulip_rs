# Language Bindings

TulipRS is written in Rust and exposes its full API through language bindings. The core calling convention — inputs, options, outputs, and state — is identical at the Rust boundary regardless of the target language.

Bindings share the same Rust core calling convention; C FFI binds straight to the extern "C" surface. Each language binding layer only needs to marshal data into `Vec<f64>` slices, call the indicator, and unpack the output. SIMD, state management, and error handling are all handled by the core Rust library — the binding layer stays thin.

---

## Binding Comparison

| Target | Binding mechanism | State streaming | SIMD modes | Candlestick |
|---|---|---|---|---|
| **Rust** (native crate) | Direct dependency | `IndicatorState` returned with outputs, serialisable | by-assets (`indicator_by_assets::<N>`) + by-options (`indicator_by_options::<N>`) | CSR-packed pattern ids via `candlestick()` |
| **C** ([tulip_rs_ffi](https://github.com/me60732/tulip_rs_ffi)) | `extern "C"` shared library | `<ind>_batch()` on opaque state pointer | all indicators by-assets + with-options by-options | CSR-packed pattern ids |
| Python (`tulip_rs_python`) | PyO3/maturin wrapper | `indicator()`/`batch_indicator()` on state object | by-assets (nested lists) + by-options (option lists) | Plain Python lists for OHLC |
| Node.js (`tulip-rs-node`) | napi-rs native addon | `indicator()`/`batchIndicator()` on state object | `simdByAssets()` / `simdByOptions()` | Float64Array inputs |
| Go (`tulip_rs_go`) | cgo over C FFI | `Indicator()`/`Batch()` on state object | `SimdByAssets()` / `SimdByOptions()` | float64 slices |
| Browser WASM (`tulip-rs-wasm`) | WebAssembly via wasm-pack | `indicator()`/`batchIndicator()` on state object | same as Node.js | same as Node.js |

---

## Subpages

- [C FFI](language_bindings/c.md)
- [Python](language_bindings/python.md)
- [Node.js](language_bindings/node.md)
- [Go](language_bindings/go.md)
- [Browser (WASM)](language_bindings/wasm.md)

---

## Contributing a Binding

All language bindings share the same Rust calling convention:

1. Inputs arrive as `&[&[f64]]`, one slice per series.
2. Options arrive as `&[f64]`.
3. The function returns `Result<(Vec<Vec<f64>>, IndicatorState), IndicatorError>`.
4. `IndicatorState` implements `serde::Serialize` / `Deserialize` for cross-language state persistence.

A minimal binding only needs to marshal data into `Vec<f64>` slices, call the indicator, and unpack the output. SIMD, state management, and error handling are all handled by the core Rust library — the binding layer stays thin. If you'd like to contribute a binding for another language, open an issue on the [main repository](https://github.com/me60732/tulip_rs) to discuss the approach.

---
