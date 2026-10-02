// R1: main starts a worker task, waits for it to finish, then begins the
//     same start-and-wait cycle again (the cycle body is factored so it can
//     be repeated; here exactly one full cycle runs, matching done=1).
// R2: the worker performs no shared work and shares no mutexes or counters
//     with the main task.
// R3: each started worker is awaited exactly once before the next starts.
// R4: the worker always completes, so the await never stalls.
// R5: every schedule/interleaving terminates (single finite cycle).
// R6: prints exactly `DONE done=1` and exits.

use tokio::task::JoinHandle;

// Role: worker
async fn worker() {
    // No shared work, no mutexes, no counters.
}

// One full start-and-wait cycle: spawn the worker, then wait for it exactly once.
async fn start_and_wait_cycle() {
    // Start the worker task.
    let handle: JoinHandle<()> = tokio::spawn(worker());

    // Wait for that worker to finish (exactly once, before any next cycle).
    handle.await.expect("worker task must not panic");
}

#[tokio::main]
async fn main() {
    // R1: start-and-wait cycle; the cycle is repeatable, and one completed
    // cycle yields done=1 as required by R6.
    start_and_wait_cycle().await;

    // R6: print exactly this line, then exit.
    println!("DONE done=1");
}
