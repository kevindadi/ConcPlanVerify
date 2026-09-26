mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn helper(done: Arc<Semaphore>) {
    // Auxiliary routine: performs its work, then signals completion.
    let permit = done.acquire();
    permit.release();
}

fn main() { cir_trace::init();
    let done = Semaphore::new_named("done_semaphore0", 0);

    // First call sequence: main calls helper, waits for it to complete.
    let d1 = Arc::clone(&done);
    let h1 = cir_trace::spawn("helper", move || helper(d1));
    h1.join().unwrap();

    // Second call sequence: main calls helper again, waits for completion.
    let d2 = Arc::clone(&done);
    let h2 = cir_trace::spawn("helper", move || helper(d2));
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
