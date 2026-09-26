mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let _sem = Semaphore::new_named("_sem_semaphore0", 1);
    for _ in 0..2 {
        let handle = cir_trace::spawn("handle", worker);
        handle.join().unwrap();
    }
    let done = 1;
    println!("DONE done={done}");
 cir_trace::finish();}
