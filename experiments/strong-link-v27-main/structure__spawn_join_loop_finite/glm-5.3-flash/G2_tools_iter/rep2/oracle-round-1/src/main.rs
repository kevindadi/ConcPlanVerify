mod cir_trace;
fn main() { cir_trace::init();
    // R1: main starts a worker and waits for it to finish, then repeats the cycle.
    // R6: exactly one cycle, so the final count is 1.
    let mut done = 0;

    for _ in 0..1 {
        // R3: start the worker, keep its join handle, and wait for it
        // exactly once before starting the next worker.
        let handle = cir_trace::spawn("handle#343", || {
            // R2: the worker performs no shared work and shares no
            // mutexes or counters with the main task.
        });

        // R4/R5: the worker always finishes (it does nothing), so the
        // join below always completes and no schedule can stall.
        handle.join().expect("worker panicked");

        done += 1;
    }

    // R6: print exactly this line, then exit.
    println!("DONE done={}", done);
 cir_trace::finish();}
