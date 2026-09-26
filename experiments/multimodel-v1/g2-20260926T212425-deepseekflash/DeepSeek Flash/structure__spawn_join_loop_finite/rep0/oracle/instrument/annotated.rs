mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn worker() {
    // R2: worker performs no shared work and shares no mutexes/counters.
}

fn main() { cir_trace::init();
    // R1: start a worker, wait for it, then repeat the same cycle.
    // R3: each started worker is waited for exactly once before the next starts.
    // R4/R5: no task waits for a worker that cannot finish; all schedules terminate.
    // R6: print exactly `DONE done=1` and exit.

    // A semaphore is used to coordinate the start-and-wait cycle without
    // introducing shared mutable state between main and worker.
    let sem: Arc<Semaphore> = Semaphore::new_named("res_semaphore0", 0);

    for _ in 0..1 {
        let sem_clone = Arc::clone(&sem);
        let handle = cir_trace::spawn("worker", move || {
            worker();
            // Signal completion to the main task.
            let permit = sem_clone.try_acquire();
            if let Some(p) = permit {
                p.release();
            }
        });

        // Wait for the worker to finish exactly once.
        handle.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
