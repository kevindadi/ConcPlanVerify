use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(0i32)); // Shared counter c protected by mutex m

    let m1 = Arc::clone(&m);
    let w1_handle = thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        let val = *guard;
        if val < 1 {
            *guard = val + 1;
        }
        drop(guard);
    });

    let m2 = Arc::clone(&m);
    let w2_handle = thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        let val = *guard;
        if val < 1 {
            *guard = val + 1;
        }
        drop(guard);
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    let final_val = *m.lock().unwrap();
    println!("DONE done={}", final_val);
}
