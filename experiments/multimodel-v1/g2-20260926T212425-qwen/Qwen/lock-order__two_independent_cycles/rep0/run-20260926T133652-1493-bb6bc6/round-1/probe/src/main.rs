use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // R2: Four locks shared among workers.
    // First pair: a, b (used by t1, t2)
    // Second pair: c, d (used by t3, t4)
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    // R6: Within the first pair, take locks in same relative sequence: a then b.
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1_handle = thread::spawn(move || {
        // R4: Hold both locks at the same time.
        let _lock_a = a1.lock().unwrap();
        let _lock_b = b1.lock().unwrap();
        // Work happens while holding both locks.
        // R9: Locks are released when guards go out of scope.
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2_handle = thread::spawn(move || {
        // R6: Same relative sequence as t1: a then b.
        let _lock_a = a2.lock().unwrap();
        let _lock_b = b2.lock().unwrap();
        // Work happens while holding both locks.
        // R9: Locks are released when guards go out of scope.
    });

    // R7: Within the second pair, take locks in same relative sequence: c then d.
    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let t3_handle = thread::spawn(move || {
        // R4: Hold both locks at the same time.
        let _lock_c = c3.lock().unwrap();
        let _lock_d = d3.lock().unwrap();
        // Work happens while holding both locks.
        // R9: Locks are released when guards go out of scope.
    });

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let t4_handle = thread::spawn(move || {
        // R7: Same relative sequence as t3: c then d.
        let _lock_c = c4.lock().unwrap();
        let _lock_d = d4.lock().unwrap();
        // Work happens while holding both locks.
        // R9: Locks are released when guards go out of scope.
    });

    // R10: Main thread waits for all four workers to finish.
    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();
    t4_handle.join().unwrap();

    // R12: Print exactly the line `DONE done=1` and exit.
    println!("DONE done=1");
}
