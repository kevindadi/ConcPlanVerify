mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0", 0));
    let c = Arc::new(Mutex::new_named("c_mutex0", 0));
    let d = Arc::new(Mutex::new_named("d_mutex0", 0));

    let mut handles = Vec::new();

    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            let _ga = a.lock().unwrap();
            let _gb = b.lock().unwrap();
        }));
    }
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        handles.push(thread::spawn(move || {
            let _ga = a.lock().unwrap();
            let _gb = b.lock().unwrap();
        }));
    }
    {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || {
            let _gc = c.lock().unwrap();
            let _gd = d.lock().unwrap();
        }));
    }
    {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        handles.push(thread::spawn(move || {
            let _gc = c.lock().unwrap();
            let _gd = d.lock().unwrap();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
