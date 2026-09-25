# Java

**Repository:** [github.com/me60732/tulip_rs_java](https://github.com/me60732/tulip_rs_java)

The Java binding uses the Panama FFM API (`java.lang.foreign`, final since JDK 22)
directly against the shared [`tulip_rs_ffi`](c.md) native library — no JNI, no
code generation, zero third-party dependencies. It exposes every indicator, both
SIMD modes, state streaming, serialization, and candlestick patterns.

### Installation

**From Maven Central** — the native library comes with the dependency: an
OS-activated profile in the published pom pulls the matching platform classifier
jar (linux-amd64/arm64, darwin-amd64/arm64, windows-amd64) containing the
portable-baseline binary, extracted automatically at first load.

```xml
<dependency>
  <groupId>io.github.me60732</groupId>
  <artifactId>tulip-rs-java</artifactId>
  <version>0.2.10</version>
</dependency>
```

Gradle: `implementation 'io.github.me60732:tulip-rs-java:0.2.10'`

**Requirements:** JDK 22+ (no `--enable-preview` needed; tested on Temurin 25).

**Build from source (CPU-tuned)** — mirroring the other bindings, a local Rust
toolchain + `-C target-cpu=native` beats the portable baseline:

```bash
git clone https://github.com/me60732/tulip_rs_java
cd tulip_rs_java
./bootstrap.sh --source   # clones ../tulip_rs_ffi and builds with native CPU tuning
./build.sh                # -> out/ (library) + out-examples/
```

---

### Quick Examples

Every indicator follows the same shape:

```java
import org.tuliprs.*;
import org.tuliprs.indicators.Sma;

double[] close = {81.59, 81.06, 82.87, 83.00, 83.61,
                  83.15, 82.84, 83.99, 84.55, 84.36};

// inputs: double[][] (one array per input series) | options: double[]
Outcome oc = Sma.indicator(new double[][] {close}, new double[] {5.0});
try (Result res = oc.result(); State st = oc.state()) {
    double[] sma = res.toDoubleArray(0);  // zero-copy view over Rust memory

    // Streaming continuation — feed new bars without reprocessing history
    Result br = st.batch(new double[][] {newBars});
    try (br) { ... }
}
```

### Memory model

Outputs and streaming state are Rust-allocated, so the JVM cannot reclaim them:
wrap `Result` / `State` / `SimdResult` in try-with-resources and every native
free runs automatically, in the contractual order. All `close()`s are
idempotent and a `java.lang.ref.Cleaner` backstop catches leaks — the end user
never writes a free.

### State persistence

```java
byte[] blob = st.serialize(Format.BINCODE);   // or Format.JSON
State restored = Sma.deserializeState(blob);  // blobs are self-describing
State snapshot = st.duplicate();              // Rust-Clone value semantics
```

### SIMD

```java
// N assets (2, 4, 8, 16) through one option set, in one CPU pass:
try (SimdResult sim = Sma.simdByAssets(new double[][][] {{a1}, {a2}, {a3}, {a4}},
        new double[] {5.0}, null)) {
    for (int i = 0; i < sim.numResults(); i++) {
        double[] assetI = sim.toDoubleArray(i, 0);  // lane i, output row 0
    }
}

// One asset through N option sets:
try (SimdResult sim = Sma.simdByOptions(new double[][] {close},
        new double[][] {{2}, {5}, {8}, {10}}, null)) { ... }
```

### Indicator info

```java
Info info = Sma.info();       // name, fullName, inputs, options, outputs,
                              // optionalOutputs, type, displayGroups
long need = Sma.minData(new double[] {5.0});
```

Constants `Sma.INPUTS`, `Sma.OPTIONS`, `Sma.ID` (the FFI's FNV-1a32 indicator
id used by serialization) are exposed on every facade.

### Candlestick

Pattern detection returns per-bar pattern ids/names instead of f64 rows, takes
an `int` forecast filter (`CandlePattern.FORECAST_BULLISH_REVERSAL` and
friends), and has no SIMD variants. See
`examples/org/tuliprs/examples/CandlestickExample.java` in the repository.
