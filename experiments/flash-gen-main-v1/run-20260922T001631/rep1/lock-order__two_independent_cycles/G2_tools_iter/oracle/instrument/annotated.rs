mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a1 = Arc::new(Mutex::new_named("a1_mutex0", ()));
    let a2 = Arc::new(Mutex::new_named("a2_mutex0", ()));
    let b1 = Arc::new(Mutex::new_named("b1_mutex0", ()));
    let b2 = Arc::new(Mutex::new_named("b2_mutex0", ()));

    let mut handles = Vec::new();

    {
        let l1 = Arc::clone(&a1);
        let l2 = Arc::clone(&a2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
        }));
    }
    {
        let l1 = Arc::clone(&a1);
        let l2 = Arc::clone(&a2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
        }));
    }
    {
        let l1 = Arc::clone(&b1);
        let l2 = Arc::clone(&b2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
        }));
    }
    {
        let l1 = Arc::clone(&b1);
        let l2 = Arc::clone(&b2);
        handles.push(thread::spawn(move || {
            let _g1 = l1.lock().unwrap();
            let _g2 = l2.lock().unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
