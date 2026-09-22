mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    let outer = cir_trace::spawn("outer_0", move || {
        let a1 = Arc::clone(&a_outer);
        let b1 = Arc::clone(&b_outer);
        let x1 = cir_trace::spawnawn("x1", "outer_1", move || {
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
        });

        let a2 = Arc::clone(&a_outer);
        let b2 = Arc::clone(&b_outer);
        let x2 = cir_trace::spawnawn("x2", "outer_2", move || {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
        });

        x1.join().unwrap();
        x2.join().unwrap();
    });

    outer.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
