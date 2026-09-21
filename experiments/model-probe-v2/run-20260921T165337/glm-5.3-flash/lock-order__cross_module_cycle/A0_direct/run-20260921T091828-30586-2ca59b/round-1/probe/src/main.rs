use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Module main owns resource a; module other owns resource b.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // Both tasks need both resources and must declare the cross-module
    // dependency. To guarantee every interleaving terminates, all tasks
    // acquire the shared resources in a single consistent global order
    // (a before b), which makes circular wait impossible.
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let t1 = thread::spawn(move || {
        let _ga = a1.lock().unwrap(); // resource owned by module main
        let _gb = b1.lock().unwrap(); // resource owned by module other
    });

    let (a2, b2) = (Arc::clone(&a), Arc::clone(&b));
    let t2 = thread::spawn(move || {
        let _ga = a2.lock().unwrap(); // resource owned by module main
        let _gb = b2.lock().unwrap(); // resource owned by module other
    });

    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
}
