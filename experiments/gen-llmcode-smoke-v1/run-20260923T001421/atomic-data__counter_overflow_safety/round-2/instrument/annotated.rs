mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    c: i32,
    done: bool,
}

fn main() { cir_trace::init();
    let shared = Arc::new(Mutex::new_named("shared_mutex0", Shared { c: 0, done: false }));

    let shared_sup = Arc::clone(&shared);
    let sup = cir_trace::spawn("sup_0", move || {
        let shared_w1 = Arc::clone(&shared_sup);
        let w1 = cir_trace::spawnawn("w1_0", "sup_1", move || {
            let shared_w2a = Arc::clone(&shared_w1);
            let w2a = cir_trace::spawnawnawn("w2a", "w1_1", "sup_2", move || {
                let mut guard = shared_w2a.lock().unwrap();
                let cur = guard.c;
                if cur < 1 {
                    guard.c = cur + 1;
                }
            });

            let shared_w2b = Arc::clone(&shared_w1);
            let w2b = cir_trace::spawnawnawn("w2b", "w1_2", "sup_3", move || {
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
 cir_trace::finish();}
