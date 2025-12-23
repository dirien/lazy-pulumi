//! Build script for injecting version information at compile time.
//!
//! When building with GoReleaser, the VERSION and GIT_COMMIT environment
//! variables are set and will be used. Otherwise, falls back to Cargo's
//! package version.

fn main() {
    // Rerun build script if these environment variables change
    println!("cargo:rerun-if-env-changed=VERSION");
    println!("cargo:rerun-if-env-changed=GIT_COMMIT");
    println!("cargo:rerun-if-env-changed=BUILD_DATE");

    // Inject VERSION from GoReleaser or fall back to CARGO_PKG_VERSION
    if let Ok(version) = std::env::var("VERSION") {
        println!("cargo:rustc-env=APP_VERSION={}", version);
    }

    // Inject GIT_COMMIT if available
    if let Ok(commit) = std::env::var("GIT_COMMIT") {
        println!("cargo:rustc-env=APP_COMMIT={}", commit);
    }

    // Inject BUILD_DATE if available
    if let Ok(date) = std::env::var("BUILD_DATE") {
        println!("cargo:rustc-env=APP_BUILD_DATE={}", date);
    }
}
