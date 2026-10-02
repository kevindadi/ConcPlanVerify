use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    // mutex_lock main::a
    let guard_a = a.lock().unwrap();
    // mutex_lock main::b
    let guard_b = b.lock().unwrap();

    // assign_local work = 1
    let work: i32 = 1;
    let _ = work;

    // mutex_unlock main::b
    drop(guard_b);
    // mutex_unlock main::a
    drop(guard_a);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    // mutex_lock main::a
    let guard_a = a.lock().unwrap();
    // mutex_lock main::b
    let guard_b = b.lock().unwrap();

    // assign_local work = 1
    let work: i32 = 1;
    let _ = work;

    // mutex_unlock main::b
    drop(guard_b);
    // mutex_unlock main::a
    drop(guard_a);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a_for_t1 = Arc::clone(&a);
    let b_for_t1 = Arc::clone(&b);
    let handle_t1 = thread::spawn(move || {
        t1(a_for_t1, b_for_t1);
    });

    let a_for_t2 = Arc::clone(&a);
    let b_for_t2 = Arc::clone(&b);
    let handle_t2 = thread::spawn(move || {
        t2(a_for_t2, b_for_t2);
    });

    handle_t1.join().unwrap();
    handle_t2.join().unwrap();

    println!("DONE t1=1 t2=1");
}
