mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap(); // mutex_lock main::m
    if *c < 1 {
        *c += 1; // write_shared c = c + 1
    }
    // mutex_unlock main::m (guard dropped)
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut c = m.lock().unwrap(); // mutex_lock main::m
    if *c < 1 {
        *c += 1; // write_shared c = c + 1
    }
    // mutex_unlock main::m (guard dropped)
}

fn main() { cir_trace::init();
    // Shared resources: mutex `m` protecting variable `c` (init 0).
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));

    let h1 = cir_trace::spawn("h1", {
        let m = Arc::clone(&m);
        move || w1(m)
    });
    let h2 = cir_trace::spawn("h2", {
        let m = Arc::clone(&m);
        move || w2(m)
    });

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
