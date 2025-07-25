use std::env;
use std::path::Path;

fn main() {
    // Only require YARA if the yara-scanning feature is enabled
    if cfg!(feature = "yara-scanning") {
        // Auto-detect YARA installation paths on different systems
        let yara_paths = [
            "/opt/homebrew", // macOS with Homebrew on Apple Silicon
            "/usr/local",    // macOS with Homebrew on Intel, Linux
            "/usr",          // System-wide Linux installation
        ];

        for base_path in &yara_paths {
            let include_path = format!("{base_path}/include");
            let lib_path = format!("{base_path}/lib");
            let yara_header = format!("{base_path}/include/yara.h");

            if Path::new(&yara_header).exists() {
                println!("cargo:rustc-link-search=native={lib_path}");
                println!("cargo:rustc-link-lib=dylib=yara");

                // Set environment variables for yara-sys build script
                env::set_var("YARA_LIBRARY_PATH", &lib_path);
                env::set_var("BINDGEN_EXTRA_CLANG_ARGS", format!("-I{include_path}"));

                println!("cargo:rerun-if-changed=build.rs");
                return;
            }
        }

        println!("cargo:warning=YARA not found in standard locations. Please install YARA or set YARA_LIBRARY_PATH manually.");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
