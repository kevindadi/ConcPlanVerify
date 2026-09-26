use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { c: 0 }));

    let m1 = Arc::clone(&m);
    let w1 = thread::spawn(move || {
        {
            let mut guard = m1.lock().unwrap();
            guard.c = guard.c + 1;
        }
    });

    let m2 = Arc::clone(&m);
    let w2 = thread::spawn(move || {
        {
            let mut guard = m2.lock().unwrap();
            guard.c = guard.c + 1;
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = {
        let guard = m.lock().unwrap();
        guard.c
    };

    println!("DONE done={}", done);
}
