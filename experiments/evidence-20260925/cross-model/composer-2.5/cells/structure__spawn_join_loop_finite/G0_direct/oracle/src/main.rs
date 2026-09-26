mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn worker() {
    // R2: no shared mutexes, counters, or work with the main task.
}

fn main() { cir_trace::init();
    let sem: Arc<Semaphore> = Semaphore::new_named("res_semaphore0", 1);

    let mut done = 0;
    while done < 1 {
        let _permit = sem.acquire();
        let handle = cir_trace::spawn("handle", worker);
        handle.join().unwrap();
        done += 1;
    }

    println!("DONE done={}", done);
 cir_trace::finish();}
