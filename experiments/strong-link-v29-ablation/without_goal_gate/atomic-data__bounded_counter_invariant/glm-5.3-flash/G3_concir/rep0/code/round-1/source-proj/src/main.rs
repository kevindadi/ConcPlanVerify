use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: &Arc<Mutex<Shared>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // read_shared c
    let c = guard.c;
    // write_shared c = c + 1
    guard.c = c + 1;
    // mutex_unlock m (guard drops at end of scope)
    drop(guard);
}

fn w2(m: &Arc<Mutex<Shared>>) {
    // mutex_lock m
    let mut guard = m.lock().unwrap();
    // read_shared c
    let c = guard.c;
    // write_shared c = c + 1
    guard.c = c + 1;
    // mutex_unlock m (guard drops at end of scope)
    drop(guard);
}

fn main() {
    // shared resource c (range 0..=2, starts at 0) protected by lock m
    let m = Arc::new(Mutex::new(Shared { c: 0 }));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || w1(&m1));

    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || w2(&m2));

    // supervising task waits for both workers to finish
    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");

    let done = { m.lock().unwrap().c / 2 };
    println!("DONE done={}", done);
}
