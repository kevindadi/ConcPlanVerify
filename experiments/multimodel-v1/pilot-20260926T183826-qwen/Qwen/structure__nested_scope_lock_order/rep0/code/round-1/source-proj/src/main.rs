use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let outer_handle = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || {
            let x1_handle = thread::spawn({
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                move || {
                    // x1: lock a, lock b, unlock b, unlock a
                    let _guard_a = a.lock().unwrap();
                    let _guard_b = b.lock().unwrap();
                    drop(_guard_b);
                    drop(_guard_a);
                }
            });

            let x2_handle = thread::spawn({
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                move || {
                    // x2: lock a, lock b, unlock b, unlock a
                    let _guard_a = a.lock().unwrap();
                    let _guard_b = b.lock().unwrap();
                    drop(_guard_b);
                    drop(_guard_a);
                }
            });

            x1_handle.join().unwrap();
            x2_handle.join().unwrap();
        }
    });

    outer_handle.join().unwrap();

    println!("DONE done=1");
}
