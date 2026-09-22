mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0u32));
    let c = Arc::new(Mutex::new_named("c_mutex0", 0u32));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        handles.push(thread::spawn(move || {
            let _guard = m.lock().unwrap();
            let mut val = c.lock().unwrap();
            if *val < 1 {
                *val += 1;
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let done = *c.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
