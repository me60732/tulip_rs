# Go

**Repository:** [github.com/me60732/tulip_rs_go](https://github.com/me60732/tulip_rs_go)

The Go binding is built with cgo over the `tulip_rs_ffi` C ABI. It exposes every indicator, both SIMD modes, state management, and persistence to Go with zero-copy output views and idiomatic memory ownership.

### Installation

The Go repo ships a `bootstrap.sh` that installs the native library (`libtulip_rs_ffi`) in one shot. Run it once, then `go build ./...` just works.

**From source (recommended)** — clones the FFI repo as a sibling (if missing) and `cargo build`s it on your machine, picking up `-C target-cpu=native` from the FFI repo's `.cargo/config.toml`. LLVM then uses every instruction set your CPU supports; this is the path the benchmark suite measures. Requires Go 1.22+, Rust nightly (pinned by the FFI toolchain file), and a C toolchain for cgo:

```bash
git clone https://github.com/me60732/tulip_rs_go
cd tulip_rs_go
./bootstrap.sh --source    # optional REF arg pins the FFI tag/branch
go build ./...
```

**Prebuilt (no Rust required)** — downloads the GitHub-release cdylib for your GOOS/GOARCH from the [tulip_rs_ffi releases page](https://github.com/me60732/tulip_rs_ffi/releases) into `ffi/lib/`:

```bash
./bootstrap.sh --prebuilt
go build ./...
```

Prebuilds ship portable baselines only (`x86-64-v3` on x86, aarch64 `generic`) — CI CPUs are not yours, and a native-flags artifact would `SIGILL` at runtime on older machines. Windows gets the staticlib (cross-built via mingw-w64) so cgo links statically with no DLL to ship. The tarball also carries the generated headers, so `ffi/` always matches the linked lib.

> `go get`/`go install` are not supported entry points: module builds cannot run download hooks, so the cgo lib must already be present. Clone + bootstrap is the way.

**Link resolution order:** the cgo flags search `ffi/lib/` (prebuilt) first, then `../tulip_rs_ffi/target/release` and `target/debug` (source build), with matching rpaths. Binaries are not relocatable outside the checkout (dev-mode caveat). Everything under `ffi/` is generated and gitignored.

---

### Quick Examples

**SMA — single input, single output:**

```go
import (
    "github.com/me60732/tulip_rs_go/indicators"
    "github.com/me60732/tulip_rs_go/tulip"
)

close := []float64{81.59, 81.06, 82.87, 83.00, 83.61,
                  83.15, 82.84, 83.99, 84.55, 84.36}

options := []float64{5.0}
res, st, err := indicators.Sma.Indicator(close, options, nil)
if err != nil { ... }
defer res.Close()
defer st.Close()

smaValues := tulip.AsFloat64(res.Rows[0]) // zero-copy view → plain float64
```

**MACD — three outputs:**

```go
outputs, state, err := indicators.Macd.Indicator(close, []float64{12.0, 26.0, 9.0}, nil)
macdLine  := tulip.AsFloat64(outputs.Rows[0])
signal    := tulip.AsFloat64(outputs.Rows[1])
histogram := tulip.AsFloat64(outputs.Rows[2])
```

**ADX — multiple inputs (high, low, close) with optional outputs:**

```go
res, st, err := indicators.Adx.Indicator(high, low, close, []float64{14.0}, []bool{true, true, true})
defer res.Close()
defer st.Close()

adxValues := tulip.AsFloat64(res.Rows[0]) // primary output
dx        := tulip.AsFloat64(res.Rows[1]) // optional 0: dx
atr       := tulip.AsFloat64(res.Rows[2]) // optional 1: atr
tr        := tulip.AsFloat64(res.Rows[3]) // optional 2: tr
```

---

### State Object API

The `State` returned by every call to `Indicator()` (or `Batch()`) exposes the following API:

| Method | Signature | Description |
|---|---|---|
| `Batch` | `(inputs..., optionalOutputs []bool) -> (*tulip.Result, error)` | Continue computation on new bars; returns only new output values |
| `Serialize` | `(format tulip.Format) -> ([]byte, error)` | Serialise state to a TRFS blob (self-describing, cross-language) |
| `Clone` | `() -> (*State, error)` | Deep copy the state (no serde round-trip) |
| `Closed` | `() -> bool` | Reports whether Close has run |

**Persistence (Go has FULL format support):**

```go
// Bincode (compact, handles NaN/Inf — recommended)
blob, err := st.Serialize(tulip.FormatBincode)

// JSON (human-readable; FFI rejects non-finite f64s)
blob, err := st.Serialize(tulip.FormatJSON)

// Restore from blob (any tulip_rs_ffi binding can write/read the same format)
st2, err := indicators.Sma.DeserializeState(blob)
st3, err := st.Clone() // in-process snapshot

// Continue from restored state
br, err := st2.Batch(newClose, nil)
defer br.Close()
```

---

### Optional Outputs

Indicators that expose optional intermediate series accept a boolean mask as the last argument to `Indicator()` and `Batch()`:

```go
// ADX exposes optional outputs: dx, atr, tr
res, st, err := indicators.Adx.Indicator(high, low, close, []float64{14.0}, []bool{true, true, true})
defer res.Close()
defer st.Close()

// Request only the first optional output (dx)
partialRes, partialSt, err := indicators.Adx.Indicator(high[:50], low[:50], close[:50], []float64{14.0}, []bool{true, false, false})
defer partialRes.Close()
defer partialSt.Close()

// Pass the same mask to Batch
continued, err := partialSt.Batch(high[50:], low[50:], close[50:], []bool{true, false, false})
defer continued.Close()
```

Use `indicators.Adx.Info().OptionalOutputs` to discover which optional outputs an indicator has and in what order.

---

### SIMD

**By assets — same options applied to N assets:**

```go
// Each asset carries the same series (real) of equal length; all lanes share one options set
assets := [][indicators.SmaInputs][]float64{
    {real},
    {scale(real, 1.2)},
}
sim, err := indicators.Sma.SimdByAssets(assets, []float64{14.0}, nil)
defer sim.Close()

for i := range sim.Results {
    assetSMA := tulip.AsFloat64(sim.Results[i][0])
    // ...
}
```

**By options — N option sets applied to one shared series:**

```go
optionSets := [][]float64{{3}, {5}, {7}, {10}} // 4 period values
sim, err := indicators.Sma.SimdByOptions(real, optionSets, nil)
defer sim.Close()

for i, o := range optionSets {
    periodSMA := tulip.AsFloat64(sim.Results[i][0])
    // ...
}
```

---

### Indicator Info & Min Data

Each indicator exposes static metadata and utility functions:

```go
info := indicators.Sma.Info()
// {
//   Name:            "sma",
//   FullName:        "Simple Moving Average",
//   Type:            "Trend",
//   Inputs:          []string{"real"},
//   Options:         []string{"period"},
//   Outputs:         []string{"sma"},
//   OptionalOutputs: []string{},
//   DisplayGroups:   []tulip.DisplayGroup{...}
// }

minBars := indicators.Sma.MinData([]float64{5.0}) // minimum bars needed to produce output
```
