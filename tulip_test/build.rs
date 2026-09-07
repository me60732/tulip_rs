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

/// Ensures the specified git submodule is checked out.
/// If the marker file (relative to CARGO_MANIFEST_DIR) is missing (submodule not yet
/// initialised), runs `git submodule update --init` automatically so that
/// `cargo build` / `cargo bench` just works on a fresh clone.
fn ensure_submodule(marker_relative_path: &str, submodule_dir_name: &str) {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let pkg_root = std::path::Path::new(&manifest);
    let marker = pkg_root.join(marker_relative_path);

    if marker.exists() {
        return; // Already initialised — nothing to do
    }

    println!(
        "cargo:warning={submodule_dir_name} not found — initialising git submodule automatically..."
    );

    // CARGO_MANIFEST_DIR is tulip_test/; its parent is the workspace/git root.
    let repo_root = pkg_root
        .parent()
        .expect("Cannot determine git root from CARGO_MANIFEST_DIR");

    // Compute the submodule path relative to the repo root dynamically
    // so this still works if the tulip_test directory is ever renamed.
    let submodule_path = pkg_root
        .strip_prefix(repo_root)
        .unwrap()
        .join(submodule_dir_name);

    let status = std::process::Command::new("git")
        .args([
            "submodule",
            "update",
            "--init",
            "--",
            submodule_path.to_str().unwrap(),
        ])
        .current_dir(repo_root)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning={submodule_dir_name} submodule initialised successfully");
        }
        Ok(_) => panic!(
            "\n\ngit submodule update --init failed.\
             \nPlease run manually: git submodule update --init --recursive\n"
        ),
        Err(e) => panic!(
            "\n\nFailed to run git: {e}\
             \nPlease run manually: git submodule update --init --recursive\n"
        ),
    }
}

fn main() {
    // Pull TALIB_LIB_DIR (and other vars) from .env if not already set.
    load_dotenv();
    println!("cargo:rerun-if-changed=.env");

    // Auto-init the tulip_indicators submodule on first build after a fresh clone.
    ensure_submodule("tulip_indicators/tiamalgamation.c", "tulip_indicators");

    // Rebuild if the upstream C sources change (e.g. after `git submodule update`).
    println!("cargo:rerun-if-changed=tulip_indicators/tiamalgamation.c");
    println!("cargo:rerun-if-changed=tulip_indicators/indicators.h");
    println!("cargo:rerun-if-changed=tulip_indicators/candles.h");

    // Build Tulip Indicators from the single-file amalgamation.
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
    if std::env::var("CARGO_FEATURE_TALIB").is_ok() {
        ensure_submodule("ta_lib_src/include/ta_libc.h", "ta_lib_src");

        println!("cargo:rerun-if-changed=ta_lib_src/src/ta_func");
        println!("cargo:rerun-if-changed=ta_lib_src/src/ta_common");
        println!("cargo:rerun-if-changed=src/talib_bindings.rs");

        let ta_func_dir = std::path::Path::new("ta_lib_src/src/ta_func");
        let mut ta_func_sources: Vec<std::path::PathBuf> = std::fs::read_dir(ta_func_dir)
            .expect("ta_lib_src/src/ta_func not found — did the submodule init correctly?")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "c"))
            .collect();
        ta_func_sources.sort();

        let mut build = cc::Build::new();
        build
            .include("ta_lib_src/include")
            .include("ta_lib_src/src/ta_common")
            .include("ta_lib_src/src/ta_func")
            .flag_if_supported("-O3")
            .flag_if_supported("-w")
            .file("ta_lib_src/src/ta_common/ta_global.c")
            .file("ta_lib_src/src/ta_common/ta_retcode.c")
            .file("ta_lib_src/src/ta_common/ta_version.c");

        for src in &ta_func_sources {
            build.file(src);
        }

        build.compile("ta_lib");
    }
}
