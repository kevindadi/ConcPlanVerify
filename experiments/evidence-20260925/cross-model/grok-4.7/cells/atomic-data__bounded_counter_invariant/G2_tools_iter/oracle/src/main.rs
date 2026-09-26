mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn w1(m: Arc<Mutex<i32>>, limit: Arc<Semaphore>) {
    let _permit = limit.acquire();
    let mut c = m.lock().unwrap();
    if *c < 2 {
        *c += 1;
    }
}

fn w2(m: Arc<Mutex<i32>>, limit: Arc<Semaphore>) {
    let _permit = limit.acquire();
    let mut c = m.lock().unwrap();
    if *c < 2 {
        *c += 1;
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let limit = Semaphore::new_named("limit_semaphore0", 2);

    let m1 = Arc::clone(&m);
    let limit1 = Arc::clone(&limit);
    let t1 = cir_trace::spawn("w1", move || w1(m1, limit1));

    let m2 = Arc::clone(&m);
    let limit2 = Arc::clone(&limit);
    let t2 = cir_trace::spawn("w2", move || w2(m2, limit2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
