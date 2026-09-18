//! Reference Rust for tests/e2e/dead_transition/fixed.json.
//! Fix: the constant-condition branch is removed; the program reads the flag
//! and returns through a single live path.

fn main() {
    let flag: i32 = 0;
    let _ = flag; // s0: read flag
    // s2: return
}
