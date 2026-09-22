mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0i32));

    let w1 = {
        let m = Arc::clone(&m);
        move || {
            let mut c = m.lock().unwrap();
            if *c < 1 {
                *c = *c + 1;
            }
            drop(c);
        }
    };

    let w2 = {
        let m = Arc::clone(&m);
        move || {
            let mut c = m.lock().unwrap();
            if *c < 1 {
                *c = *c + 1;
            }
            drop(c);
        }
    };

    let h1 = cir_trace::spawn("h1", w1);
    let h2 = cir_trace::spawn("h2", w2);

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
