use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Shared resources: a and b (R2: shared by both workers, exclusive access)
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = thread::spawn(move || {
        worker("t1", a1, b1);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_handle = thread::spawn(move || {
        worker("t2", a2, b2);
    });

    // R6: main starts both workers and finishes only after both finish.
    t1_handle.join().expect("t1 panicked");
    t2_handle.join().expect("t2 panicked");

    // R9: exactly this line, then exit.
    println!("DONE t1=1 t2=1");
}

fn worker(name: &str, a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    // R3: hold both locks at the same time during critical work.
    // R5: both workers acquire in the same order (a, then b), so a cycle
    //     (each holding one lock and waiting for the other) can never form.
    let _ga = a.lock().expect("lock a poisoned"); // R4: blocks until free
    let _gb = b.lock().expect("lock b poisoned"); // R4: blocks until free

    // Critical work while holding both locks.
    println!("{}: critical work", name);

    // R7: guards drop here, releasing both locks before the worker finishes.
    drop(_gb);
    drop(_ga);
}
