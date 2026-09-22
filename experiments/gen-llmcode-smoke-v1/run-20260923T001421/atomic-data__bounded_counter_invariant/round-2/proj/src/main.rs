mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let c = Arc::new(Mutex::new_named("c_mutex0", 0i32));
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    let c_w1 = Arc::clone(&c);
    let m_w1 = Arc::clone(&m);
    let sup = cir_trace::spawn("sup_0", move || {
        let c_w2a = Arc::clone(&c_w1);
        let m_w2a = Arc::clone(&m_w1);
        let w1 = cir_trace::spawnawn("w1", "sup_1", move || {
            let _guard = m_w2a.lock().unwrap();
            let tmp = *c_w2a.lock().unwrap();
            let tmp2 = tmp + 1;
            *c_w2a.lock().unwrap() = tmp2;
        });

        let c_w2b = Arc::clone(&c_w1);
        let m_w2b = Arc::clone(&m_w1);
        let w2 = cir_trace::spawnawn("w2", "sup_2", move || {
            let _guard = m_w2b.lock().unwrap();
            let tmp = *c_w2b.lock().unwrap();
            let tmp2 = tmp + 1;
            *c_w2b.lock().unwrap() = tmp2;
        });

        w1.join().unwrap();
        w2.join().unwrap();
    });

    sup.join().unwrap();

    let done = *c.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
