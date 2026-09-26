mod cir_trace;
use std::thread;
use concir_sync::Semaphore;

fn helper() {
    let sem = Semaphore::new_named("sem_semaphore0", 1);
    let permit = sem.acquire();
    permit.release();
}

fn main() { cir_trace::init();
    let handle = cir_trace::spawn("handle", helper);
    handle.join().unwrap();

    let handle = cir_trace::spawn("handle", helper);
    handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
