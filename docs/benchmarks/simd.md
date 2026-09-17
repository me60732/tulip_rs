# SIMD Performance

`tulip_rs` uses 256-bit AVX2 `f64x4` SIMD (N = 4 lanes) in two modes:

- **[by_assets](simd/by-assets.md)** - process 4 different assets with the same options in a single pass
- **[by_options](simd/by-options.md)** - process 1 asset with 4 different option configurations in a single pass

All times are nanoseconds (ns). Lower is better.

---

## by_assets: 4 Assets Simultaneously

Full comparison tables (vs Rust, vs C / TA-Lib, Python Binding, Node Binding, C Binding, Go Binding, and each binding's reference libraries): **[SIMD by_assets →](simd/by-assets.md)**

| Binding | Indicators showing SIMD speedup | Median speedup (benefiting indicators) |
|---|---|---|
| Rust | 66 / 93 (71%) | ~1.83× |
| Python | 75 / 93 (81%) | ~1.89× |
| Node | 59 / 93 (63%) | ~1.29× |
| C | 64 / 91 (70%) | ~2.05× |
| Go | 62 / 94 (66%) | ~1.60× |

---

## by_options: 4 Option Sets Simultaneously

Full comparison tables (vs Rust, vs Sequential Rust / Binding scalar, C / TA-Lib, Python Binding, Node Binding, C Binding, Go Binding): **[SIMD by_options →](simd/by-options.md)**

| Binding | Indicators showing SIMD speedup | Median speedup (benefiting indicators) |
|---|---|---|
| Rust | 52 / 75 (69%) | ~2.02× |
| Python | 67 / 75 (89%) | ~2.00× |
| Node | 54 / 75 (72%) | ~1.37× |
| C | 51 / 75 (68%) | ~2.09× |
| Go | 53 / 76 (70%) | ~1.73× |
