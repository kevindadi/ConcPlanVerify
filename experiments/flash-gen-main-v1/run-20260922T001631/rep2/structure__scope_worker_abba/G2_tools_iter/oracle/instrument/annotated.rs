mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let mut handles = Vec::new();

    for _ in 0..2 {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            // Take mutexes in a consistent order to avoid deadlock.
            let _ga = a.lock().unwrap();
            let _gb = b.lock().unwrap();
            // Both mutexes are held simultaneously here.
            // Work is finished; guards drop in reverse order, releasing both.
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
