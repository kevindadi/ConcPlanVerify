mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    let h_outer = cir_trace::spawn("clone", move || {
        let a_x1 = Arc::clone(&a_outer);
        let b_x1 = Arc::clone(&b_outer);
        let a_x2 = Arc::clone(&a_outer);
        let b_x2 = Arc::clone(&b_outer);

        let h_x1 = cir_trace::spawn("h_x1", move || {
            let _lock_a = a_x1.lock().unwrap();
            let _lock_b = b_x1.lock().unwrap();
            // Drop order: _lock_b then _lock_a (reverse of declaration)
            // This matches mutex_unlock b then mutex_unlock a
        });

        let h_x2 = cir_trace::spawn("h_x2", move || {
            let _lock_a = a_x2.lock().unwrap();
            let _lock_b = b_x2.lock().unwrap();
            // Drop order: _lock_b then _lock_a (reverse of declaration)
            // This matches mutex_unlock b then mutex_unlock a
        });

        h_x1.join().unwrap();
        h_x2.join().unwrap();
    });

    h_outer.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
