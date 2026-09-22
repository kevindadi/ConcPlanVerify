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

    let shared_w1 = Arc::clone(&shared);
    let sup = cir_trace::spawn("sup", move || {
        w1(shared_w1);
    });

    sup.join().unwrap();

    let done = shared.lock().unwrap().done;
    println!("DONE done={}", if done { 1 } else { 0 });
 cir_trace::finish();}

fn w1(shared: Arc<Mutex<Shared>>) {
    let shared_a = Arc::clone(&shared);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        w2(shared_a);
    });

    let shared_b = Arc::clone(&shared);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        w2(shared_b);
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    let mut guard = shared.lock().unwrap();
    guard.done = true;
}

fn w2(shared: Arc<Mutex<Shared>>) {
    let mut guard = shared.lock().unwrap();
    let cur = guard.c;
    if cur < 1 {
        guard.c = cur + 1;
    }
}
