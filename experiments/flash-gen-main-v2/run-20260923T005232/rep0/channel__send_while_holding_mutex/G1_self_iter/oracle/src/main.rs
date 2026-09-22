mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc, mpsc};
use std::thread;

fn main() { cir_trace::init();
    let lock = Arc::new(Mutex::new_named("lock_mutex0", ()));
    let (tx1, rx1) = mpsc::channel::<i32>();
    let (tx2, rx2) = mpsc::channel::<i32>();

    let lock_s = Arc::clone(&lock);
    let s = cir_trace::spawn("s", move || {
        {
            let _g = lock_s.lock().unwrap();
        }
        tx1.send(1).unwrap();
        let v = rx2.recv().unwrap();
        {
            let _g = lock_s.lock().unwrap();
        }
        v
    });

    let lock_r = Arc::clone(&lock);
    let r = cir_trace::spawn("r", move || {
        {
            let _g = lock_r.lock().unwrap();
        }
        let v = rx1.recv().unwrap();
        tx2.send(v).unwrap();
        {
            let _g = lock_r.lock().unwrap();
        }
        v
    });

    let sv = s.join().unwrap();
    let rv = r.join().unwrap();
    println!("DONE done={}", sv + rv - 1);
 cir_trace::finish();}
