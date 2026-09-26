use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard_kept = m.lock().unwrap();
    let tmp = guard_kept.c;
    let tmp2 = tmp + 1;
    guard_kept.c = tmp2;
    drop(guard_kept);
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard_kept = m.lock().unwrap();
    let tmp = guard_kept.c;
    let tmp2 = tmp + 1;
    guard_kept.c = tmp2;
    drop(guard_kept);
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { c: 0 }));

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);

    let h1 = thread::spawn(move || w1(m1));
    let h2 = thread::spawn(move || w2(m2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = {
        let guard_kept = m.lock().unwrap();
        guard_kept.c
    };

    println!("DONE done={}", done);
}
