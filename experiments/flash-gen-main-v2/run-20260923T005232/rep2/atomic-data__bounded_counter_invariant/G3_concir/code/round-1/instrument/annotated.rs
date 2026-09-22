mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", 0i32));

    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let w1 = cir_trace::spawn("w1", move || {
        {
            let _guard = m1.lock().unwrap();
            let tmp = {
                let cg = c1.lock().unwrap();
                *cg
            };
            let tmp2 = tmp + 1;
            {
                let mut cg = c1.lock().unwrap();
                *cg = tmp2;
            }
        }
    });

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let w2 = cir_trace::spawn("w2", move || {
        {
            let _guard = m2.lock().unwrap();
            let tmp = {
                let cg = c2.lock().unwrap();
                *cg
            };
            let tmp2 = tmp + 1;
            {
                let mut cg = c2.lock().unwrap();
                *cg = tmp2;
            }
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
