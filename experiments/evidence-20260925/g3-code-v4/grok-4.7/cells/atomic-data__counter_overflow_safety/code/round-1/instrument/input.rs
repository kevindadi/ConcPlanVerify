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

    let done = m.lock().unwrap().c;
    println!("DONE done={done}");
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let mut cur: i32 = 0;
    cur = guard.c;
    if cur + 1 <= 1 {
        guard.c = cur + 1;
    }
    drop(guard);
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let mut cur: i32 = 0;
    cur = guard.c;
    if cur + 1 <= 1 {
        guard.c = cur + 1;
    }
    drop(guard);
}
