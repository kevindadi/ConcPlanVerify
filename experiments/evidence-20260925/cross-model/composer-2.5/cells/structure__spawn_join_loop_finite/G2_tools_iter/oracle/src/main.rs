mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let sem = Semaphore::new_named("sem_semaphore0", 1);
    for _ in 0..2 {
        let _permit = sem.acquire();
        let handle = cir_trace::spawn("handle", worker);
        handle.join().unwrap();
    }
    println!("DONE done=1");
 cir_trace::finish();}
