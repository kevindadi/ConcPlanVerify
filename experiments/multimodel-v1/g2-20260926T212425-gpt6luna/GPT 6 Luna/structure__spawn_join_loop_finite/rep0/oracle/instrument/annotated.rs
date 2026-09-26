mod cir_trace;
use concir_sync::Semaphore;

fn worker() {}

fn main() { cir_trace::init();
    let semaphore = Semaphore::new_named("semaphore_semaphore0", 1);

    for _ in 0..2 {
        let permit = semaphore.acquire();
        let handle = cir_trace::spawn("handle", worker);
        handle.join().unwrap();
        permit.release();
    }

    println!("DONE done=1");
 cir_trace::finish();}
