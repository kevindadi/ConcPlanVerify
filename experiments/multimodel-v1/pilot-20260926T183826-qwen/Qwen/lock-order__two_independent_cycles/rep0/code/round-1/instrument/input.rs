use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || {
        let _guard_a = a1.lock().unwrap();
        let _guard_b = b1.lock().unwrap();
        // Hold both locks briefly (simulated work)
        drop(_guard_b);
        drop(_guard_a);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = thread::spawn(move || {
        let _guard_a = a2.lock().unwrap();
        let _guard_b = b2.lock().unwrap();
        // Hold both locks briefly (simulated work)
        drop(_guard_b);
        drop(_guard_a);
    });

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let t3 = thread::spawn(move || {
        let _guard_c = c3.lock().unwrap();
        let _guard_d = d3.lock().unwrap();
        // Hold both locks briefly (simulated work)
        drop(_guard_d);
        drop(_guard_c);
    });

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let t4 = thread::spawn(move || {
        let _guard_c = c4.lock().unwrap();
        let _guard_d = d4.lock().unwrap();
        // Hold both locks briefly (simulated work)
        drop(_guard_d);
        drop(_guard_c);
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();

    println!("DONE done=1");
}
