mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0", 0));

    let outer = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("outer_0", move || {
            let inner1 = {
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                cir_trace::spawnawn("inner1", "outer_1", move || {
                    let _ga = a.lock().unwrap();
                    let _gb = b.lock().unwrap();
                })
            };
            let inner2 = {
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                cir_trace::spawnawn("inner2", "outer_2", move || {
                    let _ga = a.lock().unwrap();
                    let _gb = b.lock().unwrap();
                })
            };
            inner1.join().unwrap();
            inner2.join().unwrap();
        })
    };

    outer.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
