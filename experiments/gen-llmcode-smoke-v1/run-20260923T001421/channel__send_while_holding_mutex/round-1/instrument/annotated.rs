mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn main() { cir_trace::init();
    let lock = Arc::new(Mutex::new_named("lock_mutex0", ()));
    let done = Arc::new(Mutex::new_named("done_mutex0", 0i32));
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let lock_s = Arc::clone(&lock);
    let s = cir_trace::spawn("s", move || {
        {
            let _g = lock_s.lock().unwrap();
        }
        tx.send(1).unwrap();
        {
            let _g = lock_s.lock().unwrap();
        }
    });

    let lock_r = Arc::clone(&lock);
    let done_r = Arc::clone(&done);
    let r = cir_trace::spawn("r", move || {
        {
            let _g = lock_r.lock().unwrap();
        }
        let _v: i32 = rx.recv().unwrap();
        {
            let _g = lock_r.lock().unwrap();
        }
        {
            let mut d = done_r.lock().unwrap();
            *d = 1;
        }
    });

    s.join().unwrap();
    r.join().unwrap();

    let d = *done.lock().unwrap();
    println!("DONE done={}", d);
 cir_trace::finish();}
