use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { c: 0 }));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let mut tmp: i32 = 0;
    tmp = guard.c;
    tmp = tmp + 1;
    guard.c = tmp;
    drop(guard);
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let mut tmp: i32 = 0;
    tmp = guard.c;
    tmp = tmp + 1;
    guard.c = tmp;
    drop(guard);
}
