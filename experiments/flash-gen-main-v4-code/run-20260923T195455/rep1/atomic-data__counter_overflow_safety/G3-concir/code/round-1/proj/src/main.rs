mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    m: Mutex<i32>,
}

fn w1(shared: Arc<Shared>) {
    let mut tmp;
    {
        let mut guard = shared.m.lock().unwrap();
        tmp = *guard;
        if tmp < 1 {
            *guard = tmp + 1;
        }
    }
}

fn w2(shared: Arc<Shared>) {
    let mut tmp;
    {
        let mut guard = shared.m.lock().unwrap();
        tmp = *guard;
        if tmp < 1 {
            *guard = tmp + 1;
        }
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared { m: Mutex::new_named("shared_mutex0", 0) });

    let s1 = Arc::clone(&shared);
    let h1 = cir_trace::spawn("w1", move || w1(s1));

    let s2 = Arc::clone(&shared);
    let h2 = cir_trace::spawn("w2", move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *shared.m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
