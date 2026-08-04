//! Reference Rust for tests/e2e/dead_transition/buggy.json.
//! Expected defect: DeadTransition. `flag` is initialized to 0 and never
//! written, so the `flag > 0` branch is unreachable dead code in every run.

fn main() {
    let flag: i32 = 0;
    // s0: read flag, branch on flag > 0 — the true arm can never execute.
    #[allow(clippy::absurd_extreme_comparisons)]
    if flag > 0 {
        // s1: dead branch
        return;
    }
    // s2: the only reachable exit
}
