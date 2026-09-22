use std::sync::{Arc, Mutex};
use std::thread;

fn worker(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>) {
    // Both workers acquire the mutexes in the same order (A then B),
    // so no wait cycle can form.
    let mut guard_a = a.lock().unwrap();
    let mut guard_b = b.lock().unwrap();

    // Both mutexes are held at the same time while work is performed.
    *guard_a += 1;
    *guard_b += 1;

    // Release each mutex once the work is finished.
    drop(guard_b);
    drop(guard_a);
}

fn main() {
    let a = Arc::new(Mutex::new(0i32));
    let b = Arc::new(Mutex::new(0i32));

    let mut handles = Vec::new();
    for _ in 0..2 {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || worker(a, b)));
    }

    // The group finishes only after both workers have completed.
    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
}
