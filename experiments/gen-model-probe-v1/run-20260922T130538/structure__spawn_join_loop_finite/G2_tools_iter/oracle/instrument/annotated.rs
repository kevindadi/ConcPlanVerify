mod cir_trace;
use std::thread;

fn main() { cir_trace::init();
    let mut done: u32 = 0;

    // Start a worker, wait for it to finish, then repeat the cycle.
    // The worker performs no shared work and shares nothing with main.
    while done < 1 {
        let worker = cir_trace::spawn("worker", || {
            // No shared state: this worker touches no mutexes or counters.
        });
        // Wait exactly once for this worker before the next cycle begins.
        worker.join().expect("worker must be able to finish");
        done += 1;
    }

    println!("DONE done={}", done);
 cir_trace::finish();}
