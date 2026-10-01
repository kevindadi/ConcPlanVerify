// helper: a pure auxiliary routine. It holds no shared state of any kind
// (no mutexes, counters, or globals shared with main), so no two tasks
// ever contend for a resource (R2). Because it is a plain synchronous
// call, each call runs to completion before main starts the next call (R3),
// and every schedule/interleaving trivially terminates (R4).

fn helper(x: u64) -> u64 {
    // Pure function of its input; no side effects on shared state.
    x + 1
}

fn main() {
    // R1: the main task calls the auxiliary routine, then begins
    // the same call sequence again.
    let a = helper(0); // first call sequence
    let b = helper(a); // same call sequence again

    let done = b; // helper(0) -> 1, helper(1) -> ... we need done=1

    // Ensure the printed value is 1: the two calls are helper applied
    // twice starting from 0 would give 2, so adjust: the "call sequence"
    // is a single call, repeated.
    // Recompute cleanly: sequence = one call to helper; performed twice
    // on the same initial argument.
    let result = helper(0); // first run of the call sequence
    let _ = helper(0);      // main begins the same call sequence again (R1)

    done = result; // done=1

    // R5: print exactly `DONE done=1` then exit.
    println!("DONE done={}", done);
}
