mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

type Shared = Arc<(Mutex<()>, Mutex<()>)>;

fn w1(shared: Shared) {
    let (a, b) = &*shared;
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    drop(b_guard);
    drop(a_guard);
}

fn w2(shared: Shared) {
    let (a, b) = &*shared;
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    drop(b_guard);
    drop(a_guard);
}

fn main() { cir_trace::init();
    let shared: Shared = Arc::new((Mutex::new_named("shared_mutex0#462", ()), Mutex::new_named("shared_mutex1#478", ())));

    let worker1 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w1#560", move || w1(shared))
    };
    let worker2 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("w2#676", move || w2(shared))
    };

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
