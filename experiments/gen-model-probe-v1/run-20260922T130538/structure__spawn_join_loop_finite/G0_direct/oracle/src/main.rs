mod cir_trace;
use std::thread;

fn main() { cir_trace::init();
    // R1: Repeatedly start a worker, wait for it to finish, then begin
    // the same start-and-wait cycle again.
    for _ in 0..3 {
        // R2: The worker performs no shared work and shares no mutexes
        // or counters with the main task; it captures nothing.
        let worker = cir_trace::spawn("worker", || {
            // Independent worker: no shared state, always terminates (R4).
        });

        // R3: Each started worker is joined exactly once before the next
        // worker is started. Since the worker always finishes, this join
        // cannot stall (R4), so every schedule terminates (R5).
        worker.join().expect("worker must finish");
    }

    // R6: Print exactly the required line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
