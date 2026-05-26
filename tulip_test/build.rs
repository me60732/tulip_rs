/// Minimal .env loader — sets env vars from `.env` in the current directory.
/// Only sets a variable if it isn't already present in the environment,
/// so a real env var always wins over the file.
fn load_dotenv() {
    use std::io::BufRead;
    let Ok(file) = std::fs::File::open(".env") else {
        return;
    };
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        let line = line.trim().to_string();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val.trim().trim_matches('"').trim_matches('\'');
            if std::env::var(key).is_err() {
                // SAFETY: build scripts are single-threaded
                unsafe { std::env::set_var(key, val) };
            }
        }
    }
}

fn main() {
    // Pull TALIB_LIB_DIR (and other vars) from .env if not already set.
    load_dotenv();
    println!("cargo:rerun-if-changed=.env");

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

    // TA-Lib: only link when the `talib` feature is enabled.
    // Set TALIB_LIB_DIR in .env (or the environment) to point to your TA-Lib
    // installation. Defaults to /usr/local/lib if not set.
    if std::env::var("CARGO_FEATURE_TALIB").is_ok() {
        let lib_dir =
            std::env::var("TALIB_LIB_DIR").unwrap_or_else(|_| "/usr/local/lib".to_string());
        println!("cargo:rustc-link-search=native={}", lib_dir);
        println!("cargo:rustc-link-lib=ta-lib");
        println!("cargo:rerun-if-changed=src/talib_bindings.rs");
        println!("cargo:rerun-if-env-changed=TALIB_LIB_DIR");
    }
}
