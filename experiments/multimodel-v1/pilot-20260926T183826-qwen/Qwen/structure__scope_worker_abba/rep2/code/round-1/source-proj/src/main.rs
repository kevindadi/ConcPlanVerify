use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let w1 = thread::spawn(move || {
        // w1: mutex_lock main::a; mutex_lock main::b; mutex_unlock main::b; mutex_unlock main::a
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Drop guards in reverse order of acquisition to match unlock order (b then a)
        drop(_guard_b);
        drop(_guard_a);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let w2 = thread::spawn(move || {
        // w2: mutex_lock main::a; mutex_lock main::b; mutex_unlock main::b; mutex_unlock main::a
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();
        // Drop guards in reverse order of acquisition to match unlock order (b then a)
        drop(_guard_b);
        drop(_guard_a);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
