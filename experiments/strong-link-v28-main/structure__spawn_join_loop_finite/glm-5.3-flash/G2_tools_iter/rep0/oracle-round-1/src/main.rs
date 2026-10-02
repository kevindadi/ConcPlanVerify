mod cir_trace;
// Cargo.toml:
//   [dependencies]
//   tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

use tokio::task::JoinHandle;

// R2: the worker performs no shared work and shares no mutexes or
// counters with the main task. It touches no shared state at all.
async fn worker() {
    // no shared work
}

#[tokio::main]
async fn main() { cir_trace::init();
    // R1: main starts a worker, waits for it to finish, then begins the
    // same start-and-wait cycle again.
    for _ in 0..2 {
        // R3: each worker is started and then awaited exactly once,
        // before the next worker is started.
        let handle: JoinHandle<()> = tokio::spawn(worker());

        // R4/R5: the worker body is empty and cannot fail or block, so
        // it always finishes and every await terminates — no schedule
        // or interleaving can stall.
        handle.await.expect("worker task failed");
    }

    // R6: print exactly one line, then exit.
    let done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
