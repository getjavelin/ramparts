fn main() {
    // YARA-X is a pure Rust implementation and doesn't require system libraries
    // or build-time configuration unlike the original YARA which needed C bindings.
    // This build script is kept minimal for future build requirements.

    println!("cargo:rerun-if-changed=build.rs");
}
