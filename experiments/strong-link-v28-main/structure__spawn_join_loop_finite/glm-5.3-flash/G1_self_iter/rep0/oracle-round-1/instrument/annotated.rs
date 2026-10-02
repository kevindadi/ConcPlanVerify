//! Start-and-wait cycle, repeated once, then report completion.
//!
//! Concurrency design (justification of repairs):
//! - R1: `main` performs the start-and-wait cycle, then performs the same
//!   cycle a second time before printing the final line.
//! - R2: `worker` is a plain function touching no statics, no mutexes, and
//!   no counters; it shares nothing with `main`.
//! - R3: Each `JoinHandle` is consumed by exactly one `join()` call. Rust's
//!   ownership makes a double-wait or a missed-wait a compile error, so
//!   every started worker is waited for exactly once before the next start.
//! - R4: `join()` returns as soon as the worker's closure returns; the
//!   worker cannot fail to finish (its body is trivially terminating), so
//!   no wait can stall.
//! - R5: Both tasks have finite, non-blocking bodies; the only
//!   synchronization is the join, which is guaranteed to complete. Every
//!   interleaving therefore terminates.
//! - R6: The program prints exactly one line, `DONE done=1`, then exits.

mod cir_trace;
fn worker() {
    // No shared work, no mutexes, no counters (R2). Body intentionally
    // empty so the task always finishes promptly (R4, R5).
}

/// One full cycle: start the worker, then wait for it exactly once (R1, R3).
fn run_cycle() {
    let handle = cir_trace::spawn("handle#1293", worker);
    // `join` consumes `handle`, so this wait can happen at most once,
    // and it happens before `run_cycle` returns, i.e. before the next
    // worker is started (R3). It cannot block forever (R4).
    handle.join().expect("worker task panicked");
}

fn main() { cir_trace::init();
    // First start-and-wait cycle (R1).
    run_cycle();
    // The same start-and-wait cycle begins again (R1).
    run_cycle();

    // Exactly the required output line, then the process exits (R6).
    println!("DONE done=1");
 cir_trace::finish();}
