fn main() {
    // Build Tulip indicators
    cc::Build::new()
        .include("tulip_indicators")
        .file("tulip_indicators/tiamalgamation.c")
        .flag("-O3")
        .flag("-march=native")
        .flag("-ffast-math")
        //.flag("-funroll-loops")
        //.flag("-fstrict-aliasing")
        .compile("tulip");

    // TA-Lib compilation - using system installation
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-lib=ta-lib");
    println!("cargo:rerun-if-changed=src/talib_bindings.rs");
}
