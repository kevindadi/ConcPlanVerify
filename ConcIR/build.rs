// Anonymous source distribution: source hashes identify builds.
fn main() {
    println!("cargo:rustc-env=CONCIR_GIT_REV=anonymous-source-package");
    println!("cargo:rerun-if-changed=build.rs");
}
