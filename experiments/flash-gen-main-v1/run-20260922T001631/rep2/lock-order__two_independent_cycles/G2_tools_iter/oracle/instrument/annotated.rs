mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // Four locks: first pair (0,1), second pair (2,3)
    let locks: Vec<Arc<Mutex<()>>> = (0..4).map(|_| Arc::new(Mutex::new_named("res_mutex0", ()))).collect();

    let mut handles = Vec::new();

    // Workers 0 and 1 use locks 0 and 1, in the same relative order (0 then 1).
    for _ in 0..2 {
        let a = Arc::clone(&locks[0]);
        let b = Arc::clone(&locks[1]);
        handles.push(thread::spawn(move || {
            let _g1 = a.lock().unwrap();
            let _g2 = b.lock().unwrap();
            // hold both locks while working
        }));
    }

    // Workers 2 and 3 use locks 2 and 3, in the same relative order (2 then 3).
    for _ in 0..2 {
        let a = Arc::clone(&locks[2]);
        let b = Arc::clone(&locks[3]);
        handles.push(thread::spawn(move || {
            let _g1 = a.lock().unwrap();
            let _g2 = b.lock().unwrap();
            // hold both locks while working
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
