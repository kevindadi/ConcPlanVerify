use std::sync::{Arc, Mutex};
use std::thread;

// CIR: main::t1
// mutex_lock a; mutex_lock b; tmp = 1; mutex_unlock b; mutex_unlock a
fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let mut tmp: i32 = 0;

    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    tmp = 1;

    drop(guard_b);
    drop(guard_a);

    tmp
}

// CIR: main::t2
// mutex_lock a; mutex_lock b; tmp = 1; mutex_unlock b; mutex_unlock a
fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let mut tmp: i32 = 0;

    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    tmp = 1;

    drop(guard_b);
    drop(guard_a);

    tmp
}

fn main() {
    // Shared resources: a (lock), b (lock)
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    // spawn t1 with handle h1
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || t1(a1, b1));

    // spawn t2 with handle h2
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || t2(a2, b2));

    // join h1, join h2
    let r1 = h1.join().unwrap();
    let r2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", r1, r2);
}
