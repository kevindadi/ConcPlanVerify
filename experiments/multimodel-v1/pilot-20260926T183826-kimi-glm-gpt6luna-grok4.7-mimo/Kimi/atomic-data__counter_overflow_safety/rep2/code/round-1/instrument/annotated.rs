mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Shared resources: mutex `m` protecting variable `c` (init 0).
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));

    let w1 = {
        let m = Arc::clone(&m);
        move || {
            let mut c = m.lock().unwrap(); // mutex_lock main::m
            if *c < 1 {
                *c += 1; // write_shared c = c + 1
            }
            // mutex_unlock main::m (guard dropped)
        }
    };

    let w2 = {
        let m = Arc::clone(&m);
        move || {
            let mut c = m.lock().unwrap(); // mutex_lock main::m
            if *c < 1 {
                *c += 1; // write_shared c = c + 1
            }
            // mutex_unlock main::m (guard dropped)
        }
    };

    let h1 = cir_trace::spawn("h1", w1);
    let h2 = cir_trace::spawn("h2", w2);

    h1.join().unwrap();
    h2.join().unwrap();

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
