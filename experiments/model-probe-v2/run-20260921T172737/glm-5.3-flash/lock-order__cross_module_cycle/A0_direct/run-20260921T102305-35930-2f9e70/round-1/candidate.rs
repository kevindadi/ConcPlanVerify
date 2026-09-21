use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Resource a is owned by module main, resource b by module other.
    // Both tasks declare the cross-module dependency by acquiring both
    // resources. To avoid deadlock, all tasks acquire the resources in a
    // single global lock order (a before b), so every interleaving
    // terminates and both tasks complete.
    let a = Arc::new(Mutex::new(())); // resource a (module main)
    let b = Arc::new(Mutex::new(())); // resource b (module other)

    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let t1 = thread::spawn(move || {
        let _ga = a1.lock().unwrap();
        let _gb = b1.lock().unwrap();
        // task 1 holds both resources: cross-module dependency declared
    });

    let (a2, b2) = (Arc::clone(&a), Arc::clone(&b));
    let t2 = thread::spawn(move || {
        let _ga = a2.lock().unwrap();
        let _gb = b2.lock().unwrap();
        // task 2 holds both resources: cross-module dependency declared
    });

    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
}
