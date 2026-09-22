use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
    done: bool,
}

fn main() {
    let shared = Arc::new(Mutex::new(Shared { c: 0, done: false }));

    let shared_sup = Arc::clone(&shared);
    let sup = thread::spawn(move || {
        let shared_w1 = Arc::clone(&shared_sup);
        let w1 = thread::spawn(move || {
            let shared_w2a = Arc::clone(&shared_w1);
            let w2a = thread::spawn(move || {
                let mut guard = shared_w2a.lock().unwrap();
                let cur = guard.c;
                if cur < 1 {
                    guard.c = cur + 1;
                }
            });

            let shared_w2b = Arc::clone(&shared_w1);
            let w2b = thread::spawn(move || {
                let mut guard = shared_w2b.lock().unwrap();
                let cur = guard.c;
                if cur < 1 {
                    guard.c = cur + 1;
                }
            });

            w2a.join().unwrap();
            w2b.join().unwrap();

            let mut guard = shared_w1.lock().unwrap();
            guard.done = true;
        });

        w1.join().unwrap();
    });

    sup.join().unwrap();

    let guard = shared.lock().unwrap();
    println!("DONE done={}", if guard.done { 1 } else { 0 });
}
