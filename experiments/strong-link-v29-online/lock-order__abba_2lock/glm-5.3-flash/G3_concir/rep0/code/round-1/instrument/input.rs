use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // Lock a, then b (same order as t2, so no deadlock is possible).
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    // Critical work while holding both locks.
    // Unlock b, then a (guards drop in reverse acquisition order).
    drop(_guard_b);
    drop(_guard_a);
}

fn t2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    // Lock a, then b (same order as t1, so no deadlock is possible).
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    // Critical work while holding both locks.
    // Unlock b, then a (guards drop in reverse acquisition order).
    drop(_guard_b);
    drop(_guard_a);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle1 = thread::spawn(move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle2 = thread::spawn(move || t2(&a2, &b2));

    handle1.join().expect("t1 panicked");
    handle2.join().expect("t2 panicked");

    println!("DONE t1=1 t2=1");
}
